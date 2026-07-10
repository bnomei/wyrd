//! Seeded Random knot.

use wyrd_core::{HostTime, KnotKind, Seed, ONE, ZERO};
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

fn random_weave(require_gate: bool) -> Weave {
    let mut b = Weave::builder("r").unwrap();
    let k_g = b.knot("g", KnotKind::signal_in()).unwrap();
    let k_rnd = b.knot("rnd", KnotKind::random(require_gate)).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
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
        vals_a.push(out_v(&a, "y"));
        b.begin_frame(HostTime { tick: t });
        b.loom();
        vals_b.push(out_v(&b, "y"));
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
    let held0 = out_v(&rt, "y"); // first sample false → hold 0
    assert_eq!(held0, ZERO);

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(g, ONE).unwrap();
    rt.loom();
    let v1 = out_v(&rt, "y");
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
    assert_eq!(out_v(&rt, "y"), v1);

    rt.begin_frame(HostTime { tick: 3 });
    rt.port_writer().set_sense(g, ZERO).unwrap();
    rt.loom();
    assert_eq!(out_v(&rt, "y"), v1); // still hold last

    rt.begin_frame(HostTime { tick: 4 });
    rt.port_writer().set_sense(g, ONE).unwrap();
    rt.loom();
    let v2 = out_v(&rt, "y");
    assert_ne!(v1, v2);
}

#[test]
fn random_with_min_max_ports() {
    let mut b = Weave::builder("r").unwrap();
    let k_lo = b.knot("lo", KnotKind::constant(ZERO)).unwrap();
    let k_hi = b.knot("hi", KnotKind::constant(ONE)).unwrap();
    let k_rnd = b.knot("rnd", KnotKind::random(false)).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
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
    let v = out_v(&rt, "y");
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
    let k_lo = b.knot("lo", KnotKind::constant(ONE)).unwrap();
    let k_hi = b.knot("hi", KnotKind::constant(ONE)).unwrap();
    let k_rnd = b.knot("rnd", KnotKind::random(false)).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
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
    assert_eq!(out_v(&rt, "y"), ONE);
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
    let first = out_v(&rt, "y");
    rt.begin_frame(HostTime { tick: 1 });
    rt.loom();
    let second = out_v(&rt, "y");
    assert_ne!(first, second);

    // Room retry: reseed to the same BindOpts seed restores the bind stream.
    rt.reseed(Seed(1));
    rt.begin_frame(HostTime { tick: 2 });
    rt.loom();
    let after = out_v(&rt, "y");
    assert_eq!(after, first);

    let mut fresh = Runtime::bind(weave.clone(), opts).unwrap();
    fresh.begin_frame(HostTime { tick: 0 });
    fresh.loom();
    assert_eq!(out_v(&fresh, "y"), after);
}

#[test]
fn require_gate_without_wire_rejected() {
    let mut b = Weave::builder("r").unwrap();
    let k_rnd = b.knot("rnd", KnotKind::random(true)).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_rnd, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    assert!(matches!(
        b.build(),
        Err(ValidationError::UnconnectedRequired { .. })
    ));
}
