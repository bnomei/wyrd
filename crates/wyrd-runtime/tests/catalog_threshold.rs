//! Threshold: gate with optional hysteresis and edge pulses.

use wyrd_core::{from_count, is_truthy, HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::{ValidationError, Weave};
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
    let mut b = Weave::builder("t").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
    let k_th = b.knot("th", kind).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("gate")).unwrap();
    let k_up = b.knot("up", KnotKind::signal_out("up")).unwrap();
    let k_dn = b.knot("dn", KnotKind::signal_out("dn")).unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_th, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_th, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_th, "crossed_up").unwrap();
    let to = b.input(&k_up, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_th, "crossed_down").unwrap();
    let to = b.input(&k_dn, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    (weave, rt)
}

#[test]
fn threshold_simple_no_hysteresis() {
    let (_weave, mut rt) = wire_threshold(KnotKind::Threshold {
        high: from_count(5),
        low: from_count(0),
        use_hysteresis: false,
    });
    let id = rt.sense_id("in").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, from_count(4)).unwrap();
    rt.loom();
    assert!(!is_truthy(out_v(&rt, "gate")));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, from_count(5)).unwrap();
    rt.loom();
    assert!(is_truthy(out_v(&rt, "gate")));
    assert!(is_truthy(out_v(&rt, "up")));
    assert!(!is_truthy(out_v(&rt, "dn")));

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, from_count(4)).unwrap();
    rt.loom();
    assert!(!is_truthy(out_v(&rt, "gate")));
    assert!(is_truthy(out_v(&rt, "dn")));
}

#[test]
fn threshold_hysteresis_band() {
    let (_weave, mut rt) = wire_threshold(KnotKind::threshold_default());
    let id = rt.sense_id("in").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert!(is_truthy(out_v(&rt, "gate")));

    #[cfg(feature = "signal-f32")]
    let mid = 0.45;
    #[cfg(feature = "signal-i32")]
    let mid = ONE / 2 - ONE / 20;
    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, mid).unwrap();
    rt.loom();
    assert!(is_truthy(out_v(&rt, "gate")));
    assert!(!is_truthy(out_v(&rt, "dn")));

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, ZERO).unwrap();
    rt.loom();
    assert!(!is_truthy(out_v(&rt, "gate")));
    assert!(is_truthy(out_v(&rt, "dn")));
}

#[test]
fn threshold_invalid_hysteresis_rejected() {
    let mut b = Weave::builder("t").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
    let k_th = b
        .knot(
            "th",
            KnotKind::Threshold {
                high: from_count(1),
                low: from_count(2),
                use_hysteresis: true,
            },
        )
        .unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_th, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_th, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    assert!(matches!(
        b.build(),
        Err(ValidationError::InvalidParameter { .. })
    ));
}
