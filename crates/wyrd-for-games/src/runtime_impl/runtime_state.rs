//! Opaque, durable continuation checkpoints for [`Runtime`](super::Runtime).
//!
//! Capture a snapshot after loom settles and the host has applied the outbox,
//! before the next [`Runtime::begin_frame`]. Checkpoints store mutable knot
//! state and host-fed senses only — outbox effects are never replayed by
//! [`Runtime::restore`].

use crate::foundation::Signal;
use crate::foundation::{KnotId, NumericPath, PortSlot, Seed};
use crate::runtime_impl::bind::{ResolvedKnot, Runtime};
use crate::runtime_impl::error::{BindRestoreError, RestoreError};
use std::boxed::Box;
use std::collections::BTreeSet;
use std::string::String;
use std::vec::Vec;

/// RON checkpoint codec error.
#[cfg(feature = "serde-ron")]
#[derive(Debug)]
pub enum RuntimeStateRonCodecError {
    /// RON parsing failed.
    Parse(ron::error::SpannedError),
    /// RON serialization failed.
    Serialize(ron::Error),
}

#[cfg(feature = "serde-ron")]
impl core::fmt::Display for RuntimeStateRonCodecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "runtime checkpoint RON parse error: {error}"),
            Self::Serialize(error) => {
                write!(f, "runtime checkpoint RON serialization error: {error}")
            }
        }
    }
}
#[cfg(feature = "serde-ron")]
impl std::error::Error for RuntimeStateRonCodecError {}

/// JSON checkpoint codec error.
#[cfg(feature = "serde-json")]
#[derive(Debug)]
pub enum RuntimeStateJsonCodecError {
    /// JSON parsing failed.
    Parse(serde_json::Error),
    /// JSON serialization failed.
    Serialize(serde_json::Error),
}
#[cfg(feature = "serde-json")]
impl core::fmt::Display for RuntimeStateJsonCodecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "runtime checkpoint JSON parse error: {error}"),
            Self::Serialize(error) => {
                write!(f, "runtime checkpoint JSON serialization error: {error}")
            }
        }
    }
}
#[cfg(feature = "serde-json")]
impl std::error::Error for RuntimeStateJsonCodecError {}

/// Serialize a checkpoint as pretty RON.
#[cfg(feature = "serde-ron")]
pub fn runtime_state_to_ron(state: &RuntimeState) -> Result<String, RuntimeStateRonCodecError> {
    ron::ser::to_string_pretty(state, ron::ser::PrettyConfig::default())
        .map_err(RuntimeStateRonCodecError::Serialize)
}
/// Deserialize a checkpoint from RON. Bind it with [`Runtime::bind_restored`] to validate topology and state.
#[cfg(feature = "serde-ron")]
pub fn runtime_state_from_ron(text: &str) -> Result<RuntimeState, RuntimeStateRonCodecError> {
    ron::from_str(text).map_err(RuntimeStateRonCodecError::Parse)
}
/// Serialize a checkpoint as JSON.
#[cfg(feature = "serde-json")]
pub fn runtime_state_to_json(state: &RuntimeState) -> Result<String, RuntimeStateJsonCodecError> {
    serde_json::to_string_pretty(state).map_err(RuntimeStateJsonCodecError::Serialize)
}
/// Deserialize a checkpoint from JSON. Bind it with [`Runtime::bind_restored`] to validate topology and state.
#[cfg(feature = "serde-json")]
pub fn runtime_state_from_json(text: &str) -> Result<RuntimeState, RuntimeStateJsonCodecError> {
    serde_json::from_str(text).map_err(RuntimeStateJsonCodecError::Parse)
}

/// Current in-memory snapshot format version.
pub const RUNTIME_STATE_FORMAT_VERSION: u32 = 2;

/// Opaque cloneable continuation state produced by [`Runtime::snapshot`].
///
/// The Rust layout is private, but its Serde representation is a versioned
/// save contract. It contains no outbox effects: host-applied signals and
/// commands are never replayed by a restore.
#[derive(Clone, Debug)]
pub struct RuntimeState {
    version: u32,
    numeric_path: NumericPath,
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
    tick: u64,
    phase: u8,
    rng: u64,
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

