use wyrd_core::{KnotKind, NumericPath, PortDir, TimerMode, ONE, ZERO};
use wyrd_graph::{
    validate, validate_report, Budget, BudgetWarning, BuildError, KnotDef, Pattern, PatternDef,
    PatternExportDef, PortRefDef, ThreadDef, ValidationError, Weave, WeaveBuilder, WeaveDef,
};

fn def(knots: Vec<KnotDef>, threads: Vec<ThreadDef>) -> WeaveDef {
    WeaveDef {
        id: "test".into(),
        numeric: NumericPath::compiled(),
        knots,
        threads,
    }
}

fn knot(id: &str, kind: KnotKind) -> KnotDef {
    KnotDef {
        id: id.into(),
        kind,
    }
}

fn thread(from_knot: &str, from_port: &str, to_knot: &str, to_port: &str) -> ThreadDef {
    ThreadDef {
        from: PortRefDef::new(from_knot, from_port),
        to: PortRefDef::new(to_knot, to_port),
    }
}

#[test]
fn validated_weave_has_read_only_accessors_and_definition_round_trip() {
    let weave = Weave::try_from(def(vec![knot("one", KnotKind::constant(ONE))], vec![])).unwrap();
    assert_eq!(weave.id(), "test");
    assert_eq!(weave.numeric(), NumericPath::compiled());
    assert_eq!(weave.knots()[0].id, "one");
    assert!(weave.threads().is_empty());
    assert_eq!(Weave::try_from(weave.to_def()).unwrap(), weave);
}

#[test]
fn definition_rejects_empty_duplicate_and_numeric_mismatch() {
    assert!(matches!(
        Weave::try_from(def(vec![], vec![])),
        Err(ValidationError::EmptyWeave { .. })
    ));
    let duplicate = def(
        vec![
            knot("x", KnotKind::constant(ONE)),
            knot("x", KnotKind::constant(ZERO)),
        ],
        vec![],
    );
    assert!(
        matches!(Weave::try_from(duplicate), Err(ValidationError::DuplicateKnotId { knot_id }) if knot_id == "x")
    );
    let mut wrong = def(vec![knot("x", KnotKind::constant(ONE))], vec![]);
    wrong.numeric = match NumericPath::compiled() {
        NumericPath::F32 => NumericPath::I32Q16,
        NumericPath::I32Q16 => NumericPath::F32,
    };
    assert!(matches!(
        Weave::try_from(wrong),
        Err(ValidationError::NumericMismatch { .. })
    ));
}

#[test]
fn definition_rejects_empty_author_ids() {
    let mut empty_weave = def(vec![knot("x", KnotKind::constant(ONE))], vec![]);
    empty_weave.id.clear();
    assert!(matches!(
        Weave::try_from(empty_weave),
        Err(ValidationError::InvalidWeaveId { .. })
    ));
    assert!(matches!(
        Weave::try_from(def(vec![knot("", KnotKind::constant(ONE))], vec![])),
        Err(ValidationError::InvalidKnotId { .. })
    ));
}

#[test]
fn definition_reports_unknown_knots_ports_and_direction() {
    let unknown_knot = def(
        vec![knot("x", KnotKind::constant(ONE))],
        vec![thread("missing", "out", "x", "out")],
    );
    assert!(
        matches!(Weave::try_from(unknown_knot), Err(ValidationError::UnknownKnot { knot_id }) if knot_id == "missing")
    );

    let unknown_port = def(
        vec![
            knot("x", KnotKind::constant(ONE)),
            knot("n", KnotKind::not()),
        ],
        vec![thread("x", "bad", "n", "in")],
    );
    assert!(
        matches!(Weave::try_from(unknown_port), Err(ValidationError::UnknownPort { port, .. }) if port == "bad")
    );

    let reversed = def(
        vec![
            knot("a", KnotKind::signal_in()),
            knot("b", KnotKind::signal_in()),
        ],
        vec![thread("a", "out", "b", "out")],
    );
    assert!(matches!(
        Weave::try_from(reversed),
        Err(ValidationError::WrongPortDirection {
            expected: PortDir::In,
            ..
        })
    ));
}

