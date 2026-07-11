//! Compare rhs_const, Calc, RisingFromZero (step 1.4).

use wyrd::SignalDomain;
use wyrd::{CalcOp, CompareOp, HostTime, KnotKind, ONE, ZERO};
use wyrd::Weave;
use wyrd::{
    cookbook::helpers::{signal_out_truthy, signal_out_value},
    BindOpts, Runtime,
};

#[test]
fn compare_gte_rhs_const() {
    let mut b = Weave::builder("cmp").unwrap();
    let k_n = b
        .knot("n", KnotKind::signal_in(SignalDomain::Count))
        .unwrap();
    let k_cmp = b
        .knot(
            "cmp",
            KnotKind::compare(
                CompareOp::Gte,
                Some(wyrd::from_count(3)),
                SignalDomain::Count,
            ),
        )
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("ok", SignalDomain::Bool))
        .unwrap();
    let from = b.output(&k_n, "out").unwrap();
    let to = b.input(&k_cmp, "lhs").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_cmp, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let id = rt.sense_id("n").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer()
        .set_sense(id, wyrd::from_count(2))
        .unwrap();
    rt.loom();
    assert!(!signal_out_truthy(&rt, "ok"));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer()
        .set_sense(id, wyrd::from_count(3))
        .unwrap();
    rt.loom();
    assert!(signal_out_truthy(&rt, "ok"));
}

#[test]
fn calc_add_and_div0() {
    let mut b = Weave::builder("calc").unwrap();
    let k_a = b
        .knot(
            "a",
            KnotKind::constant(wyrd::from_count(6), SignalDomain::Count),
        )
        .unwrap();
    let k_b = b
        .knot(
            "b",
            KnotKind::constant(wyrd::from_count(0), SignalDomain::Count),
        )
        .unwrap();
    let k_div = b
        .knot(
            "div",
            KnotKind::Calc {
                domain: SignalDomain::Count,
                op: CalcOp::Div,
            },
        )
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("q", SignalDomain::Count))
        .unwrap();
    let from = b.output(&k_a, "out").unwrap();
    let to = b.input(&k_div, "a").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_b, "out").unwrap();
    let to = b.input(&k_div, "b").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_div, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    assert_eq!(signal_out_value(&rt, "q"), ZERO);
}

#[test]
fn rising_from_zero_one_tick() {
    let mut b = Weave::builder("rz").unwrap();
    let k_in = b
        .knot("in", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_rz = b.knot("rz", KnotKind::rising_from_zero()).unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("pulse", SignalDomain::Bool))
        .unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_rz, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_rz, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let id = rt.sense_id("in").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert!(signal_out_truthy(&rt, "pulse"));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert!(!signal_out_truthy(&rt, "pulse"));
}
