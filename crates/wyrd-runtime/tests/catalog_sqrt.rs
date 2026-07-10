//! Catalog: Sqrt.

use wyrd_core::{from_count, HostTime, KnotKind, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

fn out_v(rt: &Runtime, path: &str) -> wyrd_core::Signal {
    let pid = rt.path_id(path).unwrap();
    rt.outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| s.value)
        .unwrap_or(ZERO)
}

#[test]
fn sqrt_perfect_and_negative() {
    let mut b = Weave::builder("m").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
    let k_k = b.knot("k", KnotKind::sqrt()).unwrap();
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
    rt.port_writer().set_sense(id, from_count(9)).unwrap();
    rt.loom();
    assert_eq!(out_v(&rt, "y"), from_count(3));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, from_count(-4)).unwrap();
    rt.loom();
    assert_eq!(out_v(&rt, "y"), ZERO);
}