#[test]
fn definition_rejects_fan_in_missing_required_and_cycles() {
    let fan_in = def(
        vec![
            knot("a", KnotKind::constant(ONE)),
            knot("b", KnotKind::constant(ZERO)),
            knot("n", KnotKind::not()),
        ],
        vec![thread("a", "out", "n", "in"), thread("b", "out", "n", "in")],
    );
    assert!(
        matches!(Weave::try_from(fan_in), Err(ValidationError::FanIn { knot_id, port }) if knot_id == "n" && port == "in")
    );
    assert!(matches!(
        Weave::try_from(def(vec![knot("n", KnotKind::not())], vec![])),
        Err(ValidationError::UnconnectedRequired { .. })
    ));

    let cycle = def(
        vec![knot("a", KnotKind::not()), knot("b", KnotKind::not())],
        vec![thread("a", "out", "b", "in"), thread("b", "out", "a", "in")],
    );
    assert!(matches!(
        Weave::try_from(cycle),
        Err(ValidationError::Cycle { .. })
    ));
}

#[test]
fn definition_rejects_invalid_catalog_parameters() {
    let cases = [
        KnotKind::Digitize {
            steps: 0,
            in_min: ZERO,
            in_max: ONE,
            out_min: ZERO,
            out_max: ONE,
        },
        KnotKind::Map {
            in_min: ONE,
            in_max: ZERO,
            out_min: ZERO,
            out_max: ONE,
        },
        KnotKind::Clamp {
            min: ONE,
            max: ZERO,
        },
        KnotKind::Threshold {
            high: ZERO,
            low: ONE,
            use_hysteresis: true,
        },
        KnotKind::And { arity: 5 },
    ];
    for kind in cases {
        assert!(matches!(
            Weave::try_from(def(vec![knot("bad", kind)], vec![])),
            Err(ValidationError::InvalidParameter { .. })
        ));
    }
}

#[cfg(feature = "signal-f32")]
#[test]
fn definition_rejects_every_non_finite_signal_parameter() {
    let bad = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
    for value in bad {
        let kinds = [
            KnotKind::Constant { value },
            KnotKind::Clamp {
                min: value,
                max: ONE,
            },
            KnotKind::Clamp {
                min: ZERO,
                max: value,
            },
            KnotKind::Map {
                in_min: value,
                in_max: ONE,
                out_min: ZERO,
                out_max: ONE,
            },
            KnotKind::Map {
                in_min: ZERO,
                in_max: value,
                out_min: ZERO,
                out_max: ONE,
            },
            KnotKind::Map {
                in_min: ZERO,
                in_max: ONE,
                out_min: value,
                out_max: ONE,
            },
            KnotKind::Map {
                in_min: ZERO,
                in_max: ONE,
                out_min: ZERO,
                out_max: value,
            },
            KnotKind::Digitize {
                steps: 2,
                in_min: value,
                in_max: ONE,
                out_min: ZERO,
                out_max: ONE,
            },
            KnotKind::Digitize {
                steps: 2,
                in_min: ZERO,
                in_max: value,
                out_min: ZERO,
                out_max: ONE,
            },
            KnotKind::Digitize {
                steps: 2,
                in_min: ZERO,
                in_max: ONE,
                out_min: value,
                out_max: ONE,
            },
            KnotKind::Digitize {
                steps: 2,
                in_min: ZERO,
                in_max: ONE,
                out_min: ZERO,
                out_max: value,
            },
            KnotKind::Threshold {
                high: value,
                low: ZERO,
                use_hysteresis: false,
            },
            KnotKind::Threshold {
                high: ONE,
                low: value,
                use_hysteresis: false,
            },
        ];
        for kind in kinds {
            assert!(matches!(
                Weave::try_from(def(vec![knot("bad", kind)], vec![])),
                Err(ValidationError::InvalidParameter {
                    reason: "must be finite",
                    ..
                })
            ));
        }
    }
}