    /// Numeric representation selected when this checkpoint was captured.
    pub const fn numeric_path(&self) -> NumericPath {
        self.numeric_path
    }
}

/// Authorable initial values addressed by stable knot names.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RuntimePreset {
    entries: Vec<RuntimePresetEntry>,
}

/// One named semantic initial-state value in a [`RuntimePreset`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RuntimePresetEntry {
    /// Set a Flag latch before the first frame.
    Flag {
        /// Authored knot name.
        knot: String,
        /// Initial latch state.
        value: bool,
    },
    /// Set a Counter before the first frame.
    Counter {
        /// Authored knot name.
        knot: String,
        /// Initial whole-number count.
        value: i32,
    },
    /// Set a held SignalIn value before the first frame.
    Sense {
        /// Authored SignalIn knot name.
        knot: String,
        /// Domain-checked held value.
        value: Signal,
    },
}

impl RuntimePreset {
    /// Create an empty semantic preset.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    /// Add one semantic entry. Duplicate names are rejected by [`Runtime::bind_with_preset`].
    pub fn push(&mut self, entry: RuntimePresetEntry) {
        self.entries.push(entry);
    }
    /// Read the authored entries without exposing mutable runtime storage.
    pub fn entries(&self) -> &[RuntimePresetEntry] {
        &self.entries
    }
}

/// Owned, authored-name state entry for checkpoint auditing.
#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeStateEntry {
    /// A held SignalIn value.
    Sense {
        /// Authored SignalIn knot name.
        knot: String,
        /// Domain-checked held value.
        value: Signal,
    },
    /// A Flag latch.
    Flag {
        /// Authored Flag knot name.
        knot: String,
        /// Current latch state.
        value: bool,
    },
    /// A Counter or Random cached sample backing state.
    Counter {
        /// Authored Counter or Random knot name.
        knot: String,
        /// Whole-number backing value.
        value: i32,
    },
    /// Timer remaining loom ticks.
    Timer {
        /// Authored Timer knot name.
        knot: String,
        /// Remaining loom ticks before expiry.
        remaining: u16,
    },
    /// Delay-ring metadata and contents in logical output order.
    Delay {
        /// Authored Delay knot name.
        knot: String,
        /// Immutable ring length fixed at bind.
        len: u16,
        /// Current ring head index.
        head: u16,
        /// Ring contents in logical output order.
        values: Vec<Signal>,
    },
    /// Edge detector history.
    Edge {
        /// Authored edge-detector knot name.
        knot: String,
        /// Previous input sample used for rising/falling detection.
        previous: Signal,
    },
    /// OnStart completion state.
    OnStart {
        /// Authored OnStart knot name.
        knot: String,
        /// Whether the one-shot pulse has already fired.
        completed: bool,
    },
    /// Random cached sample; the stream position remains opaque.
    Random {
        /// Authored Random knot name.
        knot: String,
        /// Last emitted random sample.
        last_sample: Signal,
        /// Whether the xorshift stream position is intentionally omitted.
        stream_is_opaque: bool,
    },
}

/// Read-only semantic checkpoint report.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeStateReport {
    /// Checkpoint format version.
    pub version: u32,
    /// Numeric signal path.
    pub numeric_path: NumericPath,
    /// Immutable runtime fingerprint.
    pub fingerprint: u64,
    /// Last frame tick.
    pub tick: u64,
    /// Named state entries in lexical knot-name order.
    pub entries: Vec<RuntimeStateEntry>,
}

#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeStateWire {
    version: u32,
    numeric_path: NumericPath,
    fingerprint: u64,
    sense_values: Vec<u32>,
    port_vals: Vec<u32>,
    prev_in: Vec<u32>,
    prev_dec: Vec<u32>,
    counter: Vec<i32>,
    flag: Vec<bool>,
    timer_left: Vec<u16>,
    on_start_done: Vec<bool>,
    delay_buf: Vec<u32>,
    delay_head: Vec<u16>,
    tick: u64,
    phase: u8,
    rng: u64,
}

#[cfg(feature = "serde")]
fn signal_to_wire(value: Signal) -> u32 {
    #[cfg(feature = "signal-f32")]
    {
        value.to_bits()
    }
    #[cfg(feature = "signal-i32")]
    {
        value as u32
    }
}

