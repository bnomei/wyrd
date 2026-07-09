//! Counter rising-edge, Flag toggle/reset, Emit rising-edge (step 1.3).

use wyrd_core::{FlagPriority, HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

fn count_out(rt: &Runtime) -> i32 {
    let pid = rt.path_id("count").unwrap();
    let v = rt
        .outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| s.value)
        .unwrap_or(ZERO);
    v as i32
}

fn signal_truthy(rt: &Runtime, path: &str) -> bool {
    let pid = rt.path_id(path).unwrap();
    rt.outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| wyrd_core::is_truthy(s.value))
        .unwrap_or(false)
}

#[test]
fn counter_rising_edge_not_level() {
    let (b, _) = Weave::builder("c").knot("inc", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("cnt", KnotKind::counter()).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("count")).unwrap();
    let weave = b
        .wire_named("inc", "out", "cnt", "inc")
        .wire_named("cnt", "count", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let id = rt.sense_id("inc").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, ONE);
    rt.loom(&weave).unwrap();
    assert_eq!(count_out(&rt), 1);

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, ONE);
    rt.loom(&weave).unwrap();
    assert_eq!(count_out(&rt), 1);

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, ZERO);
    rt.loom(&weave).unwrap();
    rt.begin_frame(HostTime { tick: 3 });
    rt.port_writer().set_sense(id, ONE);
    rt.loom(&weave).unwrap();
    assert_eq!(count_out(&rt), 2);
}

#[test]
fn flag_toggle_rising_and_reset() {
    let (b, _) = Weave::builder("f").knot("tog", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("rst", KnotKind::signal_in()).unwrap();
    let (b, _) = b
        .knot("flag", KnotKind::flag(FlagPriority::ResetWins, true))
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("lamp")).unwrap();
    let weave = b
        .wire_named("tog", "out", "flag", "toggle")
        .wire_named("rst", "out", "flag", "reset")
        .wire_named("flag", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let tog = rt.sense_id("tog").unwrap();
    let rst = rt.sense_id("rst").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(tog, ONE);
        w.set_sense(rst, ZERO);
    }
    rt.loom(&weave).unwrap();
    assert!(signal_truthy(&rt, "lamp"));

    // held toggle — no second flip
    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(tog, ONE);
        w.set_sense(rst, ZERO);
    }
    rt.loom(&weave).unwrap();
    assert!(signal_truthy(&rt, "lamp"));

    rt.begin_frame(HostTime { tick: 2 });
    {
        let mut w = rt.port_writer();
        w.set_sense(tog, ZERO);
        w.set_sense(rst, ONE);
    }
    rt.loom(&weave).unwrap();
    assert!(!signal_truthy(&rt, "lamp"));
}

#[test]
fn emit_once_on_held_trigger() {
    let (b, _) = Weave::builder("e").knot("btn", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("em", KnotKind::emit_command("fire")).unwrap();
    let weave = b.wire_named("btn", "out", "em", "trigger").build().unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let id = rt.sense_id("btn").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, ONE);
    rt.loom(&weave).unwrap();
    assert_eq!(rt.outbox().emits().len(), 1);

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, ONE);
    rt.loom(&weave).unwrap();
    assert_eq!(rt.outbox().emits().len(), 0);

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, ZERO);
    rt.loom(&weave).unwrap();
    rt.begin_frame(HostTime { tick: 3 });
    rt.port_writer().set_sense(id, ONE);
    rt.loom(&weave).unwrap();
    assert_eq!(rt.outbox().emits().len(), 1);
}
