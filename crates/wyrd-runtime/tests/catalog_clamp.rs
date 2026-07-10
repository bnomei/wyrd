//! Catalog: Clamp.

use wyrd_core::{from_count, HostTime, KnotKind, ZERO};
use wyrd_graph::{validate, Budget, Weave};
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
fn clamp_range() {
    let (b, _) = Weave::builder("m")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b
        .knot("k", KnotKind::clamp(from_count(2), from_count(5)))
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("in", "out", "k", "in")
        .wire_named("k", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let id = rt.sense_id("in").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, from_count(1));
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "y"), from_count(2));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, from_count(3));
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "y"), from_count(3));

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, from_count(9));
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "y"), from_count(5));
}

#[test]
fn clamp_min_gt_max_rejected() {
    let (b, _) = Weave::builder("c")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b
        .knot(
            "cl",
            KnotKind::Clamp {
                min: from_count(5),
                max: from_count(1),
            },
        )
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("in", "out", "cl", "in")
        .wire_named("cl", "out", "out", "in")
        .build()
        .unwrap();
    assert_eq!(
        validate(&weave, &Budget::default()),
        Err(wyrd_core::WyrdError::InvalidParam)
    );
}
