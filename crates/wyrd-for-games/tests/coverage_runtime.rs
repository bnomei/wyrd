//! Runtime binding, interning, and diagnostic behavior not covered by the
//! per-rune loom catalog tests.

use wyrd::graph::KnotHandle;
use wyrd::{
    from_count, from_level, BindError, BindOpts, Budget, CalcOp, HandleError, HostTime, KnotKind,
    PortSlot, Runtime, SignalDomain, TimerMode, ValidationError, Weave, ONE, ZERO,
};

fn connect(
    builder: &mut wyrd::WeaveBuilder,
    from: &KnotHandle,
    output: &str,
    to: &KnotHandle,
    input: &str,
) {
    let output = builder.output(from, output).expect("known output port");
    let input = builder.input(to, input).expect("known input port");
    builder
        .connect(output, input)
        .expect("domain-compatible edge");
}

fn runtime_with_duplicate_interns() -> Runtime {
    let mut builder = Weave::builder("runtime-coverage").expect("valid weave id");
    let level = builder
        .knot("level", KnotKind::signal_in(SignalDomain::Level))
        .expect("unique knot");
    let trigger = builder
        .knot("trigger", KnotKind::signal_in(SignalDomain::Bool))
        .expect("unique knot");
    let random_min = builder
        .knot(
            "random_min",
            KnotKind::constant(from_level(-1.0), SignalDomain::Level),
        )
        .expect("unique knot");
    let random_max = builder
        .knot(
            "random_max",
            KnotKind::constant(from_level(1.0), SignalDomain::Level),
        )
        .expect("unique knot");
    let div_level_a = builder
        .knot(
            "div_level_a",
            KnotKind::constant(from_level(4.0), SignalDomain::Level),
        )
        .expect("unique knot");
    let div_level_b = builder
        .knot(
            "div_level_b",
            KnotKind::constant(from_level(2.0), SignalDomain::Level),
        )
        .expect("unique knot");
    let div_count_a = builder
        .knot(
            "div_count_a",
            KnotKind::constant(from_count(4), SignalDomain::Count),
        )
        .expect("unique knot");
    let div_count_b = builder
        .knot(
            "div_count_b",
            KnotKind::constant(from_count(2), SignalDomain::Count),
        )
        .expect("unique knot");

    let map_flat = builder
        .knot(
            "map_flat",
            KnotKind::map(
                ZERO,
                ZERO,
                from_level(3.0),
                from_level(3.0),
                SignalDomain::Level,
            ),
        )
        .expect("unique knot");
    let map_span = builder
        .knot(
            "map_span",
            KnotKind::map(ZERO, ONE, ZERO, ONE, SignalDomain::Level),
        )
        .expect("unique knot");
    let digitize_flat = builder
        .knot(
            "digitize_flat",
            KnotKind::Digitize {
                domain: SignalDomain::Level,
                steps: 1,
                in_min: ZERO,
                in_max: ONE,
                out_min: ZERO,
                out_max: ONE,
            },
        )
        .expect("unique knot");
    let digitize_span = builder
        .knot("digitize_span", KnotKind::digitize(2, SignalDomain::Level))
        .expect("unique knot");
    let random_level = builder
        .knot("random_level", KnotKind::random(true, SignalDomain::Level))
        .expect("unique knot");
    let _random_count = builder
        .knot("random_count", KnotKind::random(false, SignalDomain::Count))
        .expect("unique knot");
    let divide_level = builder
        .knot(
            "divide_level",
            KnotKind::calc(CalcOp::Div, SignalDomain::Level),
        )
        .expect("unique knot");
    let divide_count = builder
        .knot(
            "divide_count",
            KnotKind::calc(CalcOp::Div, SignalDomain::Count),
        )
        .expect("unique knot");
    let out_a = builder
        .knot(
            "out_a",
            KnotKind::signal_out("shared.level", SignalDomain::Level),
        )
        .expect("unique knot");
    let out_b = builder
        .knot(
            "out_b",
            KnotKind::signal_out("shared.level", SignalDomain::Level),
        )
        .expect("unique knot");
    let emit_a = builder
        .knot("emit_a", KnotKind::emit_command("shared.command"))
        .expect("unique knot");
    let emit_b = builder
        .knot("emit_b", KnotKind::emit_command("shared.command"))
        .expect("unique knot");

    connect(&mut builder, &level, "out", &map_flat, "in");
    connect(&mut builder, &level, "out", &map_span, "in");
    connect(&mut builder, &level, "out", &digitize_flat, "in");
    connect(&mut builder, &level, "out", &digitize_span, "in");
    connect(&mut builder, &random_min, "out", &random_level, "min");
    connect(&mut builder, &random_max, "out", &random_level, "max");
    connect(&mut builder, &trigger, "out", &random_level, "gate");
    connect(&mut builder, &div_level_a, "out", &divide_level, "a");
    connect(&mut builder, &div_level_b, "out", &divide_level, "b");
    connect(&mut builder, &div_count_a, "out", &divide_count, "a");
    connect(&mut builder, &div_count_b, "out", &divide_count, "b");
    connect(&mut builder, &level, "out", &out_a, "in");
    connect(&mut builder, &level, "out", &out_b, "in");
    connect(&mut builder, &trigger, "out", &emit_a, "trigger");
    connect(&mut builder, &trigger, "out", &emit_a, "enable");
    connect(&mut builder, &level, "out", &emit_a, "payload");
    connect(&mut builder, &trigger, "out", &emit_b, "trigger");
    connect(&mut builder, &trigger, "out", &emit_b, "enable");
    connect(&mut builder, &level, "out", &emit_b, "payload");

    Runtime::bind(builder.build().expect("valid graph"), BindOpts::default())
        .expect("runtime binds")
}

