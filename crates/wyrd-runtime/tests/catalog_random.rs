//! Seeded Random knot.

use wyrd_core::SignalDomain;
use wyrd_core::{HostTime, KnotKind, Seed, ONE, ZERO};
use wyrd_graph::{ValidationError, Weave};
use wyrd_runtime::{cookbook::helpers::signal_out_value, BindOpts, Runtime};

fn random_weave(require_gate: bool) -> Weave {
    let mut b = Weave::builder("r").unwrap();
    let k_g = b
        .knot("g", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_rnd = b
        .knot("rnd", KnotKind::random(require_gate, SignalDomain::Level))
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Level))
        .unwrap();
    let from = b.output(&k_g, "out").unwrap();
    let to = b.input(&k_rnd, "gate").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_rnd, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    b.build().unwrap()
}

#[test]
fn same_seed_same_stream() {
    let weave = random_weave(false);
    let opts = BindOpts {
        seed: Some(Seed(42)),
        ..BindOpts::default()
    };
    let mut a = Runtime::bind(weave.clone(), opts.clone()).unwrap();
    let mut b = Runtime::bind(weave.clone(), opts).unwrap();
    let mut vals_a = Vec::new();
    let mut vals_b = Vec::new();
    for t in 0..5u64 {
        a.begin_frame(HostTime { tick: t });
        a.loom();
        vals_a.push(signal_out_value(&a, "y"));
        b.begin_frame(HostTime { tick: t });
        b.loom();
        vals_b.push(signal_out_value(&b, "y"));
    }
    assert_eq!(vals_a, vals_b);
}

#[test]
fn gate_rising_samples_once() {
    let weave = random_weave(true);
    let mut rt = Runtime::bind(
        weave.clone(),
        BindOpts {
            seed: Some(Seed(7)),
            ..BindOpts::default()
        },
    )
    .unwrap();
    let g = rt.sense_id("g").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(g, ZERO).unwrap();
    rt.loom();
    let held0 = signal_out_value(&rt, "y"); // first sample false → hold 0
    assert_eq!(held0, ZERO);

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(g, ONE).unwrap();
    rt.loom();
    let v1 = signal_out_value(&rt, "y");
    #[cfg(feature = "signal-f32")]
    {
        assert!((0.0..=1.0).contains(&v1));
    }
    #[cfg(feature = "signal-i32")]
    {
        assert!(v1 >= ZERO && v1 <= ONE);
    }

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(g, ONE).unwrap(); // held — no new sample
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), v1);

    rt.begin_frame(HostTime { tick: 3 });
    rt.port_writer().set_sense(g, ZERO).unwrap();
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), v1); // still hold last

    rt.begin_frame(HostTime { tick: 4 });
    rt.port_writer().set_sense(g, ONE).unwrap();
    rt.loom();
    let v2 = signal_out_value(&rt, "y");
    assert_ne!(v1, v2);
}

#[test]
fn random_with_min_max_ports() {
    let mut b = Weave::builder("r").unwrap();
    let k_lo = b
        .knot("lo", KnotKind::constant(ZERO, SignalDomain::Level))
        .unwrap();
    let k_hi = b
        .knot("hi", KnotKind::constant(ONE, SignalDomain::Level))
        .unwrap();
    let k_rnd = b
        .knot("rnd", KnotKind::random(false, SignalDomain::Level))
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Level))
        .unwrap();
    let from = b.output(&k_lo, "out").unwrap();
    let to = b.input(&k_rnd, "min").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_hi, "out").unwrap();
    let to = b.input(&k_rnd, "max").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_rnd, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(
        weave.clone(),
        BindOpts {
            seed: Some(Seed(99)),
            ..BindOpts::default()
        },
    )
    .unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    let v = signal_out_value(&rt, "y");
    #[cfg(feature = "signal-f32")]
    {
        assert!((0.0..=1.0).contains(&v));
    }
    #[cfg(feature = "signal-i32")]
    {
        assert!(v >= ZERO && v <= ONE);
    }
}

#[test]
fn random_min_eq_max_is_constant() {
    let mut b = Weave::builder("r").unwrap();
    let k_lo = b
        .knot("lo", KnotKind::constant(ONE, SignalDomain::Level))
        .unwrap();
    let k_hi = b
        .knot("hi", KnotKind::constant(ONE, SignalDomain::Level))
        .unwrap();
    let k_rnd = b
        .knot("rnd", KnotKind::random(false, SignalDomain::Level))
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Level))
        .unwrap();
    let from = b.output(&k_lo, "out").unwrap();
    let to = b.input(&k_rnd, "min").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_hi, "out").unwrap();
    let to = b.input(&k_rnd, "max").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_rnd, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(
        weave.clone(),
        BindOpts {
            seed: Some(Seed(7)),
            ..BindOpts::default()
        },
    )
    .unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    assert_eq!(signal_out_value(&rt, "y"), ONE);
}

#[test]
fn reseed_matches_fresh_bind() {
    let weave = random_weave(false);
    let opts = BindOpts {
        seed: Some(Seed(1)),
        ..BindOpts::default()
    };
    let mut rt = Runtime::bind(weave.clone(), opts.clone()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    let first = signal_out_value(&rt, "y");
    rt.begin_frame(HostTime { tick: 1 });
    rt.loom();
    let second = signal_out_value(&rt, "y");
    assert_ne!(first, second);

    // Room retry: reseed to the same BindOpts seed restores the bind stream.
    rt.reseed(Seed(1));
    rt.begin_frame(HostTime { tick: 2 });
    rt.loom();
    let after = signal_out_value(&rt, "y");
    assert_eq!(after, first);

    let mut fresh = Runtime::bind(weave.clone(), opts).unwrap();
    fresh.begin_frame(HostTime { tick: 0 });
    fresh.loom();
    assert_eq!(signal_out_value(&fresh, "y"), after);
}

#[test]
fn require_gate_without_wire_rejected() {
    let mut b = Weave::builder("r").unwrap();
    let k_rnd = b
        .knot("rnd", KnotKind::random(true, SignalDomain::Level))
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Level))
        .unwrap();
    let from = b.output(&k_rnd, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    assert!(matches!(
        b.build(),
        Err(ValidationError::UnconnectedRequired { .. })
    ));
}
