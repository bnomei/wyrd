//! Regression coverage for loom paths not represented by the smaller catalog tests.

use wyrd::{
    cookbook::helpers::signal_out_value, from_count, from_level, BindOpts, CalcOp, HostTime,
    KnotKind, Runtime, Seed, Signal, SignalDomain, Weave, ONE, ZERO,
};

fn loom_conversion(from: SignalDomain, to: SignalDomain, value: Signal) -> Signal {
    let mut builder = Weave::builder("loom-conversions").unwrap();
    let input = builder.knot("input", KnotKind::signal_in(from)).unwrap();
    let convert = builder
        .knot("convert", KnotKind::convert(from, to))
        .unwrap();
    let output = builder
        .knot("output", KnotKind::signal_out("converted", to))
        .unwrap();

    let from = builder.output(&input, "out").unwrap();
    let to = builder.input(&convert, "in").unwrap();
    builder.connect(from, to).unwrap();
    let from = builder.output(&convert, "out").unwrap();
    let to = builder.input(&output, "in").unwrap();
    builder.connect(from, to).unwrap();

    let mut runtime = Runtime::bind(builder.build().unwrap(), BindOpts::default()).unwrap();
    let sense = runtime.sense_id("input").unwrap();
    runtime.begin_frame(HostTime { tick: 0 });
    runtime.port_writer().set_sense(sense, value).unwrap();
    runtime.loom();
    signal_out_value(&runtime, "converted")
}

fn loom_unary_level(operator: KnotKind, value: Signal) -> Signal {
    let mut builder = Weave::builder("loom-unary-level").unwrap();
    let input = builder
        .knot("input", KnotKind::signal_in(SignalDomain::Level))
        .unwrap();
    let operator = builder.knot("operator", operator).unwrap();
    let output = builder
        .knot(
            "output",
            KnotKind::signal_out("result", SignalDomain::Level),
        )
        .unwrap();

    let from = builder.output(&input, "out").unwrap();
    let to = builder.input(&operator, "in").unwrap();
    builder.connect(from, to).unwrap();
    let from = builder.output(&operator, "out").unwrap();
    let to = builder.input(&output, "in").unwrap();
    builder.connect(from, to).unwrap();

    let mut runtime = Runtime::bind(builder.build().unwrap(), BindOpts::default()).unwrap();
    let sense = runtime.sense_id("input").unwrap();
    runtime.begin_frame(HostTime { tick: 0 });
    runtime.port_writer().set_sense(sense, value).unwrap();
    runtime.loom();
    signal_out_value(&runtime, "result")
}

fn loom_level_calc(op: CalcOp, lhs: Signal, rhs: Signal, rhs_is_constant: bool) -> Signal {
    let mut builder = Weave::builder("loom-level-calc").unwrap();
    let left = builder
        .knot("left", KnotKind::signal_in(SignalDomain::Level))
        .unwrap();
    let right = if rhs_is_constant {
        builder
            .knot("right", KnotKind::constant(rhs, SignalDomain::Level))
            .unwrap()
    } else {
        builder
            .knot("right", KnotKind::signal_in(SignalDomain::Level))
            .unwrap()
    };
    let calc = builder
        .knot("calc", KnotKind::calc(op, SignalDomain::Level))
        .unwrap();
    let output = builder
        .knot(
            "output",
            KnotKind::signal_out("result", SignalDomain::Level),
        )
        .unwrap();

    let from = builder.output(&left, "out").unwrap();
    let to = builder.input(&calc, "a").unwrap();
    builder.connect(from, to).unwrap();
    let from = builder.output(&right, "out").unwrap();
    let to = builder.input(&calc, "b").unwrap();
    builder.connect(from, to).unwrap();
    let from = builder.output(&calc, "out").unwrap();
    let to = builder.input(&output, "in").unwrap();
    builder.connect(from, to).unwrap();

    let mut runtime = Runtime::bind(builder.build().unwrap(), BindOpts::default()).unwrap();
    let left = runtime.sense_id("left").unwrap();
    let right = (!rhs_is_constant).then(|| runtime.sense_id("right").unwrap());
    runtime.begin_frame(HostTime { tick: 0 });
    {
        let mut writer = runtime.port_writer();
        writer.set_sense(left, lhs).unwrap();
        if let Some(right) = right {
            writer.set_sense(right, rhs).unwrap();
        }
    }
    runtime.loom();
    signal_out_value(&runtime, "result")
}

fn loom_random_count(equal_bounds: Option<Signal>) -> Signal {
    let mut builder = Weave::builder("loom-random-count").unwrap();
    let random = builder
        .knot("random", KnotKind::random(false, SignalDomain::Count))
        .unwrap();
    if let Some(bound) = equal_bounds {
        let minimum = builder
            .knot("minimum", KnotKind::constant(bound, SignalDomain::Count))
            .unwrap();
        let maximum = builder
            .knot("maximum", KnotKind::constant(bound, SignalDomain::Count))
            .unwrap();
        let from = builder.output(&minimum, "out").unwrap();
        let to = builder.input(&random, "min").unwrap();
        builder.connect(from, to).unwrap();
        let from = builder.output(&maximum, "out").unwrap();
        let to = builder.input(&random, "max").unwrap();
        builder.connect(from, to).unwrap();
    }
    let output = builder
        .knot(
            "output",
            KnotKind::signal_out("sample", SignalDomain::Count),
        )
        .unwrap();
    let from = builder.output(&random, "out").unwrap();
    let to = builder.input(&output, "in").unwrap();
    builder.connect(from, to).unwrap();

    let mut runtime = Runtime::bind(
        builder.build().unwrap(),
        BindOpts {
            seed: Some(Seed(7)),
            ..BindOpts::default()
        },
    )
    .unwrap();
    runtime.begin_frame(HostTime { tick: 0 });
    runtime.loom();
    signal_out_value(&runtime, "sample")
}

