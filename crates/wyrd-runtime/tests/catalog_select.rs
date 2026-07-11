//! Select: falsey sel → a, truthy sel → b.

use wyrd_core::{from_count, HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{cookbook::helpers::signal_out_value, BindOpts, Runtime};

#[test]
fn select_a_when_sel_false() {
    let mut b = Weave::builder("s").unwrap();
    let k_sel = b.knot("sel", KnotKind::signal_in()).unwrap();
    let k_ca = b.knot("ca", KnotKind::constant(from_count(3))).unwrap();
    let k_cb = b.knot("cb", KnotKind::constant(from_count(7))).unwrap();
    let k_mux = b.knot("mux", KnotKind::select()).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
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

    // Non-ONE truthy still selects b; b can be ZERO (forward, don't force ONE).
    let mut b = Weave::builder("s2").unwrap();
    let k_sel = b.knot("sel", KnotKind::signal_in()).unwrap();
    let k_ca = b.knot("ca", KnotKind::constant(from_count(3))).unwrap();
    let k_cb = b.knot("cb", KnotKind::constant(ZERO)).unwrap();
    let k_mux = b.knot("mux", KnotKind::select()).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
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
    rt.port_writer().set_sense(sel, from_count(2)).unwrap();
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), ZERO);
}
