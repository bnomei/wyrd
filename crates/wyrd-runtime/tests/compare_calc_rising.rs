//! Compare rhs_const, Calc, RisingFromZero (step 1.4).

use wyrd_core::{CalcOp, CompareOp, HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

fn out_truthy(rt: &Runtime, path: &str) -> bool {
    let pid = rt.path_id(path).unwrap();
    rt.outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| wyrd_core::is_truthy(s.value))
        .unwrap_or(false)
}

#[test]
fn compare_gte_rhs_const() {
    let (b, _) = Weave::builder("cmp")
        .knot("n", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b
        .knot("cmp", KnotKind::compare(CompareOp::Gte, Some(3)))
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("ok")).unwrap();
    let weave = b
        .wire_named("n", "out", "cmp", "lhs")
        .wire_named("cmp", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let id = rt.sense_id("n").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, wyrd_core::from_count(2));
    rt.loom(&weave).unwrap();
    assert!(!out_truthy(&rt, "ok"));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, wyrd_core::from_count(3));
    rt.loom(&weave).unwrap();
    assert!(out_truthy(&rt, "ok"));
}

#[test]
fn calc_add_and_div0() {
    let (b, _) = Weave::builder("calc")
        .knot("a", KnotKind::constant(wyrd_core::from_count(6)))
        .unwrap();
    let (b, _) = b
        .knot("b", KnotKind::constant(wyrd_core::from_count(0)))
        .unwrap();
    let (b, _) = b.knot("div", KnotKind::Calc { op: CalcOp::Div }).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("q")).unwrap();
    let weave = b
        .wire_named("a", "out", "div", "a")
        .wire_named("b", "out", "div", "b")
        .wire_named("div", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom(&weave).unwrap();
    let pid = rt.path_id("q").unwrap();
    let v = rt
        .outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| s.value)
        .unwrap();
    assert_eq!(v, ZERO);
}

#[test]
fn rising_from_zero_one_tick() {
    let (b, _) = Weave::builder("rz")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("rz", KnotKind::rising_from_zero()).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("pulse")).unwrap();
    let weave = b
        .wire_named("in", "out", "rz", "in")
        .wire_named("rz", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let id = rt.sense_id("in").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, ONE);
    rt.loom(&weave).unwrap();
    assert!(out_truthy(&rt, "pulse"));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, ONE);
    rt.loom(&weave).unwrap();
    assert!(!out_truthy(&rt, "pulse"));
}
