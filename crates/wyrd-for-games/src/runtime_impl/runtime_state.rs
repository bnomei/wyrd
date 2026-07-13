//! Opaque, in-memory continuation snapshots for [`Runtime`](super::Runtime).
//!
//! This deliberately is not a codec. Hosts can wrap the opaque state in their
//! own save representation once they decide on a stable storage format.

use crate::foundation::Signal;
use crate::foundation::{KnotId, PortSlot, Seed};
use crate::runtime_impl::bind::{ResolvedKnot, Runtime};
use crate::runtime_impl::error::RestoreError;
use std::string::String;
use std::vec::Vec;

/// Current in-memory snapshot format version.
pub const RUNTIME_STATE_FORMAT_VERSION: u32 = 1;

/// Opaque cloneable continuation state produced by [`Runtime::snapshot`].
///
/// The value is intentionally not a portable byte format. Its fingerprint
/// ties it to the immutable executable graph and bind policy, while leaving
/// runtime-local dense handle owners out of the persisted representation.
#[derive(Clone, Debug)]
pub struct RuntimeState {
    version: u32,
    fingerprint: u64,
    data: RuntimeStateData,
}

#[derive(Clone, Debug)]
struct RuntimeStateData {
    sense_values: Vec<Signal>,
    port_vals: Vec<Signal>,
    prev_in: Vec<Signal>,
    prev_dec: Vec<Signal>,
    counter: Vec<i32>,
    flag: Vec<bool>,
    timer_left: Vec<u16>,
    on_start_done: Vec<bool>,
    delay_buf: Vec<Signal>,
    delay_head: Vec<u16>,
    out_signals: Vec<SignalOutState>,
    out_emits: Vec<EmitState>,
    dropped_emits: usize,
    tick: u64,
    rng: u64,
}

#[derive(Clone, Copy, Debug)]
struct SignalOutState {
    path_index: u16,
    value: Signal,
}

#[derive(Clone, Copy, Debug)]
struct EmitState {
    command_index: u16,
    payload: Signal,
}

impl RuntimeState {
    /// Snapshot format version carried by this state.
    pub const fn format_version(&self) -> u32 {
        self.version
    }

    /// Deterministic fingerprint of the immutable runtime this state requires.
    pub const fn fingerprint(&self) -> u64 {
        self.fingerprint
    }
}

impl Runtime {
    /// Snapshot every mutable value needed for deterministic continuation.
    pub fn snapshot(&self) -> RuntimeState {
        let mut state = RuntimeState {
            version: RUNTIME_STATE_FORMAT_VERSION,
            fingerprint: self.runtime_fingerprint(),
            data: RuntimeStateData {
                sense_values: Vec::new(),
                port_vals: Vec::new(),
                prev_in: Vec::new(),
                prev_dec: Vec::new(),
                counter: Vec::new(),
                flag: Vec::new(),
                timer_left: Vec::new(),
                on_start_done: Vec::new(),
                delay_buf: Vec::new(),
                delay_head: Vec::new(),
                out_signals: Vec::new(),
                out_emits: Vec::new(),
                dropped_emits: 0,
                tick: 0,
                rng: 0,
            },
        };
        self.snapshot_into(&mut state);
        state
    }

    /// Refresh an existing snapshot while reusing its owned buffer capacity.
    ///
    /// Every field is overwritten, so `state` may come from an incompatible
    /// runtime. Reuse is best-effort: buffers allocate when their retained
    /// capacity is too small and may retain a previous high-water capacity.
    pub fn snapshot_into(&self, state: &mut RuntimeState) {
        state.version = RUNTIME_STATE_FORMAT_VERSION;
        state.fingerprint = self.runtime_fingerprint();
        state.data.sense_values.clone_from(&self.sense_values);
        state.data.port_vals.clone_from(&self.port_vals);
        state.data.prev_in.clone_from(&self.prev_in);
        state.data.prev_dec.clone_from(&self.prev_dec);
        state.data.counter.clone_from(&self.counter);
        state.data.flag.clone_from(&self.flag);
        state.data.timer_left.clone_from(&self.timer_left);
        state.data.on_start_done.clone_from(&self.on_start_done);
        state.data.delay_buf.clone_from(&self.delay_buf);
        state.data.delay_head.clone_from(&self.delay_head);
        state.data.out_signals.clear();
        state
            .data
            .out_signals
            .extend(self.out_signals.iter().map(|sample| SignalOutState {
                path_index: sample.path.index,
                value: sample.value,
            }));
        state.data.out_emits.clear();
        state
            .data
            .out_emits
            .extend(self.out_emits.iter().map(|emit| EmitState {
                command_index: emit.cmd.index,
                payload: emit.payload,
            }));
        state.data.dropped_emits = self.dropped_emits;
        state.data.tick = self.tick;
        state.data.rng = self.rng;
    }

