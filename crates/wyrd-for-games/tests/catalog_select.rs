//! Select: falsey sel → a, truthy sel → b.

use wyrd::SignalDomain;
use wyrd::{from_count, HostTime, KnotKind, ONE, ZERO};
use wyrd::Weave;
use wyrd::{cookbook::helpers::signal_out_value, BindOpts, Runtime};

#[test]
fn select_a_when_sel_false() {
    let mut b = Weave::builder("s").unwrap();
    let k_sel = b
        .knot("sel", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_ca = b
        .knot("ca", KnotKind::constant(from_count(3), SignalDomain::Count))
        .unwrap();
    let k_cb = b
        .knot("cb", KnotKind::constant(from_count(7), SignalDomain::Count))
        .unwrap();
    let k_mux = b.knot("mux", KnotKind::select()).unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Count))
        .unwrap();
    let from = b.output(&k_sel, "out").unwrap();
    let to = b.input(&k_mux, "sel").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_ca, "out").unwrap();
    let to = b.input(&k_mux, "a").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_cb, "out").unwrap();
    let to = b.input(&k_mux, "b").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_mux, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let sel = rt.sense_id("sel").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(sel, ZERO).unwrap();
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), from_count(3));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(sel, ONE).unwrap();
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), from_count(7));

    // A true Bool selector selects b even when b itself is ZERO.
    let mut b = Weave::builder("s2").unwrap();
    let k_sel = b
        .knot("sel", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_ca = b
        .knot("ca", KnotKind::constant(from_count(3), SignalDomain::Count))
        .unwrap();
    let k_cb = b
        .knot("cb", KnotKind::constant(ZERO, SignalDomain::Count))
        .unwrap();
    let k_mux = b.knot("mux", KnotKind::select()).unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Count))
        .unwrap();
    let from = b.output(&k_sel, "out").unwrap();
    let to = b.input(&k_mux, "sel").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_ca, "out").unwrap();
    let to = b.input(&k_mux, "a").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_cb, "out").unwrap();
    let to = b.input(&k_mux, "b").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_mux, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let sel = rt.sense_id("sel").unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(sel, ONE).unwrap();
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), ZERO);
}