#[test]
fn typed_builder_checks_direction_port_and_owner_immediately() {
    let mut first = WeaveBuilder::new("first").unwrap();
    let source = first.knot("source", KnotKind::signal_in()).unwrap();
    let sink = first.knot("sink", KnotKind::signal_out("x")).unwrap();
    assert!(matches!(
        first.input(&source, "out"),
        Err(BuildError::WrongPortDirection { .. })
    ));
    assert!(matches!(
        first.output(&source, "missing"),
        Err(BuildError::UnknownPort { .. })
    ));

    let mut second = WeaveBuilder::new("second").unwrap();
    let other = second.knot("other", KnotKind::signal_in()).unwrap();
    assert_eq!(first.output(&other, "out"), Err(BuildError::ForeignHandle));
    let foreign = second.output(&other, "out").unwrap();
    let input = first.input(&sink, "in").unwrap();
    assert_eq!(
        first.connect(foreign, input).map(|_| ()),
        Err(BuildError::ForeignHandle)
    );
}

#[test]
fn typed_builder_constructs_a_valid_graph() {
    let mut builder = WeaveBuilder::new("door").unwrap();
    let a = builder.knot("a", KnotKind::signal_in()).unwrap();
    let b = builder.knot("b", KnotKind::signal_in()).unwrap();
    let both = builder.knot("both", KnotKind::and2()).unwrap();
    let door = builder
        .knot("door", KnotKind::signal_out("door.open"))
        .unwrap();
    builder
        .connect(
            builder.output(&a, "out").unwrap(),
            builder.input(&both, "in_0").unwrap(),
        )
        .unwrap();
    builder
        .connect(
            builder.output(&b, "out").unwrap(),
            builder.input(&both, "in_1").unwrap(),
        )
        .unwrap();
    builder
        .connect(
            builder.output(&both, "out").unwrap(),
            builder.input(&door, "in").unwrap(),
        )
        .unwrap();
    let weave = builder.build().unwrap();
    assert_eq!(weave.knots().len(), 4);
    assert_eq!(weave.threads().len(), 3);
}

fn monostable() -> Pattern {
    Pattern::try_from(PatternDef {
        id: "mono".into(),
        inner: def(
            vec![
                knot("edge", KnotKind::rising_from_zero()),
                knot("timer", KnotKind::timer(TimerMode::PulseHold, 2)),
            ],
            vec![thread("edge", "out", "timer", "start")],
        ),
        inputs: vec![PatternExportDef::new("start", "edge", "in")],
        outputs: vec![PatternExportDef::new("active", "timer", "active")],
    })
    .unwrap()
}

#[test]
fn patterns_validate_exports_and_connect_end_to_end() {
    let pattern = monostable();
    assert_eq!(pattern.id(), "mono");
    let mut builder = WeaveBuilder::new("pattern-host").unwrap();
    let trigger = builder.knot("trigger", KnotKind::signal_in()).unwrap();
    let sink = builder
        .knot("sink", KnotKind::signal_out("active"))
        .unwrap();
    let first = builder.include("first", &pattern).unwrap();
    let second = builder.include("second", &pattern).unwrap();
    builder
        .connect(
            builder.output(&trigger, "out").unwrap(),
            first.input("start").unwrap(),
        )
        .unwrap();
    builder
        .connect(
            first.output("active").unwrap(),
            second.input("start").unwrap(),
        )
        .unwrap();
    builder
        .connect(
            second.output("active").unwrap(),
            builder.input(&sink, "in").unwrap(),
        )
        .unwrap();
    let weave = builder.build().unwrap();
    assert!(weave.knots().iter().any(|knot| knot.id == "first/edge"));
    assert!(weave.knots().iter().any(|knot| knot.id == "second/timer"));
}

