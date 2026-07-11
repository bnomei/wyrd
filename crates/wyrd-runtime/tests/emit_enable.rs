//! EmitCommand enable port: unconnected = enabled; wired falsey suppresses.

use wyrd_core::SignalDomain;
use wyrd_core::{HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

#[test]
fn unconnected_enable_allows_rising_emit() {
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
    assert_eq!(rt.outbox().emits().len(), 1);
}

#[test]
fn enable_low_suppresses_emit() {
    let mut b = Weave::builder("e").unwrap();
    let k_btn = b
        .knot("btn", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_en = b
        .knot("en", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_em = b.knot("em", KnotKind::emit_command("fire")).unwrap();
    let from = b.output(&k_btn, "out").unwrap();
    let to = b.input(&k_em, "trigger").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_en, "out").unwrap();
    let to = b.input(&k_em, "enable").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let btn = rt.sense_id("btn").unwrap();
    let en = rt.sense_id("en").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(btn, ONE).unwrap();
        w.set_sense(en, ZERO).unwrap();
    }
    rt.loom();
    assert_eq!(rt.outbox().emits().len(), 0);

    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(btn, ZERO).unwrap();
        w.set_sense(en, ONE).unwrap();
    }
    rt.loom();
    rt.begin_frame(HostTime { tick: 2 });
    {
        let mut w = rt.port_writer();
        w.set_sense(btn, ONE).unwrap();
        w.set_sense(en, ONE).unwrap();
    }
    rt.loom();
    assert_eq!(rt.outbox().emits().len(), 1);
}

#[test]
fn enable_open_while_trigger_held_does_not_emit() {
    // Rising edge consumed while disabled; enable alone must not fire.
    let mut b = Weave::builder("e").unwrap();
    let k_btn = b
        .knot("btn", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_en = b
        .knot("en", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_em = b.knot("em", KnotKind::emit_command("fire")).unwrap();
    let from = b.output(&k_btn, "out").unwrap();
    let to = b.input(&k_em, "trigger").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_en, "out").unwrap();
    let to = b.input(&k_em, "enable").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let btn = rt.sense_id("btn").unwrap();
    let en = rt.sense_id("en").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(btn, ONE).unwrap();
        w.set_sense(en, ZERO).unwrap();
    }
    rt.loom();
    assert_eq!(rt.outbox().emits().len(), 0);

    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(btn, ONE).unwrap(); // held
        w.set_sense(en, ONE).unwrap(); // enable opens
    }
    rt.loom();
    assert_eq!(rt.outbox().emits().len(), 0);
}

#[test]
fn enable_high_allows_emit() {
    let mut b = Weave::builder("e").unwrap();
    let k_btn = b
        .knot("btn", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_en = b
        .knot("en", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_em = b.knot("em", KnotKind::emit_command("fire")).unwrap();
    let from = b.output(&k_btn, "out").unwrap();
    let to = b.input(&k_em, "trigger").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_en, "out").unwrap();
    let to = b.input(&k_em, "enable").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let btn = rt.sense_id("btn").unwrap();
    let en = rt.sense_id("en").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(btn, ONE).unwrap();
        w.set_sense(en, ONE).unwrap();
    }
    rt.loom();
    assert_eq!(rt.outbox().emits().len(), 1);
}
