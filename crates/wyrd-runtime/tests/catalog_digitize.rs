//! Digitize: quantize into steps over a range.

use wyrd_core::SignalDomain;
use wyrd_core::{from_count, HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::{ValidationError, Weave};
use wyrd_runtime::{cookbook::helpers::signal_out_value, BindOpts, Runtime};

#[test]
fn digitize_two_steps_endpoints() {
    let mut b = Weave::builder("d").unwrap();
    let k_in = b
        .knot("in", KnotKind::signal_in(SignalDomain::Level))
        .unwrap();
    let k_dig = b
        .knot("dig", KnotKind::digitize(2, SignalDomain::Level))
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Level))
        .unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_dig, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_dig, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let id = rt.sense_id("in").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, ZERO).unwrap();
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), ZERO);

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), ONE);
}

#[test]
fn digitize_one_step_is_out_min() {
    let mut b = Weave::builder("d").unwrap();
    let k_c = b
        .knot("c", KnotKind::constant(ONE, SignalDomain::Level))
        .unwrap();
    let k_dig = b
        .knot("dig", KnotKind::digitize(1, SignalDomain::Level))
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Level))
        .unwrap();
    let from = b.output(&k_c, "out").unwrap();
    let to = b.input(&k_dig, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_dig, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), ZERO);
}

#[test]
fn digitize_zero_span_is_out_min() {
    let mut b = Weave::builder("d").unwrap();
    let k_c = b
        .knot("c", KnotKind::constant(from_count(1), SignalDomain::Count))
        .unwrap();
    let k_dig = b
        .knot(
            "dig",
            KnotKind::Digitize {
                domain: SignalDomain::Count,
                steps: 4,
                in_min: from_count(0),
                in_max: from_count(0),
                out_min: from_count(5),
                out_max: from_count(9),
            },
        )
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Count))
        .unwrap();
    let from = b.output(&k_c, "out").unwrap();
    let to = b.input(&k_dig, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_dig, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), from_count(5));
}

#[test]
fn digitize_mid_bins_custom_out_range() {
    let mut b = Weave::builder("d").unwrap();
    let k_in = b
        .knot("in", KnotKind::signal_in(SignalDomain::Count))
        .unwrap();
    let k_dig = b
        .knot(
            "dig",
            KnotKind::Digitize {
                domain: SignalDomain::Count,
                steps: 4,
                in_min: from_count(0),
                in_max: from_count(4),
                out_min: from_count(0),
                out_max: from_count(30),
            },
        )
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Count))
        .unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_dig, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_dig, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let id = rt.sense_id("in").unwrap();

    for (input, expect) in [
        (from_count(0), from_count(0)),
        (from_count(1), from_count(10)),
        (from_count(2), from_count(20)),
        (from_count(3), from_count(30)),
        (from_count(4), from_count(30)),
    ] {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(id, input).unwrap();
        rt.loom();
        assert_eq!(signal_out_value(&rt, "y"), expect, "input bin mapping");
    }
}

#[test]
fn digitize_steps_zero_rejected_at_validate() {
    let mut b = Weave::builder("d").unwrap();
    let k_c = b
        .knot("c", KnotKind::constant(ONE, SignalDomain::Level))
        .unwrap();
    let k_dig = b
        .knot(
            "dig",
            KnotKind::Digitize {
                domain: SignalDomain::Level,
                steps: 0,
                in_min: ZERO,
                in_max: ONE,
                out_min: ZERO,
                out_max: ONE,
            },
        )
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Level))
        .unwrap();
    let from = b.output(&k_c, "out").unwrap();
    let to = b.input(&k_dig, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_dig, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    assert!(matches!(
        b.build(),
        Err(ValidationError::InvalidParameter { .. })
    ));
}

#[test]
fn digitize_inverted_in_range_rejected() {
    let mut b = Weave::builder("d").unwrap();
    let k_c = b
        .knot("c", KnotKind::constant(ONE, SignalDomain::Count))
        .unwrap();
    let k_dig = b
        .knot(
            "dig",
            KnotKind::Digitize {
                domain: SignalDomain::Count,
                steps: 4,
                in_min: from_count(5),
                in_max: from_count(1),
                out_min: ZERO,
                out_max: ONE,
            },
        )
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Count))
        .unwrap();
    let from = b.output(&k_c, "out").unwrap();
    let to = b.input(&k_dig, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_dig, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    assert!(matches!(
        b.build(),
        Err(ValidationError::InvalidParameter { .. })
    ));
}

#[test]
fn map_inverted_in_range_rejected() {
    let mut b = Weave::builder("m").unwrap();
    let k_c = b
        .knot("c", KnotKind::constant(ONE, SignalDomain::Count))
        .unwrap();
    let k_map = b
        .knot(
            "map",
            KnotKind::Map {
                domain: SignalDomain::Count,
                in_min: from_count(5),
                in_max: from_count(1),
                out_min: ZERO,
                out_max: ONE,
            },
        )
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Count))
        .unwrap();
    let from = b.output(&k_c, "out").unwrap();
    let to = b.input(&k_map, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_map, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    assert!(matches!(
        b.build(),
        Err(ValidationError::InvalidParameter { .. })
    ));
}