#[cfg(feature = "serde")]
fn signal_from_wire(value: u32) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        f32::from_bits(value)
    }
    #[cfg(feature = "signal-i32")]
    {
        value as i32
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for RuntimeState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        RuntimeStateWire {
            version: self.version,
            numeric_path: self.numeric_path,
            fingerprint: self.fingerprint,
            sense_values: self
                .data
                .sense_values
                .iter()
                .copied()
                .map(signal_to_wire)
                .collect(),
            port_vals: self
                .data
                .port_vals
                .iter()
                .copied()
                .map(signal_to_wire)
                .collect(),
            prev_in: self
                .data
                .prev_in
                .iter()
                .copied()
                .map(signal_to_wire)
                .collect(),
            prev_dec: self
                .data
                .prev_dec
                .iter()
                .copied()
                .map(signal_to_wire)
                .collect(),
            counter: self.data.counter.clone(),
            flag: self.data.flag.clone(),
            timer_left: self.data.timer_left.clone(),
            on_start_done: self.data.on_start_done.clone(),
            delay_buf: self
                .data
                .delay_buf
                .iter()
                .copied()
                .map(signal_to_wire)
                .collect(),
            delay_head: self.data.delay_head.clone(),
            tick: self.data.tick,
            phase: self.data.phase,
            rng: self.data.rng,
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for RuntimeState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = RuntimeStateWire::deserialize(deserializer)?;
        Ok(Self {
            version: wire.version,
            numeric_path: wire.numeric_path,
            fingerprint: wire.fingerprint,
            data: RuntimeStateData {
                sense_values: wire
                    .sense_values
                    .into_iter()
                    .map(signal_from_wire)
                    .collect(),
                port_vals: wire.port_vals.into_iter().map(signal_from_wire).collect(),
                prev_in: wire.prev_in.into_iter().map(signal_from_wire).collect(),
                prev_dec: wire.prev_dec.into_iter().map(signal_from_wire).collect(),
                counter: wire.counter,
                flag: wire.flag,
                timer_left: wire.timer_left,
                on_start_done: wire.on_start_done,
                delay_buf: wire.delay_buf.into_iter().map(signal_from_wire).collect(),
                delay_head: wire.delay_head,
                tick: wire.tick,
                phase: wire.phase,
                rng: wire.rng,
            },
        })
    }
}

impl Runtime {
    /// Bind a runtime then apply every named preset entry atomically.
    pub fn bind_with_preset(
        weave: crate::authoring::Weave,
        opts: crate::runtime_impl::bind::BindOpts,
        preset: &RuntimePreset,
    ) -> Result<Self, crate::runtime_impl::error::PresetError> {
        use crate::runtime_impl::error::PresetError;
        let mut runtime =
            Self::bind(weave, opts).map_err(|error| PresetError::Bind(Box::new(error)))?;
        let mut names = BTreeSet::new();
        for entry in &preset.entries {
            let name = match entry {
                RuntimePresetEntry::Flag { knot, .. }
                | RuntimePresetEntry::Counter { knot, .. }
                | RuntimePresetEntry::Sense { knot, .. } => knot,
            };
            if !names.insert(name) {
                return Err(PresetError::Duplicate { knot: name.clone() });
            }
            let Some(id) = runtime.name_to_id.get(name).copied() else {
                return Err(PresetError::Missing { knot: name.clone() });
            };
            match entry {
                RuntimePresetEntry::Flag { .. }
                    if matches!(
                        runtime.knots[usize::from(id)].kind,
                        crate::foundation::KnotKind::Flag { .. }
                    ) => {}
                RuntimePresetEntry::Counter { .. }
                    if matches!(
                        runtime.knots[usize::from(id)].kind,
                        crate::foundation::KnotKind::Counter
                    ) => {}
                RuntimePresetEntry::Sense { value, .. } => {
                    match runtime.knots[usize::from(id)].kind {
                        crate::foundation::KnotKind::SignalIn { domain }
                            if crate::runtime_impl::outbox::domain_value_is_valid(
                                domain, *value,
                            ) => {}
                        crate::foundation::KnotKind::SignalIn { domain } => {
                            return Err(PresetError::InvalidSignal {
                                knot: name.clone(),
                                domain,
                            })
                        }
                        _ => {
                            return Err(PresetError::WrongKind {
                                knot: name.clone(),
                                expected: "SignalIn",
                            })
                        }
                    }
                }
                RuntimePresetEntry::Flag { .. } => {
                    return Err(PresetError::WrongKind {
                        knot: name.clone(),
                        expected: "Flag",
                    })
                }
                RuntimePresetEntry::Counter { .. } => {
                    return Err(PresetError::WrongKind {
                        knot: name.clone(),
                        expected: "Counter",
                    })
                }
            }
        }
        for entry in &preset.entries {
            let name = match entry {
                RuntimePresetEntry::Flag { knot, .. }
                | RuntimePresetEntry::Counter { knot, .. }
                | RuntimePresetEntry::Sense { knot, .. } => knot,
            };
            let id = runtime.name_to_id[name];
            let index = usize::from(id);
            match entry {
                RuntimePresetEntry::Flag { value, .. } => runtime.flag[index] = *value,
                RuntimePresetEntry::Counter { value, .. } => runtime.counter[index] = *value,
                RuntimePresetEntry::Sense { value, .. } => runtime.sense_values[index] = *value,
            }
        }
        Ok(runtime)
    }