#[test]
fn bind_precomputes_variant_plans_and_reuses_interned_ids() {
    let mut runtime = runtime_with_duplicate_interns();

    assert_eq!(runtime.kind_tag_count(), 20);
    assert_eq!(runtime.inbound_edge_count(), 19);
    assert_eq!(runtime.clear_port_index_count(), 3);
    assert_eq!(runtime.delay_buf_len(), 0);
    assert_eq!(runtime.outbox_signals_capacity(), 2);
    assert_eq!(runtime.sense_id("constant"), None);
    assert_eq!(runtime.sense_id("missing"), None);
    assert_eq!(runtime.knot_id("missing"), None);
    assert_eq!(runtime.path_id("missing"), None);
    assert_eq!(runtime.cmd_id("missing"), None);

    let level = runtime.sense_id("level").expect("level sense");
    let path = runtime.path_id("shared.level").expect("interned path");
    let command = runtime.cmd_id("shared.command").expect("interned command");
    assert_eq!(runtime.path_name(path), Ok("shared.level"));
    assert_eq!(runtime.cmd_name(command), Ok("shared.command"));

    let trigger = runtime.sense_id("trigger").expect("trigger sense");
    runtime.begin_frame(HostTime { tick: 7 });
    let mut writer = runtime.port_writer();
    writer
        .set_sense(level, from_level(0.25))
        .expect("finite level accepts host input");
    writer
        .set_sense(trigger, ONE)
        .expect("boolean trigger accepts ONE");
    runtime.loom();

    let outbox = runtime.outbox();
    assert_eq!(outbox.signals().len(), 2);
    assert!(outbox.signals().iter().all(|sample| sample.path == path));
    assert!(outbox
        .signals()
        .iter()
        .all(|sample| sample.value == from_level(0.25)));
    assert_eq!(outbox.emits().len(), 2);
    assert!(outbox.emits().iter().all(|emit| emit.cmd == command));
    assert!(outbox
        .emits()
        .iter()
        .all(|emit| emit.payload == from_level(0.25)));
    assert_eq!(outbox.dropped_emits(), 0);

    let level_knot = runtime.knot_id("level").expect("knot handle");
    assert_eq!(
        runtime.get_port_checked(level_knot, PortSlot::new(0)),
        Ok(from_level(0.25))
    );
    runtime
        .set_port_checked(level_knot, PortSlot::new(0), from_level(0.5))
        .expect("checked output write");
    assert_eq!(
        runtime.get_port_checked(level_knot, PortSlot::new(0)),
        Ok(from_level(0.5))
    );

    let invalid_get = runtime
        .get_port_checked(level_knot, PortSlot::new(7))
        .expect_err("nonexistent port fails");
    assert_eq!(
        invalid_get,
        HandleError::InvalidPort {
            knot: level_knot,
            port: PortSlot::new(7),
        }
    );
    let invalid_set = runtime
        .set_port_checked(level_knot, PortSlot::new(7), ZERO)
        .expect_err("nonexistent port fails");
    assert_eq!(invalid_set, invalid_get);

    runtime.begin_frame(HostTime { tick: 8 });
    assert!(runtime.outbox().signals().is_empty());
    assert!(runtime.outbox().emits().is_empty());
    assert_eq!(runtime.outbox().dropped_emits(), 0);
}

#[test]
fn both_timer_modes_bind_into_runtime_dispatch() {
    for (id, mode, input) in [
        ("pulse", TimerMode::PulseHold, "start"),
        ("fed", TimerMode::FedCountdown, "feed"),
    ] {
        let mut builder = Weave::builder(id).expect("valid weave id");
        let source = builder
            .knot("source", KnotKind::constant(ONE, SignalDomain::Bool))
            .expect("unique knot");
        let timer = builder
            .knot("timer", KnotKind::timer(mode, 2))
            .expect("unique knot");
        connect(&mut builder, &source, "out", &timer, input);

        let mut runtime = Runtime::bind(builder.build().expect("valid graph"), BindOpts::default())
            .expect("runtime binds");
        runtime.begin_frame(HostTime { tick: 0 });
        runtime.loom();
        assert_eq!(runtime.kind_tag_count(), 2);
    }
}

#[test]
fn bind_wraps_budget_validation_with_the_weave_id() {
    let mut builder = Weave::builder("over-budget").expect("valid weave id");
    builder
        .knot("source", KnotKind::constant(ONE, SignalDomain::Bool))
        .expect("unique knot");
    let weave = builder.build().expect("structurally valid graph");

    let result = Runtime::bind(
        weave,
        BindOpts {
            budget: Budget {
                max_knots: 0,
                ..Budget::default()
            },
            ..BindOpts::default()
        },
    );

    match result {
        Err(BindError::InvalidWeave { weave_id, source }) => {
            assert_eq!(weave_id, "over-budget");
            assert_eq!(
                source,
                ValidationError::BudgetExceeded {
                    metric: "knots",
                    actual: 1,
                    limit: 0,
                    at_knot: None,
                }
            );
        }
        Err(other) => panic!("expected invalid-weave bind error, got {other}"),
        Ok(_) => panic!("over-budget weave unexpectedly bound"),
    }
}
