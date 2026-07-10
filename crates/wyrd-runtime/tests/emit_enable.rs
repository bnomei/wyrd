//! EmitCommand enable port: unconnected = enabled; wired falsey suppresses.

use wyrd_core::{HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

#[test]
fn unconnected_enable_allows_rising_emit() {
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
    assert_eq!(rt.outbox().emits().len(), 1);
}

#[test]
fn enable_low_suppresses_emit() {
    let (b, _) = Weave::builder("e")
        .knot("btn", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("en", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("em", KnotKind::emit_command("fire")).unwrap();
    let weave = b
        .wire_named("btn", "out", "em", "trigger")
        .wire_named("en", "out", "em", "enable")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let btn = rt.sense_id("btn").unwrap();
    let en = rt.sense_id("en").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(btn, ONE);
        w.set_sense(en, ZERO);
    }
    rt.loom(&weave).unwrap();
    assert_eq!(rt.outbox().emits().len(), 0);

    // rising again after enable high
    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(btn, ZERO);
        w.set_sense(en, ONE);
    }
    rt.loom(&weave).unwrap();
    rt.begin_frame(HostTime { tick: 2 });
    {
        let mut w = rt.port_writer();
        w.set_sense(btn, ONE);
        w.set_sense(en, ONE);
    }
    rt.loom(&weave).unwrap();
    assert_eq!(rt.outbox().emits().len(), 1);
}

#[test]
fn enable_high_allows_emit() {
    let (b, _) = Weave::builder("e")
        .knot("btn", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("en", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("em", KnotKind::emit_command("fire")).unwrap();
    let weave = b
        .wire_named("btn", "out", "em", "trigger")
        .wire_named("en", "out", "em", "enable")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let btn = rt.sense_id("btn").unwrap();
    let en = rt.sense_id("en").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(btn, ONE);
        w.set_sense(en, ONE);
    }
    rt.loom(&weave).unwrap();
    assert_eq!(rt.outbox().emits().len(), 1);
}