    /// Restore a compatible snapshot without changing runtime-local handles.
    ///
    /// Every compatibility and shape check runs before any mutable runtime
    /// field is assigned, so a rejected restore leaves the runtime unchanged.
    pub fn restore(&mut self, state: &RuntimeState) -> Result<(), RestoreError> {
        if state.version != RUNTIME_STATE_FORMAT_VERSION {
            return Err(RestoreError::UnsupportedVersion {
                found: state.version,
                supported: RUNTIME_STATE_FORMAT_VERSION,
            });
        }
        let expected = self.runtime_fingerprint();
        if state.fingerprint != expected {
            return Err(RestoreError::FingerprintMismatch {
                expected,
                found: state.fingerprint,
            });
        }

        self.validate_snapshot_shapes(&state.data)?;

        self.sense_values.clone_from(&state.data.sense_values);
        self.port_vals.clone_from(&state.data.port_vals);
        self.prev_in.clone_from(&state.data.prev_in);
        self.prev_dec.clone_from(&state.data.prev_dec);
        self.counter.clone_from(&state.data.counter);
        self.flag.clone_from(&state.data.flag);
        self.timer_left.clone_from(&state.data.timer_left);
        self.on_start_done.clone_from(&state.data.on_start_done);
        self.delay_buf.clone_from(&state.data.delay_buf);
        self.delay_head.clone_from(&state.data.delay_head);
        self.out_signals.clear();
        self.out_signals
            .extend(state.data.out_signals.iter().map(|sample| {
                crate::runtime_impl::outbox::SignalOutSample {
                    path: crate::runtime_impl::handles::HostPathId::new(
                        self.owner,
                        sample.path_index,
                    ),
                    value: sample.value,
                }
            }));
        self.out_emits.clear();
        self.out_emits
            .extend(
                state
                    .data
                    .out_emits
                    .iter()
                    .map(|emit| crate::runtime_impl::outbox::Emit {
                        cmd: crate::runtime_impl::handles::CmdId::new(
                            self.owner,
                            emit.command_index,
                        ),
                        payload: emit.payload,
                    }),
            );
        self.dropped_emits = state.data.dropped_emits;
        self.tick = state.data.tick;
        self.rng = state.data.rng;
        Ok(())
    }

    /// Format version emitted by [`Self::snapshot`].
    pub const fn state_format_version() -> u32 {
        RUNTIME_STATE_FORMAT_VERSION
    }

    /// Deterministic immutable compatibility fingerprint for this runtime.
    pub const fn runtime_fingerprint(&self) -> u64 {
        self.state_fingerprint
    }

    fn validate_snapshot_shapes(&self, data: &RuntimeStateData) -> Result<(), RestoreError> {
        check_len(
            "sense_values",
            self.sense_values.len(),
            data.sense_values.len(),
        )?;
        check_len("port_vals", self.port_vals.len(), data.port_vals.len())?;
        check_len("prev_in", self.prev_in.len(), data.prev_in.len())?;
        check_len("prev_dec", self.prev_dec.len(), data.prev_dec.len())?;
        check_len("counter", self.counter.len(), data.counter.len())?;
        check_len("flag", self.flag.len(), data.flag.len())?;
        check_len("timer_left", self.timer_left.len(), data.timer_left.len())?;
        check_len(
            "on_start_done",
            self.on_start_done.len(),
            data.on_start_done.len(),
        )?;
        check_len("delay_buf", self.delay_buf.len(), data.delay_buf.len())?;
        check_len("delay_head", self.delay_head.len(), data.delay_head.len())?;
        if data.out_signals.len() > self.out_signals.capacity() {
            return Err(RestoreError::ShapeMismatch {
                field: "out_signals",
                expected: self.out_signals.capacity(),
                found: data.out_signals.len(),
            });
        }
        if data.out_emits.len() > self.out_emits.capacity() {
            return Err(RestoreError::ShapeMismatch {
                field: "out_emits",
                expected: self.out_emits.capacity(),
                found: data.out_emits.len(),
            });
        }
        for sample in &data.out_signals {
            if usize::from(sample.path_index) >= self.path_names.len() {
                return Err(RestoreError::InvalidHandleIndex {
                    field: "out_signals.path_index",
                    index: sample.path_index,
                    len: self.path_names.len(),
                });
            }
        }
        for emit in &data.out_emits {
            if usize::from(emit.command_index) >= self.cmd_names.len() {
                return Err(RestoreError::InvalidHandleIndex {
                    field: "out_emits.command_index",
                    index: emit.command_index,
                    len: self.cmd_names.len(),
                });
            }
        }
        Ok(())
    }
}

