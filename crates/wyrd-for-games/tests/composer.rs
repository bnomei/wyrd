use wyrd::{
    from_count, pattern, weave, CalcOp, CompareOp, ComposeError, FlagPriority, KnotKind, Level,
    LevelWire, SignalDomain, TimerMode, Weave, ONE, ZERO,
};

#[test]
fn composer_matches_equivalent_declarative_weave() {
    let declarative = weave! {
        id: "two-plate";
        knots {
            a = KnotKind::signal_in(SignalDomain::Bool);
            b = KnotKind::signal_in(SignalDomain::Bool);
            both = KnotKind::and2();
            out = KnotKind::signal_out("door.open", SignalDomain::Bool);
        }
        threads { a.out -> both.in_0; b.out -> both.in_1; both.out -> out.in; }
    }
    .expect("declarative weave is valid");

    let composed = Weave::compose("two-plate", |composer| {
        let a = composer.bool_input("a")?;
        let b = composer.bool_input("b")?;
        let both = composer.and("both", &a, &b)?;
        composer.signal_out("out", "door.open", &both)
    })
    .expect("typed composer is valid");

    assert_eq!(composed, declarative);
}

#[test]
fn composer_supports_branching_and_cookbook_semantics() {
    let weave = Weave::compose("cooldown", |composer| {
        let button = composer.bool_input("button")?;
        let edge = composer.rising("edge", &button)?;
        let cooling = composer.pulse_hold("hold", 2, &edge)?;
        composer.signal_out("shot", "shot", &edge)?;
        composer.signal_out("cooling", "cooling", &cooling)
    })
    .expect("fan-out is valid");

    assert_eq!(weave.threads().len(), 4);
    assert_eq!(weave.knots().len(), 5);
    assert!(weave
        .threads()
        .iter()
        .any(|thread| thread.from.knot == "edge" && thread.to.knot == "shot"));
    assert!(weave
        .threads()
        .iter()
        .any(|thread| thread.from.knot == "edge" && thread.to.knot == "hold"));
}

#[test]
fn raw_knot_and_thread_escape_hatch_keeps_full_catalog_access() {
    let weave = Weave::compose("raw-catalog", |composer| {
        let source = composer.knot("source", KnotKind::constant_count(2))?;
        let convert = composer.knot(
            "convert",
            KnotKind::convert(SignalDomain::Count, SignalDomain::Level),
        )?;
        let output = composer.knot("output", KnotKind::signal_out("level", SignalDomain::Level))?;
        let source_out = composer.output(&source, "out")?;
        let convert_in = composer.input(&convert, "in")?;
        let convert_out = composer.output(&convert, "out")?;
        let output_in = composer.input(&output, "in")?;
        composer.thread(&source_out, &convert_in)?;
        composer.thread(&convert_out, &output_in)
    })
    .expect("raw catalog composition is valid");

    assert_eq!(
        weave.knots()[1].kind,
        KnotKind::convert(SignalDomain::Count, SignalDomain::Level)
    );
}

#[test]
fn composer_preserves_structured_builder_and_validation_errors() {
    let duplicate = Weave::compose("duplicate", |composer| {
        composer.bool_constant("same", true)?;
        composer.bool_constant("same", false)?;
        Ok(())
    });
    assert!(matches!(duplicate, Err(ComposeError::Build(_))));

    let invalid = Weave::compose("invalid", |composer| {
        let source = composer.bool_constant("source", true)?;
        composer.signal_out("out", "out", &source)?;
        composer.knot("dangling", KnotKind::not())?;
        Ok(())
    });
    assert!(matches!(invalid, Err(ComposeError::Validation(_))));
}

