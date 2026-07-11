//! Catalog: Sqrt.

use wyrd::SignalDomain;
use wyrd::{from_count, HostTime, KnotKind, ZERO};
use wyrd::Weave;
use wyrd::{cookbook::helpers::signal_out_value, BindOpts, Runtime};

#[test]
fn sqrt_perfect_and_negative() {
    let mut b = Weave::builder("m").unwrap();
    let k_in = b
        .knot("in", KnotKind::signal_in(SignalDomain::Count))
        .unwrap();
    let k_k = b.knot("k", KnotKind::sqrt(SignalDomain::Count)).unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Count))
        .unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_k, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_k, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let id = rt.sense_id("in").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, from_count(9)).unwrap();
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), from_count(3));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, from_count(-4)).unwrap();
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), ZERO);
}
