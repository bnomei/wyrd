//! Delay N-tick ring (step 1.2).

use wyrd_core::{HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

fn out_val(rt: &Runtime, path: &str) -> wyrd_core::Signal {
    let pid = rt.path_id(path).unwrap();
    rt.outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| s.value)
        .unwrap_or(ZERO)
}

fn tick(rt: &mut Runtime, weave: &Weave, t: u64, v: wyrd_core::Signal) {
    rt.begin_frame(HostTime { tick: t });
    let id = rt.sense_id("in").unwrap();
    rt.port_writer().set_sense(id, v);
    rt.loom(weave).unwrap();
}

#[test]
fn delay_zero_is_passthrough() {
    let (b, _) = Weave::builder("d0").knot("in", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("d", KnotKind::Delay { ticks: 0 }).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("in", "out", "d", "in")
        .wire_named("d", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    tick(&mut rt, &weave, 0, ONE);
    assert!(wyrd_core::is_truthy(out_val(&rt, "y")));
}

#[test]
fn delay_three_ticks() {
    let (b, _) = Weave::builder("d3").knot("in", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("d", KnotKind::Delay { ticks: 3 }).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("in", "out", "d", "in")
        .wire_named("d", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();

    tick(&mut rt, &weave, 0, ONE);
    assert!(!wyrd_core::is_truthy(out_val(&rt, "y")));
    tick(&mut rt, &weave, 1, ZERO);
    assert!(!wyrd_core::is_truthy(out_val(&rt, "y")));
    tick(&mut rt, &weave, 2, ZERO);
    assert!(!wyrd_core::is_truthy(out_val(&rt, "y")));
    // 3rd loom after inject: ONE appears
    tick(&mut rt, &weave, 3, ZERO);
    assert!(wyrd_core::is_truthy(out_val(&rt, "y")));
    tick(&mut rt, &weave, 4, ZERO);
    assert!(!wyrd_core::is_truthy(out_val(&rt, "y")));
}