#[test]
fn loom_converts_every_authorable_domain_pair() {
    assert_eq!(
        loom_conversion(SignalDomain::Bool, SignalDomain::Level, ONE),
        ONE
    );
    assert_eq!(
        loom_conversion(SignalDomain::Bool, SignalDomain::Count, ZERO),
        ZERO
    );
    assert_eq!(
        loom_conversion(SignalDomain::Level, SignalDomain::Bool, from_level(0.5)),
        ONE
    );
    assert_eq!(
        loom_conversion(SignalDomain::Level, SignalDomain::Count, from_level(-2.5)),
        from_count(-3)
    );
    assert_eq!(
        loom_conversion(SignalDomain::Count, SignalDomain::Bool, ZERO),
        ZERO
    );
    assert_eq!(
        loom_conversion(SignalDomain::Count, SignalDomain::Level, from_count(2)),
        from_level(2.0)
    );
}

#[test]
fn loom_stages_four_inbound_values_before_evaluating_and() {
    let mut builder = Weave::builder("loom-four-input-and").unwrap();
    let inputs = [
        builder
            .knot("input_0", KnotKind::signal_in(SignalDomain::Bool))
            .unwrap(),
        builder
            .knot("input_1", KnotKind::signal_in(SignalDomain::Bool))
            .unwrap(),
        builder
            .knot("input_2", KnotKind::signal_in(SignalDomain::Bool))
            .unwrap(),
        builder
            .knot("input_3", KnotKind::signal_in(SignalDomain::Bool))
            .unwrap(),
    ];
    let and = builder.knot("and", KnotKind::And { arity: 4 }).unwrap();
    let output = builder
        .knot(
            "output",
            KnotKind::signal_out("all-present", SignalDomain::Bool),
        )
        .unwrap();

    for (slot, input) in inputs.iter().enumerate() {
        let from = builder.output(input, "out").unwrap();
        let to = builder.input(&and, &format!("in_{slot}")).unwrap();
        builder.connect(from, to).unwrap();
    }
    let from = builder.output(&and, "out").unwrap();
    let to = builder.input(&output, "in").unwrap();
    builder.connect(from, to).unwrap();

    let mut runtime = Runtime::bind(builder.build().unwrap(), BindOpts::default()).unwrap();
    let senses = [
        runtime.sense_id("input_0").unwrap(),
        runtime.sense_id("input_1").unwrap(),
        runtime.sense_id("input_2").unwrap(),
        runtime.sense_id("input_3").unwrap(),
    ];

    runtime.begin_frame(HostTime { tick: 0 });
    {
        let mut writer = runtime.port_writer();
        for sense in senses {
            writer.set_sense(sense, ONE).unwrap();
        }
    }
    runtime.loom();
    assert_eq!(signal_out_value(&runtime, "all-present"), ONE);

    runtime.begin_frame(HostTime { tick: 1 });
    {
        let mut writer = runtime.port_writer();
        writer.set_sense(senses[0], ONE).unwrap();
        writer.set_sense(senses[1], ONE).unwrap();
        writer.set_sense(senses[2], ONE).unwrap();
        writer.set_sense(senses[3], ZERO).unwrap();
    }
    runtime.loom();
    assert_eq!(signal_out_value(&runtime, "all-present"), ZERO);
}

#[test]
fn loom_covers_level_arithmetic_abs_neg_and_sqrt() {
    assert_eq!(
        loom_level_calc(CalcOp::Add, from_level(0.5), from_level(0.25), false),
        from_level(0.75)
    );
    assert_eq!(
        loom_level_calc(CalcOp::Sub, from_level(0.75), from_level(0.25), false),
        from_level(0.5)
    );
    assert_eq!(
        loom_level_calc(CalcOp::Mul, from_level(0.5), from_level(0.5), false),
        from_level(0.25)
    );
    assert_eq!(
        loom_level_calc(CalcOp::Div, from_level(0.5), from_level(0.25), false),
        from_level(2.0)
    );
    assert_eq!(
        loom_level_calc(CalcOp::Div, from_level(0.5), from_level(2.0), true),
        from_level(0.25)
    );
    assert_eq!(
        loom_unary_level(KnotKind::abs(SignalDomain::Level), from_level(-0.5)),
        from_level(0.5)
    );
    assert_eq!(
        loom_unary_level(KnotKind::neg(SignalDomain::Level), from_level(0.5)),
        from_level(-0.5)
    );
    assert_eq!(
        loom_unary_level(KnotKind::sqrt(SignalDomain::Level), from_level(0.25)),
        from_level(0.5)
    );
    assert_eq!(
        loom_unary_level(KnotKind::sqrt(SignalDomain::Level), from_level(-1.0)),
        ZERO
    );
}

#[test]
fn loom_random_count_handles_default_and_equal_bounds() {
    let default_sample = loom_random_count(None);
    assert!(default_sample >= ZERO && default_sample <= from_count(1));
    assert_eq!(loom_random_count(Some(from_count(4))), from_count(4));
}
