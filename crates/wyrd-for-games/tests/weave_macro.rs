use wyrd::{
    pattern, weave, BuildError, KnotDef, KnotKind, NumericPath, Pattern, PatternDef,
    PatternExportDef, PortRefDef, SignalDomain, TimerMode, ValidationError, Weave, WeaveBuilder,
    WeaveDef,
};

fn edge_pattern() -> Pattern {
    Pattern::try_from(PatternDef {
        id: "edge".into(),
        inner: WeaveDef {
            id: "edge.inner".into(),
            numeric: NumericPath::compiled(),
            knots: vec![KnotDef {
                id: "edge".into(),
                kind: KnotKind::rising_from_zero(),
            }],
            threads: vec![],
        },
        inputs: vec![PatternExportDef::new("start", "edge", "in")],
        outputs: vec![PatternExportDef::new("active", "edge", "out")],
    })
    .unwrap()
}

#[test]
fn pattern_macro_matches_typed_definition_and_preserves_aliases() {
    let from_macro = pattern! {
        id: "pulse";
        numeric: NumericPath::compiled();
        knots {
            edge as "edge.detect" = KnotKind::rising_from_zero();
            timer = KnotKind::timer(TimerMode::PulseHold, 2);
        }
        exports {
            input start = edge.in;
            output active = timer.active;
        }
        threads {
            edge.out -> timer.start;
        }
    }
    .unwrap();

    let expected = Pattern::try_from(PatternDef {
        id: "pulse".into(),
        inner: WeaveDef {
            id: "pulse.inner".into(),
            numeric: NumericPath::compiled(),
            knots: vec![
                KnotDef {
                    id: "edge.detect".into(),
                    kind: KnotKind::rising_from_zero(),
                },
                KnotDef {
                    id: "timer".into(),
                    kind: KnotKind::timer(TimerMode::PulseHold, 2),
                },
            ],
            threads: vec![wyrd::ThreadDef {
                from: PortRefDef::new("edge.detect", "out"),
                to: PortRefDef::new("timer", "start"),
            }],
        },
        inputs: vec![PatternExportDef::new("start", "edge.detect", "in")],
        outputs: vec![PatternExportDef::new("active", "timer", "active")],
    })
    .unwrap();

    assert_eq!(from_macro, expected);
}

#[test]
fn pattern_macro_expressions_are_evaluated_once_in_source_order() -> Result<(), BuildError> {
    use std::cell::RefCell;

    let seen = RefCell::new(Vec::new());
    let pattern: Pattern = pattern! {
        id: { seen.borrow_mut().push("id"); "ordered" };
        numeric: { seen.borrow_mut().push("numeric"); NumericPath::compiled() };
        knots {
            edge = { seen.borrow_mut().push("edge-kind"); KnotKind::rising_from_zero() };
            timer = { seen.borrow_mut().push("timer-kind"); KnotKind::timer(TimerMode::PulseHold, 2) };
        }
        exports {
            input start = edge.in;
            output active = timer.active;
        }
        threads {
            edge.out -> timer.start;
        }
    }?;

    assert_eq!(pattern.id(), "ordered");
    assert_eq!(
        seen.into_inner(),
        vec!["id", "numeric", "edge-kind", "timer-kind"]
    );
    Ok(())
}

#[test]
fn pattern_macro_invalid_and_duplicate_exports_are_contextual() {
    let duplicate_name = pattern! {
        id: "duplicate-name";
        knots {
            edge = KnotKind::rising_from_zero();
            timer = KnotKind::timer(TimerMode::PulseHold, 2);
        }
        exports {
            input start = edge.in;
            input start = timer.start;
            output active = timer.active;
        }
        threads {}
    }
    .unwrap_err();
    assert!(matches!(
        duplicate_name,
        BuildError::Validation(ValidationError::DuplicateExport { export }) if export == "start"
    ));

    let duplicate_input = pattern! {
        id: "duplicate-input";
        knots { edge = KnotKind::rising_from_zero(); }
        exports {
            input start = edge.in;
            input again = edge.in;
            output active = edge.out;
        }
        threads {}
    }
    .unwrap_err();
    assert!(matches!(
        duplicate_input,
        BuildError::Validation(ValidationError::DuplicatePatternInput { .. })
    ));

    let wrong_direction = pattern! {
        id: "wrong-direction";
        knots { edge = KnotKind::rising_from_zero(); }
        exports { output not_an_output = edge.in; }
        threads {}
    }
    .unwrap_err();
    assert!(matches!(
        wrong_direction,
        BuildError::Validation(ValidationError::WrongPortDirection { .. })
    ));
}

