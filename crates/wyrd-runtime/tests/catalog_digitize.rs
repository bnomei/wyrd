//! Digitize: quantize into steps over a range.

use wyrd_core::{from_count, HostTime, KnotKind, ONE, ZERO};
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
fn digitize_two_steps_endpoints() {
    // steps=2 over 0..ONE → bins 0 and 1 map to ZERO and ONE
    let (b, _) = Weave::builder("d")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("dig", KnotKind::digitize(2)).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("in", "out", "dig", "in")
        .wire_named("dig", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let id = rt.sense_id("in").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, ZERO);
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "y"), ZERO);

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, ONE);
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "y"), ONE);
}

#[test]
fn digitize_one_step_is_out_min() {
    let (b, _) = Weave::builder("d")
        .knot("c", KnotKind::constant(ONE))
        .unwrap();
    let (b, _) = b.knot("dig", KnotKind::digitize(1)).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("c", "out", "dig", "in")
        .wire_named("dig", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "y"), ZERO);
}

#[test]
fn digitize_zero_span_is_out_min() {
    let (b, _) = Weave::builder("d")
        .knot("c", KnotKind::constant(from_count(1)))
        .unwrap();
    let (b, _) = b
        .knot(
            "dig",
            KnotKind::Digitize {
                steps: 4,
                in_min: from_count(0),
                in_max: from_count(0),
                out_min: from_count(5),
                out_max: from_count(9),
            },
        )
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("c", "out", "dig", "in")
        .wire_named("dig", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "y"), from_count(5));
}

#[test]
fn digitize_steps_zero_rejected_at_validate() {
    let (b, _) = Weave::builder("d")
        .knot("c", KnotKind::constant(ONE))
        .unwrap();
    let (b, _) = b
        .knot(
            "dig",
            KnotKind::Digitize {
                steps: 0,
                in_min: ZERO,
                in_max: ONE,
                out_min: ZERO,
                out_max: ONE,
            },
        )
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("c", "out", "dig", "in")
        .wire_named("dig", "out", "out", "in")
        .build()
        .unwrap();
    assert_eq!(
        validate(&weave, &Budget::default()),
        Err(wyrd_core::WyrdError::InvalidParam)
    );
}
