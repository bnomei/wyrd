//! Host unit coverage (kept out of host.rs to avoid match residual line noise).

use wyrd::SignalDomain;
use wyrd::{is_truthy, HostTime, KnotKind, ONE, ZERO};
use wyrd::Weave;
use wyrd::{
    append_commands, outbox_to_commands, tick_once, BindOpts, HandleError, Host, HostCommand,
    Outbox, PortWriter, Runtime, ScriptedHost, SenseId,
};

struct RecordHost {
    tick: u64,
    cmds: Vec<HostCommand>,
}

impl Host for RecordHost {
    fn time(&self) -> HostTime {
        HostTime { tick: self.tick }
    }
    fn sample_into(
        &mut self,
        _ports: &mut wyrd::PortWriter<'_>,
    ) -> Result<(), wyrd::HandleError> {
        Ok(())
    }
    fn apply(&mut self, outbox: Outbox<'_>) {
        self.cmds.clear();
        append_commands(outbox, &mut self.cmds);
        self.tick = self.tick.wrapping_add(1);
    }
}

struct InvalidSenseHost(SenseId);

impl Host for InvalidSenseHost {
    fn time(&self) -> HostTime {
        HostTime { tick: 0 }
    }

    fn sample_into(&mut self, ports: &mut PortWriter<'_>) -> Result<(), HandleError> {
        ports.set_sense(self.0, ONE)
    }

    fn apply(&mut self, _outbox: Outbox<'_>) {
        panic!("apply must not run after a sampling error");
    }
}

#[test]
fn tick_once_maps_signal_out_to_set_level() {
    let mut b = Weave::builder("h").unwrap();
    let k_c = b
        .knot("c", KnotKind::constant(ONE, SignalDomain::Bool))
        .unwrap();
    let k_n = b.knot("n", KnotKind::not()).unwrap();
    let k_o = b
        .knot(
            "o",
            KnotKind::signal_out("debug.inverted", SignalDomain::Bool),
        )
        .unwrap();
    let from = b.output(&k_c, "out").unwrap();
    let to = b.input(&k_n, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_n, "out").unwrap();
    let to = b.input(&k_o, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let path = rt.path_id("debug.inverted").unwrap();
    let mut host = RecordHost {
        tick: 0,
        cmds: Vec::new(),
    };
    tick_once(&mut host, &mut rt).unwrap();
    assert_eq!(host.cmds.len(), 1);
    match host.cmds[0] {
        HostCommand::SetLevel { path: p, value } => {
            assert_eq!(p, path);
            assert!(!is_truthy(value));
        }
        HostCommand::Emit { .. } => panic!("expected SetLevel"),
        _ => panic!("unexpected host command"),
    }
    assert_eq!(host.tick, 1);
}

#[test]
fn outbox_to_commands_includes_emit() {
    let mut b = Weave::builder("e").unwrap();
    let k_btn = b
        .knot("btn", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_em = b.knot("em", KnotKind::emit_command("fire")).unwrap();
    let from = b.output(&k_btn, "out").unwrap();
    let to = b.input(&k_em, "trigger").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let btn = rt.sense_id("btn").unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(btn, ONE).unwrap();
    rt.loom();
    let cmds = outbox_to_commands(rt.outbox());
    assert_eq!(cmds.len(), 1);
    match cmds[0] {
        HostCommand::Emit { payload, cmd } => {
            assert_eq!(payload, ZERO);
            assert_eq!(rt.cmd_name(cmd), Ok("fire"));
        }
        HostCommand::SetLevel { .. } => panic!("expected Emit"),
        _ => panic!("unexpected host command"),
    }
}

#[test]
fn host_command_variants_constructible() {
    let mut b = Weave::builder("commands").unwrap();
    let constant = b
        .knot("constant", KnotKind::constant(ONE, SignalDomain::Bool))
        .unwrap();
    let out = b
        .knot("out", KnotKind::signal_out("out", SignalDomain::Bool))
        .unwrap();
    let emit = b.knot("emit", KnotKind::emit_command("emit")).unwrap();
    let from = b.output(&constant, "out").unwrap();
    let to = b.input(&out, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&constant, "out").unwrap();
    let to = b.input(&emit, "trigger").unwrap();
    b.connect(from, to).unwrap();
    let mut rt = Runtime::bind(b.build().unwrap(), BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    let s = HostCommand::SetLevel {
        path: rt.path_id("out").unwrap(),
        value: ONE,
    };
    let e = HostCommand::Emit {
        cmd: rt.outbox().emits()[0].cmd,
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
    let mut b = Weave::builder("n").unwrap();
    let _k_c = b
        .knot("c", KnotKind::constant(ONE, SignalDomain::Bool))
        .unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let mut host = ScriptedHost::new();
    tick_once(&mut host, &mut rt).unwrap();
    assert_eq!(host.tick, 1);
    assert!(host.last_commands().is_empty());
}

#[test]
fn tick_once_propagates_sampling_handle_error() {
    let mut b = Weave::builder("invalid-handle").unwrap();
    let _constant = b
        .knot("constant", KnotKind::constant(ONE, SignalDomain::Bool))
        .unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave, BindOpts::default()).unwrap();
    let mut other_builder = Weave::builder("other-runtime").unwrap();
    let _sense = other_builder
        .knot("sense", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let other = Runtime::bind(other_builder.build().unwrap(), BindOpts::default()).unwrap();
    let invalid = other.sense_id("sense").unwrap();
    let mut host = InvalidSenseHost(invalid);

    assert_eq!(
        tick_once(&mut host, &mut rt),
        Err(HandleError::ForeignRuntime { handle: "sense" })
    );
}