pub(crate) fn runtime_fingerprint_for(
    knots: &[ResolvedKnot],
    threads: &[(KnotId, PortSlot, KnotId, PortSlot)],
    path_names: &[String],
    cmd_names: &[String],
    max_emits_per_tick: u16,
    seed_mix: u64,
    bind_seed: Option<Seed>,
) -> u64 {
    let mut hash = Fnv1a::new();
    hash.bytes(b"wyrd-runtime-state-v1");
    hash.usize(knots.len());
    for knot in knots {
        fingerprint_knot(&mut hash, &knot.kind);
    }
    hash.usize(threads.len());
    for &(from, from_slot, to, to_slot) in threads {
        hash.u16(from.get());
        hash.u8(from_slot.get());
        hash.u16(to.get());
        hash.u8(to_slot.get());
    }
    hash.usize(path_names.len());
    for path in path_names {
        hash.bytes(path.as_bytes());
        hash.u8(0xff);
    }
    hash.usize(cmd_names.len());
    for command in cmd_names {
        hash.bytes(command.as_bytes());
        hash.u8(0xff);
    }
    #[cfg(feature = "signal-f32")]
    hash.u8(1);
    #[cfg(feature = "signal-i32")]
    hash.u8(2);
    hash.u16(max_emits_per_tick);
    hash.u64(seed_mix);
    match bind_seed {
        Some(seed) => {
            hash.u8(1);
            hash.u64(seed.0);
        }
        None => hash.u8(0),
    }
    hash.finish()
}

fn check_len(field: &'static str, expected: usize, found: usize) -> Result<(), RestoreError> {
    if expected == found {
        Ok(())
    } else {
        Err(RestoreError::ShapeMismatch {
            field,
            expected,
            found,
        })
    }
}

struct Fnv1a(u64);

impl Fnv1a {
    const fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.u8(*byte);
        }
    }

    fn u8(&mut self, value: u8) {
        self.0 ^= u64::from(value);
        self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
    }

    fn u16(&mut self, value: u16) {
        self.bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn usize(&mut self, value: usize) {
        self.u64(value as u64);
    }

    const fn finish(&self) -> u64 {
        self.0
    }
}

