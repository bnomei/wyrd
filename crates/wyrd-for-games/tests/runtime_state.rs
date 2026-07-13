//! Runtime snapshot continuation and compatibility contracts.

use wyrd::{
    BindOpts, HostTime, KnotKind, RestoreError, Runtime, Signal, SignalDomain, Weave, ONE, ZERO,
};

fn weave(id: &str) -> Weave {
    let mut builder = Weave::builder(id).unwrap();
    let input = builder
        .knot("input", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let delay = builder.knot("delay", KnotKind::Delay { ticks: 2 }).unwrap();
    let counter = builder.knot("counter", KnotKind::counter()).unwrap();
    let output = builder
        .knot("output", KnotKind::signal_out("value", SignalDomain::Count))
        .unwrap();
    let emit = builder
        .knot("emit", KnotKind::emit_command("pulse"))
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
