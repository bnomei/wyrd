//! Catalog: Change.

use wyrd_core::{is_truthy, HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{cookbook::helpers::signal_out_value, BindOpts, Runtime};

#[test]
fn change_either_edge() {
    let mut b = Weave::builder("m").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
    let k_k = b.knot("k", KnotKind::change()).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
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
    assert!(is_truthy(signal_out_value(&rt, "y"))); // 0→1

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert!(!is_truthy(signal_out_value(&rt, "y")));

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, ZERO).unwrap();
    rt.loom();
    assert!(is_truthy(signal_out_value(&rt, "y"))); // 1→0
}
