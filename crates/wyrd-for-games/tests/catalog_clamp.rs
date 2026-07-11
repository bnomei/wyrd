//! Catalog: Clamp.

use wyrd::SignalDomain;
use wyrd::{cookbook::helpers::signal_out_value, BindOpts, Runtime};
use wyrd::{from_count, HostTime, KnotKind};
use wyrd::{ValidationError, Weave};

#[test]
fn clamp_range() {
    let mut b = Weave::builder("m").unwrap();
    let k_in = b
        .knot("in", KnotKind::signal_in(SignalDomain::Count))
        .unwrap();
    let k_k = b
        .knot(
            "k",
            KnotKind::clamp(from_count(2), from_count(5), SignalDomain::Count),
        )
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Count))
        .unwrap();
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
    rt.port_writer().set_sense(id, from_count(1)).unwrap();
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), from_count(2));

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, from_count(3)).unwrap();
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), from_count(3));

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, from_count(9)).unwrap();
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), from_count(5));
}

#[test]
fn clamp_min_gt_max_rejected() {
    let mut b = Weave::builder("c").unwrap();
    let k_in = b
        .knot("in", KnotKind::signal_in(SignalDomain::Count))
        .unwrap();
    let k_cl = b
        .knot(
            "cl",
            KnotKind::Clamp {
                domain: SignalDomain::Count,
                min: from_count(5),
                max: from_count(1),
            },
        )
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Count))
        .unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_cl, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_cl, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    assert!(matches!(
        b.build(),
        Err(ValidationError::InvalidParameter { .. })
    ));
}