    /// Inspect the current runtime through stable authored names.
    pub fn inspect_state(&self) -> RuntimeStateReport {
        self.inspect_data(
            Self::state_format_version(),
            NumericPath::compiled(),
            self.runtime_fingerprint(),
            &RuntimeStateData {
                sense_values: self.sense_values.clone(),
                port_vals: self.port_vals.clone(),
                prev_in: self.prev_in.clone(),
                prev_dec: self.prev_dec.clone(),
                counter: self.counter.clone(),
                flag: self.flag.clone(),
                timer_left: self.timer_left.clone(),
                on_start_done: self.on_start_done.clone(),
                delay_buf: self.delay_buf.clone(),
                delay_head: self.delay_head.clone(),
                tick: self.tick,
                phase: self.phase,
                rng: self.rng,
            },
        )
    }

    /// Validate and inspect a checkpoint without mutating this runtime.
    pub fn inspect_checkpoint(
        &self,
        state: &RuntimeState,
    ) -> Result<RuntimeStateReport, RestoreError> {
        self.validate_checkpoint(state)?;
        Ok(self.inspect_data(
            state.version,
            state.numeric_path,
            state.fingerprint,
            &state.data,
        ))
    }
    /// Bind a completely fresh runtime and atomically restore a checkpoint.
    ///
    /// This is the recommended disk-load path. The returned runtime has an
    /// empty outbox: effects from the checkpoint frame were already applied by
    /// the host before it was saved.
    pub fn bind_restored(
        weave: crate::authoring::Weave,
        opts: crate::runtime_impl::bind::BindOpts,
        state: &RuntimeState,
    ) -> Result<Self, BindRestoreError> {
        let mut runtime =
            Self::bind(weave, opts).map_err(|error| BindRestoreError::Bind(Box::new(error)))?;
        runtime.restore(state).map_err(BindRestoreError::Restore)?;
        Ok(runtime)
    }

    /// Snapshot every mutable value needed for deterministic continuation.
    ///
    /// Capture only after `loom` and host apply, before the next
    /// [`Runtime::begin_frame`]. Outbox effects are deliberately excluded;
    /// restore resumes stateful memory with an empty outbox.
    ///
    /// # Examples
    ///
    /// ```
    /// use wyrd::{weave, BindOpts, HostTime, KnotKind, Runtime, SignalDomain, ONE, ZERO};
    ///
    /// let weave = weave! {
    ///     id: "snapshot.docs";
    ///     knots {
    ///         input = KnotKind::signal_in(SignalDomain::Bool);
    ///         output = KnotKind::signal_out("snapshot.output", SignalDomain::Bool);
    ///     }
    ///     threads { input.out -> output.in; }
    /// }?;
    /// let mut runtime = Runtime::bind(weave.clone(), BindOpts::default())?;
    /// let input = runtime.required_sense("input")?;
    ///
    /// runtime.begin_frame(HostTime { tick: 1 });
    /// runtime.port_writer().set_sense(input, ONE)?;
    /// runtime.loom();
    /// let state = runtime.snapshot();
    ///
    /// runtime.begin_frame(HostTime { tick: 2 });
    /// runtime.port_writer().set_sense(input, ZERO)?;
    /// runtime.loom();
    /// assert_eq!(runtime.outbox().signals()[0].value, ZERO);
    ///
    /// let restored = Runtime::bind_restored(weave, BindOpts::default(), &state)?;
    /// assert!(restored.outbox().signals().is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn snapshot(&self) -> RuntimeState {
        let mut state = RuntimeState {
            version: RUNTIME_STATE_FORMAT_VERSION,
            numeric_path: NumericPath::compiled(),
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
                tick: 0,
                phase: 0,
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
        state.numeric_path = NumericPath::compiled();
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
        state.data.tick = self.tick;
        state.data.phase = self.phase;
        state.data.rng = self.rng;
    }