#[test]
fn pattern_validation_is_contextual() {
    let mut pattern = monostable().to_def();
    pattern
        .inputs
        .push(PatternExportDef::new("start", "edge", "in"));
    assert!(
        matches!(Pattern::try_from(pattern), Err(ValidationError::DuplicateExport { export }) if export == "start")
    );
    let mut pattern = monostable().to_def();
    pattern.outputs[0] = PatternExportDef::new("active", "edge", "in");
    assert!(matches!(
        Pattern::try_from(pattern),
        Err(ValidationError::WrongPortDirection { .. })
    ));
}

#[test]
fn pattern_rejects_duplicate_physical_input_exports() {
    let mut pattern = monostable().to_def();
    pattern
        .inputs
        .push(PatternExportDef::new("also_start", "edge", "in"));
    assert!(matches!(
        Pattern::try_from(pattern),
        Err(ValidationError::DuplicatePatternInput {
            knot_id,
            port,
            first_export,
            duplicate_export,
        }) if knot_id == "edge" && port == "in" && first_export == "start" && duplicate_export == "also_start"
    ));
}

#[test]
fn pattern_rejects_export_of_internally_connected_input() {
    let mut pattern = monostable().to_def();
    pattern
        .inner
        .knots
        .push(knot("source", KnotKind::constant(ONE)));
    pattern
        .inner
        .threads
        .push(thread("source", "out", "edge", "in"));
    assert!(matches!(
        Pattern::try_from(pattern),
        Err(ValidationError::PatternInputAlreadyConnected {
            export,
            knot_id,
            port,
        }) if export == "start" && knot_id == "edge" && port == "in"
    ));
}

#[test]
fn budgets_are_separate_from_structural_validation() {
    let weave = Weave::try_from(def(
        vec![
            knot("a", KnotKind::constant(ONE)),
            knot("b", KnotKind::not()),
            knot("c", KnotKind::not()),
        ],
        vec![thread("a", "out", "b", "in"), thread("b", "out", "c", "in")],
    ))
    .unwrap();
    let tight = Budget {
        max_chain_depth: 1,
        ..Budget::default()
    };
    assert!(matches!(
        validate(&weave, &tight),
        Err(ValidationError::BudgetExceeded {
            metric: "chain depth",
            actual: 2,
            limit: 1,
            ..
        })
    ));
    let soft = Budget {
        soft_knots: 1,
        soft_chain_depth: 0,
        ..Budget::default()
    };
    let report = validate_report(&weave, &soft).unwrap();
    assert!(report
        .warnings
        .iter()
        .any(|w| matches!(w, BudgetWarning::SoftKnots { count: 3, soft: 1 })));
    assert!(report
        .warnings
        .iter()
        .any(|w| matches!(w, BudgetWarning::SoftChainDepth { .. })));
}

#[cfg(feature = "serde-json")]
#[test]
fn json_codec_preserves_parse_context_and_validates() {
    let weave = Weave::try_from(def(vec![knot("one", KnotKind::constant(ONE))], vec![])).unwrap();
    let text = wyrd_graph::to_json(&weave).unwrap();
    assert_eq!(wyrd_graph::from_json(&text).unwrap(), weave);
    match wyrd_graph::from_json("{ bad") {
        Err(wyrd_graph::JsonCodecError::Parse { line, column, .. }) => {
            assert_eq!(line, 1);
            assert!(column > 0);
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[cfg(feature = "serde-ron")]
#[test]
fn ron_codec_preserves_parse_context_and_validates() {
    let weave = Weave::try_from(def(vec![knot("one", KnotKind::constant(ONE))], vec![])).unwrap();
    let text = wyrd_graph::to_ron(&weave).unwrap();
    assert_eq!(wyrd_graph::from_ron(&text).unwrap(), weave);
    match wyrd_graph::from_ron("(bad:") {
        Err(wyrd_graph::RonCodecError::Parse { line, column, .. }) => {
            assert_eq!(line, 1);
            assert!(column > 0);
        }
        other => panic!("unexpected result: {other:?}"),
    }
}
