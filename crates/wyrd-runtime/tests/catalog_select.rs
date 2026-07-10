//! Select: falsey sel → a, truthy sel → b.

use wyrd_core::{from_count, HostTime, KnotKind, ONE, ZERO};
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
fn select_a_when_sel_false() {
    let (b, _) = Weave::builder("s")
        .knot("sel", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("ca", KnotKind::constant(from_count(3))).unwrap();
    let (b, _) = b.knot("cb", KnotKind::constant(from_count(7))).unwrap();
    let (b, _) = b.knot("mux", KnotKind::select()).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("sel", "out", "mux", "sel")
        .wire_named("ca", "out", "mux", "a")
        .wire_named("cb", "out", "mux", "b")
        .wire_named("mux", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let sel = rt.sense_id("sel").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(sel, ZERO);
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "y"), from_count(3));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(sel, ONE);
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "y"), from_count(7));
}