#[test]
fn composer_helpers_cover_cookbook_operations_and_pattern_inclusion() {
    let pulse = pattern! {
        id: "pulse";
        knots { constant = KnotKind::constant_bool(true); }
        exports { output out = constant.out; }
        threads { }
    }
    .expect("constant pattern is valid");

    let weave = Weave::compose("helpers", |composer| {
        let bool_in = composer.bool_input("bool-in")?;
        let level_in = composer.level_input("level-in")?;
        let count_in = composer.count_input("count-in")?;
        let bool_constant = composer.bool_constant("bool-constant", true)?;
        let level_constant = composer.level_constant("level-constant", 0.5)?;
        let count_constant = composer.count_constant("count-constant", 2)?;
        let started = composer.on_start("started")?;

        let inverted = composer.not("not", &bool_in)?;
        let conjunction = composer.and("and", &inverted, &bool_constant)?;
        let disjunction = composer.or("or", &conjunction, &started)?;
        let exclusive = composer.xor("xor", &disjunction, &bool_constant)?;
        let rising = composer.rising("rising", &exclusive)?;
        let _falling = composer.falling("falling", &exclusive)?;
        let _change = composer.change("change", &exclusive)?;
        let flag = composer.flag(
            "flag",
            FlagPriority::ResetWins,
            Some(&rising),
            Some(&bool_in),
            Some(&bool_constant),
        )?;
        let count = composer.counter(
            "counter",
            Some(&rising),
            Some(&bool_in),
            Some(&bool_constant),
        )?;
        let hold = composer.pulse_hold("hold", 2, &rising)?;
        let fed = composer.fed_countdown("fed", 2, &bool_in)?;
        let _bool_compare = composer.compare("bool-compare", CompareOp::Eq, &flag, &hold)?;
        let _count_compare =
            composer.compare_constant("count-compare", CompareOp::Gte, &count, from_count(1))?;
        let calc = composer.calc("calc", CalcOp::Add, &level_in, &level_constant)?;
        let mapped = composer.map("map", &calc, ZERO, ONE, ZERO, ONE)?;
        let threshold = composer.threshold("threshold", &mapped, ONE, ZERO, false)?;
        let delayed = composer.delay("delay", 1, &mapped)?;
        let absolute = composer.abs("abs", &delayed)?;
        let negated = composer.neg("neg", &absolute)?;
        let rooted = composer.sqrt("sqrt", &negated)?;
        let clamped = composer.clamp("clamp", &rooted, ZERO, ONE)?;
        let digitized = composer.digitize("digitize", &clamped, 2)?;
        let random = composer.random(
            "random",
            false,
            Some(&level_constant),
            Some(&digitized),
            Some(&bool_in),
        )?;
        let selected = composer.select("select", &threshold.out, &random, &level_constant)?;
        let converted: LevelWire = composer.convert("convert", &count_in)?;
        let instance = composer.include("included", &pulse)?;
        let sink = composer.knot(
            "included-sink",
            KnotKind::signal_out("included", SignalDomain::Bool),
        )?;
        let sink_input = composer.input(&sink, "in")?;
        composer.thread(&instance.output("out")?, &sink_input)?;

        composer.signal_out("selected-out", "selected", &selected)?;
        composer.signal_out("count-constant-out", "count-constant", &count_constant)?;
        composer.signal_out("converted-out", "converted", &converted)?;
        composer.signal_out("fed-out", "fed", &fed)?;
        composer.signal_out("threshold-up-out", "threshold-up", &threshold.crossed_up)?;
        composer.signal_out(
            "threshold-down-out",
            "threshold-down",
            &threshold.crossed_down,
        )?;
        composer.emit("emit", "event", &rising)
    })
    .expect("all semantic helpers lower through the builder");

    assert!(weave
        .knots()
        .iter()
        .any(|knot| knot.id == "included/constant"));
    assert!(weave
        .knots()
        .iter()
        .any(|knot| knot.kind == KnotKind::timer(TimerMode::FedCountdown, 2)));
    assert!(weave.threads().len() > 30);
}

#[test]
fn composer_optional_wires_and_count_threshold_are_validated() {
    let weave = Weave::compose("optional-wires", |composer| {
        let count = composer.count_input("count")?;
        let threshold = composer.threshold("threshold", &count, from_count(2), ZERO, true)?;
        let random = composer.random::<Level>("random", false, None, None, None)?;

        composer.signal_out("threshold-out", "threshold", &threshold.out)?;
        composer.signal_out("random-out", "random", &random)
    })
    .expect("optional random ports are valid when its gate is not required");

    assert!(weave.knots().iter().any(|knot| knot.id == "threshold"));
    assert!(weave.knots().iter().any(|knot| knot.id == "random"));
}

#[test]
fn composer_propagates_threshold_build_errors() {
    let error = Weave::compose("duplicate-threshold", |composer| {
        let count = composer.count_input("count")?;
        composer.threshold("threshold", &count, from_count(2), ZERO, false)?;
        composer.threshold("threshold", &count, from_count(2), ZERO, false)?;
        Ok(())
    })
    .expect_err("the second threshold keeps the builder's duplicate-id error");

    assert!(matches!(error, ComposeError::Build(_)));
}

#[test]
fn compose_errors_format_both_sources() {
    let build = Weave::compose("build", |composer| {
        composer.bool_constant("same", true)?;
        composer.bool_constant("same", false)?;
        Ok(())
    })
    .expect_err("duplicate ids are build errors");
    assert!(build.to_string().contains("duplicate"));

    let validation = Weave::compose("validation", |composer| {
        composer.knot("not", KnotKind::not())?;
        Ok(())
    })
    .expect_err("unwired required ports fail final validation");
    assert!(validation.to_string().contains("required input"));
}