    /// Restore a compatible snapshot without changing runtime-local handles.
    ///
    /// Every compatibility and shape check runs before any mutable runtime
    /// field is assigned, so a rejected restore leaves the runtime unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`RestoreError`] when the format version, executable
    /// fingerprint, buffer shape, or saved outbox handles are incompatible.
    pub fn restore(&mut self, state: &RuntimeState) -> Result<(), RestoreError> {
        self.validate_checkpoint(state)?;
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
        // Effects are deliberately ephemeral. A disk load starts with an
        // empty outbox so a host never applies a prior frame twice.
        self.out_signals.clear();
        self.out_emits.clear();
        self.dropped_emits = 0;
        self.tick = state.data.tick;
        self.phase = state.data.phase;
        self.rng = state.data.rng;
        Ok(())
    }

    fn validate_checkpoint(&self, state: &RuntimeState) -> Result<(), RestoreError> {
        if state.version != RUNTIME_STATE_FORMAT_VERSION {
            return Err(RestoreError::UnsupportedVersion {
                found: state.version,
                supported: RUNTIME_STATE_FORMAT_VERSION,
            });
        }
        if state.numeric_path != NumericPath::compiled() {
            return Err(RestoreError::NumericPathMismatch {
                expected: NumericPath::compiled(),
                found: state.numeric_path,
            });
        }
        if state.data.phase != 2 {
            return Err(RestoreError::InvalidPhase {
                found: state.data.phase,
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

        for (field, values) in [
            ("port_vals", &state.data.port_vals),
            ("prev_in", &state.data.prev_in),
            ("prev_dec", &state.data.prev_dec),
            ("delay_buf", &state.data.delay_buf),
        ] {
            for (knot, value) in values.iter().enumerate() {
                #[cfg(feature = "signal-f32")]
                if !value.is_finite() {
                    return Err(RestoreError::InvalidSignal {
                        field,
                        knot,
                        domain: crate::foundation::SignalDomain::Level,
                    });
                }
                #[cfg(feature = "signal-i32")]
                let _ = (field, knot, value);
            }
        }

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
        if data.rng == 0 {
            return Err(RestoreError::InvalidRng);
        }
        for (knot, resolved) in self.knots.iter().enumerate() {
            if let crate::foundation::KnotKind::Timer { ticks, .. } = resolved.kind {
                let remaining = data.timer_left[knot];
                if remaining > ticks {
                    return Err(RestoreError::InvalidTimer {
                        knot,
                        remaining,
                        max: ticks,
                    });
                }
            }
            if let crate::foundation::KnotKind::Delay { ticks } = resolved.kind {
                let head = data.delay_head[knot];
                if ticks == 0 {
                    if head != 0 {
                        return Err(RestoreError::InvalidDelayHead {
                            knot,
                            head,
                            len: ticks,
                        });
                    }
                } else if head >= ticks {
                    return Err(RestoreError::InvalidDelayHead {
                        knot,
                        head,
                        len: ticks,
                    });
                }
            }
            if let crate::foundation::KnotKind::SignalIn { domain } = resolved.kind {
                if !crate::runtime_impl::outbox::domain_value_is_valid(
                    domain,
                    data.sense_values[knot],
                ) {
                    return Err(RestoreError::InvalidSignal {
                        field: "sense_values",
                        knot,
                        domain,
                    });
                }
            }
            if let crate::foundation::KnotKind::Random { domain, .. } = resolved.kind {
                #[cfg(feature = "signal-f32")]
                if !f32::from_bits(data.counter[knot] as u32).is_finite() {
                    return Err(RestoreError::InvalidSignal {
                        field: "counter.random_sample",
                        knot,
                        domain,
                    });
                }
                #[cfg(feature = "signal-i32")]
                let _ = domain;
            }
        }
        Ok(())
    }

    fn inspect_data(
        &self,
        version: u32,
        numeric_path: NumericPath,
        fingerprint: u64,
        data: &RuntimeStateData,
    ) -> RuntimeStateReport {
        let mut entries = Vec::new();
        for (name, id) in &self.name_to_id {
            let i = usize::from(*id);
            match self.knots[i].kind {
                crate::foundation::KnotKind::SignalIn { .. } => {
                    entries.push(RuntimeStateEntry::Sense {
                        knot: name.clone(),
                        value: data.sense_values[i],
                    })
                }
                crate::foundation::KnotKind::Flag { .. } => entries.push(RuntimeStateEntry::Flag {
                    knot: name.clone(),
                    value: data.flag[i],
                }),
                crate::foundation::KnotKind::Counter => entries.push(RuntimeStateEntry::Counter {
                    knot: name.clone(),
                    value: data.counter[i],
                }),
                crate::foundation::KnotKind::Timer { .. } => {
                    entries.push(RuntimeStateEntry::Timer {
                        knot: name.clone(),
                        remaining: data.timer_left[i],
                    })
                }
                crate::foundation::KnotKind::Delay { ticks } => {
                    let offset = usize::from(self.delay_off[i]);
                    let len = usize::from(ticks);
                    let head = usize::from(data.delay_head[i]);
                    let values = if len == 0 {
                        Vec::new()
                    } else {
                        (0..len)
                            .map(|n| data.delay_buf[offset + ((head + n) % len)])
                            .collect()
                    };
                    entries.push(RuntimeStateEntry::Delay {
                        knot: name.clone(),
                        len: ticks,
                        head: data.delay_head[i],
                        values,
                    });
                }
                crate::foundation::KnotKind::OnStart => entries.push(RuntimeStateEntry::OnStart {
                    knot: name.clone(),
                    completed: data.on_start_done[i],
                }),
                crate::foundation::KnotKind::Random { .. } => {
                    #[cfg(feature = "signal-f32")]
                    let last_sample = f32::from_bits(data.counter[i] as u32);
                    #[cfg(feature = "signal-i32")]
                    let last_sample = data.counter[i];
                    entries.push(RuntimeStateEntry::Random {
                        knot: name.clone(),
                        last_sample,
                        stream_is_opaque: true,
                    });
                }
                crate::foundation::KnotKind::RisingFromZero
                | crate::foundation::KnotKind::FallingToZero
                | crate::foundation::KnotKind::Change => entries.push(RuntimeStateEntry::Edge {
                    knot: name.clone(),
                    previous: data.prev_in[i],
                }),
                _ => {}
            }
        }
        RuntimeStateReport {
            version,
            numeric_path,
            fingerprint,
            tick: data.tick,
            entries,
        }
    }
}

pub(crate) fn runtime_fingerprint_for(
    knots: &[ResolvedKnot],
    threads: &[(KnotId, PortSlot, KnotId, PortSlot)],
    path_names: &[String],
    cmd_names: &[String],
    max_emits_per_tick: u16,
    seed_mix: u64,
    _bind_seed: Option<Seed>,
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
    fn restore_rejects_version_shapes_and_invalid_runtime_state() {
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
        state.data.rng = 0;
        assert!(matches!(
            runtime.restore(&state),
            Err(RestoreError::InvalidRng)
        ));
    }

    #[test]
    fn restore_clears_ephemeral_outbox_effects() {
        let source = outbox_runtime();
        let state = source.snapshot();
        let mut destination = outbox_runtime();
        destination.restore(&state).unwrap();
        assert!(destination.outbox().signals().is_empty());
        assert!(destination.outbox().emits().is_empty());
    }
}
