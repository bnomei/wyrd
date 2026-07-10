//! Threshold: gate with optional hysteresis and edge pulses.

use wyrd_core::{from_count, is_truthy, HostTime, KnotKind, ONE, ZERO};
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

fn wire_threshold(kind: KnotKind) -> (Weave, Runtime) {
    let (b, _) = Weave::builder("t")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("th", kind).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("gate")).unwrap();
    let (b, _) = b.knot("up", KnotKind::signal_out("up")).unwrap();
    let (b, _) = b.knot("dn", KnotKind::signal_out("dn")).unwrap();
    let weave = b
        .wire_named("in", "out", "th", "in")
        .wire_named("th", "out", "out", "in")
        .wire_named("th", "crossed_up", "up", "in")
        .wire_named("th", "crossed_down", "dn", "in")
        .build()
        .unwrap();
    let rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    (weave, rt)
}

#[test]
fn threshold_simple_no_hysteresis() {
    let (weave, mut rt) = wire_threshold(KnotKind::Threshold {
        high: from_count(5),
        low: from_count(0),
        use_hysteresis: false,
    });
    let id = rt.sense_id("in").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, from_count(4));
    rt.loom(&weave).unwrap();
    assert!(!is_truthy(out_v(&rt, "gate")));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, from_count(5));
    rt.loom(&weave).unwrap();
    assert!(is_truthy(out_v(&rt, "gate")));
    assert!(is_truthy(out_v(&rt, "up")));
    assert!(!is_truthy(out_v(&rt, "dn")));

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, from_count(4));
    rt.loom(&weave).unwrap();
    assert!(!is_truthy(out_v(&rt, "gate")));
    assert!(is_truthy(out_v(&rt, "dn")));
}

#[test]
fn threshold_hysteresis_band() {
    let (weave, mut rt) = wire_threshold(KnotKind::threshold_default());
    let id = rt.sense_id("in").unwrap();

    // rise through high
    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, ONE);
    rt.loom(&weave).unwrap();
    assert!(is_truthy(out_v(&rt, "gate")));

    // still high while between low and high (0.45 if ONE=1)
    #[cfg(feature = "signal-f32")]
    let mid = 0.45;
    #[cfg(feature = "signal-i32")]
    let mid = ONE / 2 - ONE / 20;
    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, mid);
    rt.loom(&weave).unwrap();
    assert!(is_truthy(out_v(&rt, "gate")));
    assert!(!is_truthy(out_v(&rt, "dn")));

    // fall below low
    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, ZERO);
    rt.loom(&weave).unwrap();
    assert!(!is_truthy(out_v(&rt, "gate")));
    assert!(is_truthy(out_v(&rt, "dn")));
}

#[test]
fn threshold_invalid_hysteresis_rejected() {
    let (b, _) = Weave::builder("t")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b
        .knot(
            "th",
            KnotKind::Threshold {
                high: from_count(1),
                low: from_count(2),
                use_hysteresis: true,
            },
        )
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("in", "out", "th", "in")
        .wire_named("th", "out", "out", "in")
        .build()
        .unwrap();
    assert_eq!(
        validate(&weave, &Budget::default()),
        Err(wyrd_core::WyrdError::InvalidParam)
    );
}
