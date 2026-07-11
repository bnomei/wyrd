//! Threshold: gate with optional hysteresis and edge pulses.

use wyrd::SignalDomain;
use wyrd::{cookbook::helpers::signal_out_value, BindOpts, Runtime};
use wyrd::{from_count, is_truthy, HostTime, KnotKind, ONE, ZERO};
use wyrd::{ValidationError, Weave};

fn wire_threshold(kind: KnotKind, domain: SignalDomain) -> (Weave, Runtime) {
    let mut b = Weave::builder("t").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in(domain)).unwrap();
    let k_th = b.knot("th", kind).unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("gate", SignalDomain::Bool))
        .unwrap();
    let k_up = b
        .knot("up", KnotKind::signal_out("up", SignalDomain::Bool))
        .unwrap();
    let k_dn = b
        .knot("dn", KnotKind::signal_out("dn", SignalDomain::Bool))
        .unwrap();
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
    let (_weave, mut rt) = wire_threshold(
        KnotKind::Threshold {
            domain: SignalDomain::Count,
            high: from_count(5),
            low: from_count(0),
            use_hysteresis: false,
        },
        SignalDomain::Count,
    );
    let id = rt.sense_id("in").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, from_count(4)).unwrap();
    rt.loom();
    assert!(!is_truthy(signal_out_value(&rt, "gate")));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, from_count(5)).unwrap();
    rt.loom();
    assert!(is_truthy(signal_out_value(&rt, "gate")));
    assert!(is_truthy(signal_out_value(&rt, "up")));
    assert!(!is_truthy(signal_out_value(&rt, "dn")));

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, from_count(4)).unwrap();
    rt.loom();
    assert!(!is_truthy(signal_out_value(&rt, "gate")));
    assert!(is_truthy(signal_out_value(&rt, "dn")));
}

#[test]
fn threshold_hysteresis_band() {
    let (_weave, mut rt) = wire_threshold(
        KnotKind::threshold_default(SignalDomain::Level),
        SignalDomain::Level,
    );
    let id = rt.sense_id("in").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert!(is_truthy(signal_out_value(&rt, "gate")));

    #[cfg(feature = "signal-f32")]
    let mid = 0.45;
    #[cfg(feature = "signal-i32")]
    let mid = ONE / 2 - ONE / 20;
    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, mid).unwrap();
    rt.loom();
    assert!(is_truthy(signal_out_value(&rt, "gate")));
    assert!(!is_truthy(signal_out_value(&rt, "dn")));

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, ZERO).unwrap();
    rt.loom();
    assert!(!is_truthy(signal_out_value(&rt, "gate")));
    assert!(is_truthy(signal_out_value(&rt, "dn")));
}

#[test]
fn threshold_invalid_hysteresis_rejected() {
    let mut b = Weave::builder("t").unwrap();
    let k_in = b
        .knot("in", KnotKind::signal_in(SignalDomain::Count))
        .unwrap();
    let k_th = b
        .knot(
            "th",
            KnotKind::Threshold {
                domain: SignalDomain::Count,
                high: from_count(1),
                low: from_count(2),
                use_hysteresis: true,
            },
        )
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Bool))
        .unwrap();
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