fn fingerprint_knot(hash: &mut Fnv1a, kind: &crate::foundation::KnotKind) {
    use crate::foundation::KnotKind;
    match kind {
        KnotKind::Constant { domain, value } => {
            hash.u8(0);
            domain_hash(hash, *domain);
            signal_hash(hash, *value);
        }
        KnotKind::SignalIn { domain } => {
            hash.u8(1);
            domain_hash(hash, *domain);
        }
        KnotKind::OnStart => hash.u8(2),
        KnotKind::Not => hash.u8(3),
        KnotKind::And { arity } => {
            hash.u8(4);
            hash.u8(*arity);
        }
        KnotKind::Or { arity } => {
            hash.u8(5);
            hash.u8(*arity);
        }
        KnotKind::Compare {
            domain,
            op,
            rhs_const,
        } => {
            hash.u8(6);
            domain_hash(hash, *domain);
            hash.u8(compare_hash(*op));
            option_signal_hash(hash, *rhs_const);
        }
        KnotKind::RisingFromZero => hash.u8(7),
        KnotKind::Flag {
            priority,
            enable_toggle,
        } => {
            hash.u8(8);
            hash.u8(flag_priority_hash(*priority));
            hash.u8(u8::from(*enable_toggle));
        }
        KnotKind::Counter => hash.u8(9),
        KnotKind::Timer { mode, ticks } => {
            hash.u8(10);
            hash.u8(timer_mode_hash(*mode));
            hash.u16(*ticks);
        }
        KnotKind::Delay { ticks } => {
            hash.u8(11);
            hash.u16(*ticks);
        }
        KnotKind::Calc { domain, op } => {
            hash.u8(12);
            domain_hash(hash, *domain);
            hash.u8(calc_hash(*op));
        }
        KnotKind::Map {
            domain,
            in_min,
            in_max,
            out_min,
            out_max,
        } => {
            hash.u8(13);
            domain_hash(hash, *domain);
            signal_hash(hash, *in_min);
            signal_hash(hash, *in_max);
            signal_hash(hash, *out_min);
            signal_hash(hash, *out_max);
        }
        KnotKind::Abs { domain } => {
            hash.u8(14);
            domain_hash(hash, *domain);
        }
        KnotKind::Neg { domain } => {
            hash.u8(15);
            domain_hash(hash, *domain);
        }
        KnotKind::Select => hash.u8(16),
        KnotKind::Digitize {
            domain,
            steps,
            in_min,
            in_max,
            out_min,
            out_max,
        } => {
            hash.u8(17);
            domain_hash(hash, *domain);
            hash.u16(*steps);
            signal_hash(hash, *in_min);
            signal_hash(hash, *in_max);
            signal_hash(hash, *out_min);
            signal_hash(hash, *out_max);
        }
        KnotKind::Threshold {
            domain,
            high,
            low,
            use_hysteresis,
        } => {
            hash.u8(18);
            domain_hash(hash, *domain);
            signal_hash(hash, *high);
            signal_hash(hash, *low);
            hash.u8(u8::from(*use_hysteresis));
        }
        KnotKind::Random {
            domain,
            require_gate,
        } => {
            hash.u8(19);
            domain_hash(hash, *domain);
            hash.u8(u8::from(*require_gate));
        }
        KnotKind::Sqrt { domain } => {
            hash.u8(20);
            domain_hash(hash, *domain);
        }
        KnotKind::Xor => hash.u8(21),
        KnotKind::FallingToZero => hash.u8(22),
        KnotKind::Change => hash.u8(23),
        KnotKind::Clamp { domain, min, max } => {
            hash.u8(24);
            domain_hash(hash, *domain);
            signal_hash(hash, *min);
            signal_hash(hash, *max);
        }
        KnotKind::Convert { from, to } => {
            hash.u8(25);
            domain_hash(hash, *from);
            domain_hash(hash, *to);
        }
        KnotKind::SignalOut { path, domain } => {
            hash.u8(26);
            hash.bytes(path.as_bytes());
            hash.u8(0);
            domain_hash(hash, *domain);
        }
        KnotKind::EmitCommand { name } => {
            hash.u8(27);
            hash.bytes(name.as_bytes());
            hash.u8(0);
        }
    }
}

