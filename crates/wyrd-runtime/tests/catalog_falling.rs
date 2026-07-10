//! Catalog: FallingToZero.

use wyrd_core::{is_truthy, HostTime, KnotKind, ONE, ZERO};
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
fn falling_to_zero_edge() {
    let (b, _) = Weave::builder("m")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("k", KnotKind::falling_to_zero()).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("in", "out", "k", "in")
        .wire_named("k", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let id = rt.sense_id("in").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, ONE);
    rt.loom(&weave).unwrap();
    assert!(!is_truthy(out_v(&rt, "y")));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, ZERO);
    rt.loom(&weave).unwrap();
    assert!(is_truthy(out_v(&rt, "y")));

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, ZERO);
    rt.loom(&weave).unwrap();
    assert!(!is_truthy(out_v(&rt, "y")));
}
