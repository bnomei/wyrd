//! Catalog: FallingToZero.

mod common;

use common::signal_out_value;
use wyrd::SignalDomain;
use wyrd::Weave;
use wyrd::{is_truthy, HostTime, KnotKind, ONE, ZERO};
use wyrd::{BindOpts, Runtime};

#[test]
fn falling_to_zero_edge() {
    let mut b = Weave::builder("m").unwrap();
    let k_in = b
        .knot("in", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_k = b.knot("k", KnotKind::falling_to_zero()).unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Bool))
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
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert!(!is_truthy(signal_out_value(&rt, "y")));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, ZERO).unwrap();
    rt.loom();
    assert!(is_truthy(signal_out_value(&rt, "y")));

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, ZERO).unwrap();
    rt.loom();
    assert!(!is_truthy(signal_out_value(&rt, "y")));
}