fn domain_hash(hash: &mut Fnv1a, domain: crate::foundation::SignalDomain) {
    hash.u8(match domain {
        crate::foundation::SignalDomain::Bool => 0,
        crate::foundation::SignalDomain::Level => 1,
        crate::foundation::SignalDomain::Count => 2,
    });
}
fn compare_hash(op: crate::foundation::CompareOp) -> u8 {
    match op {
        crate::foundation::CompareOp::Eq => 0,
        crate::foundation::CompareOp::Ne => 1,
        crate::foundation::CompareOp::Lt => 2,
        crate::foundation::CompareOp::Lte => 3,
        crate::foundation::CompareOp::Gt => 4,
        crate::foundation::CompareOp::Gte => 5,
    }
}
fn flag_priority_hash(priority: crate::foundation::FlagPriority) -> u8 {
    match priority {
        crate::foundation::FlagPriority::ResetWins => 0,
        crate::foundation::FlagPriority::SetWins => 1,
    }
}
fn timer_mode_hash(mode: crate::foundation::TimerMode) -> u8 {
    match mode {
        crate::foundation::TimerMode::FedCountdown => 0,
        crate::foundation::TimerMode::PulseHold => 1,
    }
}
fn calc_hash(op: crate::foundation::CalcOp) -> u8 {
    match op {
        crate::foundation::CalcOp::Add => 0,
        crate::foundation::CalcOp::Sub => 1,
        crate::foundation::CalcOp::Mul => 2,
        crate::foundation::CalcOp::Div => 3,
    }
}
fn option_signal_hash(hash: &mut Fnv1a, value: Option<Signal>) {
    match value {
        Some(value) => {
            hash.u8(1);
            signal_hash(hash, value);
        }
        None => hash.u8(0),
    }
}
fn signal_hash(hash: &mut Fnv1a, value: Signal) {
    #[cfg(feature = "signal-f32")]
    hash.u64(u64::from(value.to_bits()));
    #[cfg(feature = "signal-i32")]
    hash.bytes(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{HostTime, KnotKind, SignalDomain, ONE};
    use crate::{BindOpts, Weave};

    fn outbox_runtime() -> Runtime {
        let mut builder = Weave::builder("snapshot-shapes").unwrap();
        let source = builder
            .knot("source", KnotKind::constant(ONE, SignalDomain::Bool))
            .unwrap();
        let output = builder
            .knot(
                "output",
                KnotKind::signal_out("snapshot.value", SignalDomain::Bool),
            )
            .unwrap();
        let emit = builder
            .knot("emit", KnotKind::emit_command("snapshot.command"))
            .unwrap();
        for (target, port) in [(&output, "in"), (&emit, "trigger")] {
            let from = builder.output(&source, "out").unwrap();
            let to = builder.input(target, port).unwrap();
            builder.connect(from, to).unwrap();
        }
        let mut runtime = Runtime::bind(
            builder.build().unwrap(),
            BindOpts {
                max_emits_per_tick: 1,
                ..BindOpts::default()
            },
        )
        .unwrap();
        runtime.begin_frame(HostTime { tick: 0 });
        runtime.loom();
        runtime
    }

    #[test]
    fn restore_rejects_version_shapes_and_owner_free_indices() {
        let mut runtime = outbox_runtime();
        assert_eq!(
            Runtime::state_format_version(),
            RUNTIME_STATE_FORMAT_VERSION
        );

        let mut state = runtime.snapshot();
        state.version = 0;
        assert!(matches!(
            runtime.restore(&state),
            Err(RestoreError::UnsupportedVersion { .. })
        ));

        let mut state = runtime.snapshot();
        state.data.sense_values.pop();
        assert!(matches!(
            runtime.restore(&state),
            Err(RestoreError::ShapeMismatch {
                field: "sense_values",
                ..
            })
        ));

        let mut state = runtime.snapshot();
        state.data.on_start_done.pop();
        assert!(matches!(
            runtime.restore(&state),
            Err(RestoreError::ShapeMismatch {
                field: "on_start_done",
                ..
            })
        ));

        let mut state = runtime.snapshot();
        state.data.out_signals.push(state.data.out_signals[0]);
        assert!(matches!(
            runtime.restore(&state),
            Err(RestoreError::ShapeMismatch {
                field: "out_signals",
                ..
            })
        ));

        let mut state = runtime.snapshot();
        state.data.out_emits.push(state.data.out_emits[0]);
        assert!(matches!(
            runtime.restore(&state),
            Err(RestoreError::ShapeMismatch {
                field: "out_emits",
                ..
            })
        ));

        let mut state = runtime.snapshot();
        state.data.out_signals[0].path_index = u16::MAX;
        assert!(matches!(
            runtime.restore(&state),
            Err(RestoreError::InvalidHandleIndex {
                field: "out_signals.path_index",
                ..
            })
        ));

        let mut state = runtime.snapshot();
        state.data.out_emits[0].command_index = u16::MAX;
        assert!(matches!(
            runtime.restore(&state),
            Err(RestoreError::InvalidHandleIndex {
                field: "out_emits.command_index",
                ..
            })
        ));
    }

    #[test]
    fn restore_rebuilds_valid_signal_and_emit_handles() {
        let source = outbox_runtime();
        let state = source.snapshot();
        let mut destination = outbox_runtime();
        destination.restore(&state).unwrap();
        let signal = destination.outbox().signals()[0];
        let emit = destination.outbox().emits()[0];
        assert_eq!(destination.path_name(signal.path), Ok("snapshot.value"));
        assert_eq!(destination.cmd_name(emit.cmd), Ok("snapshot.command"));
    }
}
