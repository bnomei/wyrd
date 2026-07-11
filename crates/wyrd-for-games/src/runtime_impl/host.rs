//! Host-facing tick: begin_frame → sample → loom → apply.
//!
//! Dense ids only on the hot path (D-id-space / D-hostpath). Loom never sees
//! engine types; apply maps the outbox into [`HostCommand`]s or host-local effects.
//! Resolve `SenseId` / `HostPathId` once after bind — not each sample.

use std::vec::Vec;

use crate::foundation::{HostTime, Signal};

use crate::runtime_impl::bind::Runtime;
use crate::runtime_impl::error::HandleError;
use crate::runtime_impl::handles::{CmdId, HostPathId, SenseId};
use crate::runtime_impl::outbox::{Outbox, PortWriter};

/// Dense command emitted for host apply (from SignalOut / EmitCommand).
///
/// `SetLevel` is host pedagogy language for any SignalOut sample (full Signal).
#[derive(Copy, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum HostCommand {
    SetLevel { path: HostPathId, value: Signal },
    Emit { cmd: CmdId, payload: Signal },
}

/// Append outbox contents as dense host commands (signals then emits).
///
/// Prefer iterating [`Outbox::signals`] / [`Outbox::emits`] on the hottest
/// paths; use this helper when a command list is convenient (tests, scripted
/// hosts). Reuse `out` across ticks to avoid fresh allocation.
pub fn append_commands(outbox: Outbox<'_>, out: &mut Vec<HostCommand>) {
    out.reserve(outbox.signals().len() + outbox.emits().len());
    for s in outbox.signals() {
        out.push(HostCommand::SetLevel {
            path: s.path,
            value: s.value,
        });
    }
    for e in outbox.emits() {
        out.push(HostCommand::Emit {
            cmd: e.cmd,
            payload: e.payload,
        });
    }
}

/// Map a loom outbox into a new command vec (allocates). See [`append_commands`].
pub fn outbox_to_commands(outbox: Outbox<'_>) -> Vec<HostCommand> {
    let mut cmds = Vec::new();
    append_commands(outbox, &mut cmds);
    cmds
}

/// Engine-neutral host: sample senses into a [`PortWriter`], apply outbox after loom.
///
/// Implement on a concrete type; do not use `dyn Host` on the hot path.
/// Hold dense [`SenseId`]s on the host; do not call `Runtime::sense_id` every tick.
pub trait Host {
    fn time(&self) -> HostTime;
    /// Write sense ports for this tick (dense `set_sense` only).
    fn sample_into(&mut self, ports: &mut PortWriter<'_>) -> Result<(), HandleError>;
    fn apply(&mut self, outbox: Outbox<'_>);
}

/// One host tick: begin_frame → sample → loom → apply.
pub fn tick_once(host: &mut impl Host, rt: &mut Runtime) -> Result<(), HandleError> {
    rt.begin_frame(host.time());
    {
        let mut w = rt.port_writer();
        host.sample_into(&mut w)?;
    }
    rt.loom();
    host.apply(rt.outbox());
    Ok(())
}

/// No-op host (benches, placeholder worlds).
#[derive(Clone, Debug, Default)]
pub struct NullHost {
    pub tick: u64,
}

impl Host for NullHost {
    fn time(&self) -> HostTime {
        HostTime { tick: self.tick }
    }
    fn sample_into(&mut self, _ports: &mut PortWriter<'_>) -> Result<(), HandleError> {
        Ok(())
    }
    fn apply(&mut self, _outbox: Outbox<'_>) {
        self.tick = self.tick.wrapping_add(1);
    }
}

/// Scripted senses + recorded apply commands for deterministic replay tests.
///
/// Hold dense [`SenseId`]s resolved after bind. Each frame is a **write list**
/// (not a full port snapshot) applied in `sample_into` when
/// `HostTime.tick == frame_index`. Missing sense keys **hold last** value
/// (runtime does not clear senses each frame). Ticks past `frames.len()`
/// write nothing (last values remain). Prefer setting every sense every frame
/// in tests so scripts stay explicit.
///
/// After each tick, `commands` grows with that frame's outbox mapping.
#[derive(Clone, Debug, Default)]
pub struct ScriptedHost {
    pub tick: u64,
    /// Write list for sample when `tick == i` (before `apply` increments tick).
    pub frames: Vec<Vec<(SenseId, Signal)>>,
    /// Flattened HostCommand history (append-only across ticks).
    pub commands: Vec<HostCommand>,
    /// Per-tick command counts (for slicing `commands`).
    pub commands_per_tick: Vec<usize>,
}

impl ScriptedHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push one frame of dense sense writes (order preserved).
    pub fn push_frame(&mut self, senses: impl IntoIterator<Item = (SenseId, Signal)>) {
        self.frames.push(senses.into_iter().collect());
    }

    /// Commands applied on the last completed tick (after `tick_once`).
    pub fn last_commands(&self) -> &[HostCommand] {
        let Some(&n) = self.commands_per_tick.last() else {
            return &[];
        };
        let start = self.commands.len().saturating_sub(n);
        &self.commands[start..]
    }
}

impl Host for ScriptedHost {
    fn time(&self) -> HostTime {
        HostTime { tick: self.tick }
    }

    fn sample_into(&mut self, ports: &mut PortWriter<'_>) -> Result<(), HandleError> {
        let i = self.tick as usize;
        let Some(frame) = self.frames.get(i) else {
            return Ok(());
        };
        for &(id, v) in frame {
            ports.set_sense(id, v)?;
        }
        Ok(())
    }

    fn apply(&mut self, outbox: Outbox<'_>) {
        let before = self.commands.len();
        append_commands(outbox, &mut self.commands);
        self.commands_per_tick
            .push(self.commands.len().saturating_sub(before));
        self.tick = self.tick.wrapping_add(1);
    }
}
