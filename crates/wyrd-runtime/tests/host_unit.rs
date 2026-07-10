//! Host unit coverage (kept out of host.rs to avoid match residual line noise).

use wyrd_core::{is_truthy, CmdId, HostPathId, HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{
    append_commands, outbox_to_commands, tick_once, BindOpts, Host, HostCommand, Outbox, Runtime,
    ScriptedHost,
};

struct RecordHost {
    tick: u64,
    cmds: Vec<HostCommand>,
}

impl Host for RecordHost {
    fn time(&self) -> HostTime {
        HostTime { tick: self.tick }
    }
    fn sample_into(&mut self, _ports: &mut wyrd_runtime::PortWriter<'_>) {}
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
    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(btn, ONE);
    rt.loom(&weave).unwrap();
    let cmds = outbox_to_commands(rt.outbox());
    assert_eq!(cmds.len(), 1);
    match cmds[0] {
        HostCommand::Emit { payload, cmd } => {
            assert_eq!(payload, ZERO);
            assert_eq!(rt.cmd_name(cmd), "fire");
        }
        HostCommand::SetLevel { .. } => panic!("expected Emit"),
    }
}

#[test]
fn host_command_variants_constructible() {
    let s = HostCommand::SetLevel {
        path: HostPathId(1),
        value: ONE,
    };
    let e = HostCommand::Emit {
        cmd: CmdId(0),
        payload: ZERO,
    };
    let _ = format!("{s:?}{e:?}");
}

#[test]
fn scripted_last_commands_empty_before_tick() {
    let h = ScriptedHost::new();
    assert!(h.last_commands().is_empty());
}

#[test]
fn scripted_sample_with_no_frames_is_noop() {
    let (b, _) = Weave::builder("n")
        .knot("c", KnotKind::constant(ONE))
        .unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let mut host = ScriptedHost::new();
    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert_eq!(host.tick, 1);
    assert!(host.last_commands().is_empty());
}
