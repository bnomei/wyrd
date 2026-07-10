//! Host-facing tick: begin_frame → sample → loom → apply.
//!
//! Dense ids only on the hot path (D-id-space / D-hostpath). Loom never sees
//! engine types; apply maps the outbox into [`HostCommand`]s or host-local effects.
//! Resolve `KnotId` / `HostPathId` once at bind — not each sample.

use std::vec::Vec;

use wyrd_core::{CmdId, HostPathId, HostTime, Result, Signal};
use wyrd_graph::Weave;

use crate::bind::Runtime;
use crate::outbox::{Outbox, PortWriter};

/// Dense command emitted for host apply (from SignalOut / EmitCommand).
///
/// `SetLevel` is host pedagogy language for any SignalOut sample (full Signal).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HostCommand {
    SetLevel {
        path: HostPathId,
        value: Signal,
    },
    Emit {
        cmd: CmdId,
        payload: Signal,
    },
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
/// Hold dense `KnotId`s on the host; do not call `sense_id` every tick.
pub trait Host {
    fn time(&self) -> HostTime;
    /// Write sense ports for this tick (dense `set_sense` only).
    fn sample_into(&mut self, ports: &mut PortWriter<'_>);
    fn apply(&mut self, outbox: Outbox<'_>);
}

/// One host tick: begin_frame → sample → loom → apply.
pub fn tick_once(host: &mut impl Host, rt: &mut Runtime, weave: &Weave) -> Result<()> {
    rt.begin_frame(host.time());
    {
        let mut w = rt.port_writer();
        host.sample_into(&mut w);
    }
    rt.loom(weave)?;
    host.apply(rt.outbox());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wyrd_core::{is_truthy, KnotKind, ONE, ZERO};
    use wyrd_graph::Weave;

    use crate::bind::BindOpts;

    /// Minimal host that only advances tick and records commands.
    struct RecordHost {
        tick: u64,
        cmds: Vec<HostCommand>,
    }

    impl Host for RecordHost {
        fn time(&self) -> HostTime {
            HostTime { tick: self.tick }
        }
        fn sample_into(&mut self, _ports: &mut PortWriter<'_>) {}
        fn apply(&mut self, outbox: Outbox<'_>) {
            self.cmds.clear();
            append_commands(outbox, &mut self.cmds);
            self.tick = self.tick.wrapping_add(1);
        }
    }

    #[test]
    fn tick_once_maps_signal_out_to_set_level() {
        let (b, _) = Weave::builder("h")
            .knot("c", KnotKind::constant(ONE))
            .unwrap();
        let (b, _) = b.knot("n", KnotKind::not()).unwrap();
        let (b, _) = b.knot("o", KnotKind::signal_out("debug.inverted")).unwrap();
        let weave = b
            .wire_named("c", "out", "n", "in")
            .wire_named("n", "out", "o", "in")
            .build()
            .unwrap();
        let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
        let path = rt.path_id("debug.inverted").unwrap();
        let mut host = RecordHost {
            tick: 0,
            cmds: Vec::new(),
        };

        tick_once(&mut host, &mut rt, &weave).unwrap();

        assert_eq!(host.cmds.len(), 1);
        match host.cmds[0] {
            HostCommand::SetLevel { path: p, value } => {
                assert_eq!(p, path);
                assert!(!is_truthy(value));
            }
            HostCommand::Emit { .. } => panic!("expected SetLevel"),
        }
        assert_eq!(host.tick, 1);
    }

    #[test]
    fn outbox_to_commands_includes_emit() {
        let (b, _) = Weave::builder("e")
            .knot("btn", KnotKind::signal_in())
            .unwrap();
        let (b, _) = b.knot("em", KnotKind::emit_command("fire")).unwrap();
        let weave = b.wire_named("btn", "out", "em", "trigger").build().unwrap();
        let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
        let btn = rt.sense_id("btn").unwrap();
        let cmd = {
            // rising edge emit
            rt.begin_frame(HostTime { tick: 0 });
            rt.port_writer().set_sense(btn, ONE);
            rt.loom(&weave).unwrap();
            let cmds = outbox_to_commands(rt.outbox());
            assert_eq!(cmds.len(), 1);
            match cmds[0] {
                HostCommand::Emit { cmd, payload } => {
                    assert_eq!(payload, ZERO); // default unconnected payload
                    cmd
                }
                HostCommand::SetLevel { .. } => panic!("expected Emit"),
            }
        };
        assert_eq!(rt.cmd_name(cmd), "fire");
    }
}