#[test]
fn static_macro_matches_typed_builder() {
    let from_macro = weave! {
        id: "door";
        numeric: NumericPath::compiled();
        knots {
            plate_a = KnotKind::signal_in(SignalDomain::Bool);
            plate_b = KnotKind::signal_in(SignalDomain::Bool);
            both = KnotKind::and2();
            door as "door.output" = KnotKind::signal_out("door.open", SignalDomain::Bool);
        }
        threads {
            plate_a.out -> both.in_0;
            plate_b.out -> both.in_1;
            both.out -> door.in;
        }
    }
    .unwrap();

    let mut builder = WeaveBuilder::new("door").unwrap();
    builder.set_numeric(NumericPath::compiled()).unwrap();
    let plate_a = builder
        .knot("plate_a", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let plate_b = builder
        .knot("plate_b", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let both = builder.knot("both", KnotKind::and2()).unwrap();
    let door = builder
        .knot(
            "door.output",
            KnotKind::signal_out("door.open", SignalDomain::Bool),
        )
        .unwrap();

    let from = builder.output(&plate_a, "out").unwrap();
    let to = builder.input(&both, "in_0").unwrap();
    builder.connect(from, to).unwrap();
    let from = builder.output(&plate_b, "out").unwrap();
    let to = builder.input(&both, "in_1").unwrap();
    builder.connect(from, to).unwrap();
    let from = builder.output(&both, "out").unwrap();
    let to = builder.input(&door, "in").unwrap();
    builder.connect(from, to).unwrap();

    assert_eq!(from_macro, builder.build().unwrap());
}

#[test]
fn patterns_support_every_endpoint_combination() {
    let edge = edge_pattern();
    let graph = weave! {
        id: "pattern-chain";
        knots {
            source = KnotKind::signal_in(SignalDomain::Bool);
            invert = KnotKind::not();
            sink = KnotKind::signal_out("active", SignalDomain::Bool);
        }
        patterns {
            first = ("edge-1", &edge);
            second = ("edge-2", &edge);
        }
        threads {
            source.out -> invert.in;
            invert.out -> first.in("start");
            first.out("active") -> second.in("start");
            second.out("active") -> sink.in;
        }
    }
    .unwrap();

    assert_eq!(graph.knots().len(), 5);
    assert_eq!(graph.threads().len(), 4);
    assert!(graph.knots().iter().any(|knot| knot.id == "edge-1/edge"));
    assert!(graph.knots().iter().any(|knot| knot.id == "edge-2/edge"));
}

#[test]
fn expressions_are_evaluated_once_in_source_order() -> Result<(), BuildError> {
    use std::cell::RefCell;

    let edge = edge_pattern();
    let seen = RefCell::new(Vec::new());
    let graph: Weave = weave! {
        id: { seen.borrow_mut().push("id"); "ordered" };
        numeric: { seen.borrow_mut().push("numeric"); NumericPath::compiled() };
        knots {
            source as "source.alias" = { seen.borrow_mut().push("source-kind"); KnotKind::signal_in(SignalDomain::Bool) };
            sink = { seen.borrow_mut().push("sink-kind"); KnotKind::signal_out("out", SignalDomain::Bool) };
        }
        patterns {
            pulse = (
                { seen.borrow_mut().push("instance-id"); "pulse-1" },
                { seen.borrow_mut().push("pattern"); &edge }
            );
        }
        threads {
            source.out -> pulse.in({ seen.borrow_mut().push("input-export"); "start" });
            pulse.out({ seen.borrow_mut().push("output-export"); "active" }) -> sink.in;
        }
    }?;

    assert_eq!(graph.id(), "ordered");
    assert_eq!(
        seen.into_inner(),
        vec![
            "id",
            "numeric",
            "source-kind",
            "sink-kind",
            "instance-id",
            "pattern",
            "input-export",
            "output-export",
        ]
    );
    Ok(())
}

#[test]
fn invalid_dynamic_export_is_contextual_build_error() {
    let edge = edge_pattern();
    let error = weave! {
        id: "bad-export";
        knots {
            source = KnotKind::signal_in(SignalDomain::Bool);
        }
        patterns {
            pulse = ("pulse", &edge);
        }
        threads {
            source.out -> pulse.in("missing");
        }
    }
    .unwrap_err();

    assert!(matches!(error, BuildError::UnknownExport { .. }));
    assert!(error.to_string().contains("missing"));
}

#[test]
fn alias_changes_author_id() {
    let graph = weave! {
        id: "alias";
        knots {
            source = KnotKind::signal_in(SignalDomain::Bool);
            sink as "path.sink" = KnotKind::signal_out("out", SignalDomain::Bool);
        }
        threads {
            source.out -> sink.in;
        }
    }
    .unwrap();

    assert_eq!(graph.threads()[0].to, PortRefDef::new("path.sink", "in"));
}

#[test]
fn duplicate_authored_ids_are_build_errors() {
    let error = weave! {
        id: "duplicate-alias";
        knots {
            left as "same.id" = KnotKind::signal_in(SignalDomain::Bool);
            right as "same.id" = KnotKind::signal_in(SignalDomain::Bool);
        }
        threads {}
    }
    .unwrap_err();

    assert!(matches!(
        error,
        BuildError::DuplicateKnotId { knot_id } if knot_id == "same.id"
    ));
}
