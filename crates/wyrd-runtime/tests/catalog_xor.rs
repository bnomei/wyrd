//! Catalog: Xor.

use wyrd_core::SignalDomain;
use wyrd_core::{is_truthy, HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{cookbook::helpers::signal_out_value, BindOpts, Runtime};

#[test]
fn xor_truth_table() {
    let mut b = Weave::builder("x").unwrap();
    let k_a = b
        .knot("a", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_b = b
        .knot("b", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_x = b.knot("x", KnotKind::xor()).unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Bool))
        .unwrap();
    let from = b.output(&k_a, "out").unwrap();
    let to = b.input(&k_x, "a").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_b, "out").unwrap();
    let to = b.input(&k_x, "b").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_x, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let a = rt.sense_id("a").unwrap();
    let b_id = rt.sense_id("b").unwrap();

    for (av, bv, expect) in [
        (ZERO, ZERO, false),
        (ONE, ZERO, true),
        (ZERO, ONE, true),
        (ONE, ONE, false),
    ] {
        rt.begin_frame(HostTime { tick: 0 });
        {
            let mut w = rt.port_writer();
            w.set_sense(a, av).unwrap();
            w.set_sense(b_id, bv).unwrap();
        }
        rt.loom();
        assert_eq!(is_truthy(signal_out_value(&rt, "y")), expect);
    }
}
