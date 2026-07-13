//! Runtime snapshot continuation and compatibility contracts.

use wyrd::{
    BindOpts, HostTime, KnotKind, RestoreError, Runtime, Signal, SignalDomain, Weave, ONE, ZERO,
};

fn weave(id: &str) -> Weave {
    weave_named(id, "value", "pulse")
}

fn weave_named(id: &str, output_path: &str, command_name: &str) -> Weave {
    let mut builder = Weave::builder(id).unwrap();
    let input = builder
        .knot("input", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let delay = builder.knot("delay", KnotKind::Delay { ticks: 2 }).unwrap();
    let counter = builder.knot("counter", KnotKind::counter()).unwrap();
    let output = builder
        .knot(
            "output",
            KnotKind::signal_out(output_path, SignalDomain::Count),
        )
        .unwrap();
    let emit = builder
        .knot("emit", KnotKind::emit_command(command_name))
        .unwrap();

    let from = builder.output(&input, "out").unwrap();
    let to = builder.input(&delay, "in").unwrap();
    builder.connect(from, to).unwrap();
    let from = builder.output(&delay, "out").unwrap();
    let to = builder.input(&counter, "inc").unwrap();
    builder.connect(from, to).unwrap();
    let from = builder.output(&counter, "count").unwrap();
    let to = builder.input(&output, "in").unwrap();
    builder.connect(from, to).unwrap();
    let from = builder.output(&delay, "out").unwrap();
    let to = builder.input(&emit, "trigger").unwrap();
    builder.connect(from, to).unwrap();
    builder.build().unwrap()
}

fn thread_order_weave(reverse: bool) -> Weave {
    let mut builder = Weave::builder("thread-order").unwrap();
    let a = builder
        .knot("a", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let b = builder
        .knot("b", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let out_a = builder
        .knot("out-a", KnotKind::signal_out("a", SignalDomain::Bool))
        .unwrap();
    let out_b = builder
        .knot("out-b", KnotKind::signal_out("b", SignalDomain::Bool))
        .unwrap();
    let mut pairs = [(a, out_a), (b, out_b)];
    if reverse {
        pairs.reverse();
    }
    for (from_knot, to_knot) in pairs {
        let from = builder.output(&from_knot, "out").unwrap();
        let to = builder.input(&to_knot, "in").unwrap();
        builder.connect(from, to).unwrap();
    }
    builder.build().unwrap()
}

fn small_weave() -> Weave {
    let mut builder = Weave::builder("small-snapshot").unwrap();
    let input = builder
        .knot("input", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let output = builder
        .knot("output", KnotKind::signal_out("value", SignalDomain::Bool))
        .unwrap();
    let from = builder.output(&input, "out").unwrap();
    let to = builder.input(&output, "in").unwrap();
    builder.connect(from, to).unwrap();
    builder.build().unwrap()
}

fn tick(runtime: &mut Runtime, tick: u64, input: Signal) {
    runtime.begin_frame(HostTime { tick });
    let sense = runtime.sense_id("input").unwrap();
    runtime.port_writer().set_sense(sense, input).unwrap();
    runtime.loom();
}

fn outbox_values(runtime: &Runtime) -> (Vec<Signal>, Vec<Signal>, usize) {
    let outbox = runtime.outbox();
    (
        outbox.signals().iter().map(|sample| sample.value).collect(),
        outbox.emits().iter().map(|emit| emit.payload).collect(),
        outbox.dropped_emits(),
    )
}

#[test]
fn fingerprint_golden_value() {
    let runtime = Runtime::bind(
        weave("fingerprint-golden"),
        BindOpts {
            seed: Some(wyrd::Seed(0x1234_5678_9abc_def0)),
            max_emits_per_tick: 3,
            ..BindOpts::default()
        },
    )
    .unwrap();
    #[cfg(feature = "signal-f32")]
    assert_eq!(runtime.runtime_fingerprint(), 0xba41_d82a_0da5_ffc5);
    #[cfg(feature = "signal-i32")]
    assert_eq!(runtime.runtime_fingerprint(), 0x6443_77c6_4014_b512);
}

#[test]
fn fingerprint_covers_bind_policy_and_immutable_graph_identity() {
    let base = Runtime::bind(weave("fingerprint-fields"), BindOpts::default())
        .unwrap()
        .runtime_fingerprint();
    let changed_cap = Runtime::bind(
        weave("fingerprint-fields"),
        BindOpts {
            max_emits_per_tick: 7,
            ..BindOpts::default()
        },
    )
    .unwrap()
    .runtime_fingerprint();
    let changed_seed = Runtime::bind(
        weave("fingerprint-fields"),
        BindOpts {
            seed: Some(wyrd::Seed(9)),
            ..BindOpts::default()
        },
    )
    .unwrap()
    .runtime_fingerprint();
    let changed_path = Runtime::bind(
        weave_named("fingerprint-fields", "other-value", "pulse"),
        BindOpts::default(),
    )
    .unwrap()
    .runtime_fingerprint();
    let changed_command = Runtime::bind(
        weave_named("fingerprint-fields", "value", "other-pulse"),
        BindOpts::default(),
    )
    .unwrap()
    .runtime_fingerprint();

    for changed in [changed_cap, changed_seed, changed_path, changed_command] {
        assert_ne!(base, changed);
    }
    assert_ne!(
        Runtime::bind(thread_order_weave(false), BindOpts::default())
            .unwrap()
            .runtime_fingerprint(),
        Runtime::bind(thread_order_weave(true), BindOpts::default())
            .unwrap()
            .runtime_fingerprint(),
    );
}

#[test]
fn restore_continues_stateful_graph_exactly() {
    let graph = weave("continuation");
    let mut uninterrupted = Runtime::bind(graph.clone(), BindOpts::default()).unwrap();
    tick(&mut uninterrupted, 0, ONE);
    tick(&mut uninterrupted, 1, ZERO);
    let state = uninterrupted.snapshot();

    let mut restored = Runtime::bind(graph, BindOpts::default()).unwrap();
    restored.restore(&state).unwrap();

    for (tick_number, value) in [(2, ZERO), (3, ONE), (4, ZERO), (5, ZERO)] {
        tick(&mut uninterrupted, tick_number, value);
        tick(&mut restored, tick_number, value);
        assert_eq!(outbox_values(&restored), outbox_values(&uninterrupted));
    }
}

#[test]
fn refreshed_snapshot_continues_like_a_fresh_snapshot() {
    let graph = weave("snapshot-refresh");
    let mut source = Runtime::bind(graph.clone(), BindOpts::default()).unwrap();
    tick(&mut source, 0, ONE);
    tick(&mut source, 1, ZERO);
    let fresh = source.snapshot();
    let mut refreshed = Runtime::bind(small_weave(), BindOpts::default())
        .unwrap()
        .snapshot();
    source.snapshot_into(&mut refreshed);

    assert_eq!(fresh.format_version(), refreshed.format_version());
    assert_eq!(fresh.fingerprint(), refreshed.fingerprint());
    let mut from_fresh = Runtime::bind(graph.clone(), BindOpts::default()).unwrap();
    let mut from_refreshed = Runtime::bind(graph, BindOpts::default()).unwrap();
    from_fresh.restore(&fresh).unwrap();
    from_refreshed.restore(&refreshed).unwrap();
    for (tick_number, value) in [(2, ZERO), (3, ONE), (4, ZERO)] {
        tick(&mut from_fresh, tick_number, value);
        tick(&mut from_refreshed, tick_number, value);
        assert_eq!(outbox_values(&from_fresh), outbox_values(&from_refreshed));
    }
}

#[test]
fn refresh_overwrites_incompatible_larger_state_without_stale_tails() {
    let large_graph = weave("snapshot-large");
    let mut large = Runtime::bind(large_graph.clone(), BindOpts::default()).unwrap();
    tick(&mut large, 0, ONE);
    tick(&mut large, 1, ZERO);
    tick(&mut large, 2, ZERO);
    assert!(!large.outbox().emits().is_empty());
    let mut state = large.snapshot();

    let small_graph = small_weave();
    let mut small = Runtime::bind(small_graph.clone(), BindOpts::default()).unwrap();
    tick(&mut small, 9, ONE);
    assert!(small.outbox().emits().is_empty());
    small.snapshot_into(&mut state);

    assert!(matches!(
        large.restore(&state),
        Err(RestoreError::FingerprintMismatch { .. })
    ));
    let mut restored = Runtime::bind(small_graph, BindOpts::default()).unwrap();
    restored.restore(&state).unwrap();
    assert_eq!(outbox_values(&restored), outbox_values(&small));
    assert!(restored.outbox().emits().is_empty());
}

#[test]
fn snapshot_crosses_owner_boundary_but_rebuilds_local_handles() {
    let graph = weave("owners");
    let mut source = Runtime::bind(graph.clone(), BindOpts::default()).unwrap();
    tick(&mut source, 0, ONE);
    tick(&mut source, 1, ZERO);
    let state = source.snapshot();

    let mut destination = Runtime::bind(graph, BindOpts::default()).unwrap();
    destination.restore(&state).unwrap();
    assert_eq!(outbox_values(&destination), outbox_values(&source));
    for sample in destination.outbox().signals() {
        assert_eq!(destination.path_name(sample.path).unwrap(), "value");
    }
    for emit in destination.outbox().emits() {
        assert_eq!(destination.cmd_name(emit.cmd).unwrap(), "pulse");
    }
}

#[test]
fn incompatible_bind_options_reject_without_mutation() {
    let graph = weave("fingerprint");
    let mut source = Runtime::bind(
        graph.clone(),
        BindOpts {
            max_emits_per_tick: 0,
            ..BindOpts::default()
        },
    )
    .unwrap();
    tick(&mut source, 0, ONE);
    let incompatible = source.snapshot();

    let mut target = Runtime::bind(graph, BindOpts::default()).unwrap();
    tick(&mut target, 0, ZERO);
    let before = target.snapshot();
    assert!(matches!(
        target.restore(&incompatible),
        Err(RestoreError::FingerprintMismatch { .. })
    ));
    let after = target.snapshot();
    assert_eq!(before.format_version(), after.format_version());
    assert_eq!(before.fingerprint(), after.fingerprint());
    tick(&mut target, 1, ZERO);
    assert_eq!(outbox_values(&target).0, vec![ZERO]);
}
