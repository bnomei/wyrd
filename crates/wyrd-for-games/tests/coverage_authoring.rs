//! Behavioral coverage for authoring errors, pattern expansion, and budgets.

use std::error::Error as _;

use wyrd::{
    from_count, slot_of, validate, validate_report, weave, Budget, BuildError, CompareOp, KnotDef,
    KnotKind, NumericPath, Pattern, PatternDef, PatternExportDef, PortDir, PortRefDef,
    SignalDomain, ThreadDef, ValidationError, Weave, WeaveBuilder, WeaveDef, ONE, ZERO,
};

fn def(knots: Vec<KnotDef>, threads: Vec<ThreadDef>) -> WeaveDef {
    WeaveDef {
        id: "authoring-coverage".into(),
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

fn edge_pattern() -> Pattern {
    Pattern::try_from(PatternDef {
        id: "edge".into(),
        inner: def(vec![knot("edge", KnotKind::rising_from_zero())], vec![]),
        inputs: vec![PatternExportDef::new("start", "edge", "in")],
        outputs: vec![PatternExportDef::new("active", "edge", "out")],
    })
    .expect("valid reusable edge pattern")
}

fn constant_pattern() -> Pattern {
    Pattern::try_from(PatternDef {
        id: "constant".into(),
        inner: def(
            vec![knot(
                "constant",
                KnotKind::constant(ONE, SignalDomain::Bool),
            )],
            vec![],
        ),
        inputs: vec![],
        outputs: vec![PatternExportDef::new("out", "constant", "out")],
    })
    .expect("valid reusable constant pattern")
}

fn other_numeric_path() -> NumericPath {
    match NumericPath::compiled() {
        NumericPath::F32 => NumericPath::I32Q16,
        NumericPath::I32Q16 => NumericPath::F32,
    }
}

#[test]
fn builder_reports_invalid_ids_exports_and_pattern_include_conflicts() {
    assert!(matches!(
        WeaveBuilder::new(""),
        Err(BuildError::InvalidId {
            reason: "weave ids must be non-empty",
            ..
        })
    ));

    let mut builder = Weave::builder("builder-errors").expect("valid builder");
    assert!(matches!(
        builder.knot("", KnotKind::not()),
        Err(BuildError::InvalidId {
            reason: "knot ids must be non-empty",
            ..
        })
    ));
    builder
        .knot("source", KnotKind::signal_in(SignalDomain::Bool))
        .expect("first knot is unique");
    assert!(matches!(
        builder.knot("source", KnotKind::signal_in(SignalDomain::Bool)),
        Err(BuildError::DuplicateKnotId { knot_id }) if knot_id == "source"
    ));
    assert!(matches!(
        slot_of(&KnotKind::not(), "missing"),
        Err(BuildError::UnknownPort { knot_id, port, expected })
            if knot_id == "<kind>" && port == "missing" && expected == vec!["in", "out"]
    ));

    let pattern = constant_pattern();
    let instance = builder
        .include("constant", &pattern)
        .expect("first include succeeds");
    assert_eq!(instance.id(), "constant");
    assert!(instance.output("out").is_ok());
    assert!(matches!(
        instance.output("missing"),
        Err(BuildError::UnknownExport {
            instance_id,
            export,
            direction: PortDir::Out,
        }) if instance_id == "constant" && export == "missing"
    ));
    assert!(matches!(
        builder.include("constant", &pattern),
        Err(BuildError::DuplicateKnotId { knot_id }) if knot_id == "constant/constant"
    ));
    assert!(matches!(
        builder.include("", &pattern),
        Err(BuildError::InvalidId {
            reason: "pattern instance ids must be non-empty and contain no slash",
            ..
        })
    ));

    let mut wrong_path = WeaveBuilder::new("wrong-path").expect("valid builder");
    wrong_path
        .set_numeric(other_numeric_path())
        .expect("numeric tag is mutable until build");
    assert!(matches!(
        wrong_path.include("constant", &pattern),
        Err(BuildError::NumericMismatch { .. })
    ));
}

#[test]
fn builder_enforces_knot_capacity_for_direct_and_pattern_additions() {
    let mut builder = WeaveBuilder::new("capacity").expect("valid builder");
    for index in 0..=u16::MAX {
        builder
            .knot(
                format!("knot-{index}"),
                KnotKind::constant(ZERO, SignalDomain::Bool),
            )
            .expect("u16 index remains representable");
    }
    assert!(matches!(
        builder.knot("overflow", KnotKind::constant(ZERO, SignalDomain::Bool)),
        Err(BuildError::RepresentationOverflow {
            what: "knot",
            actual,
            limit,
        }) if actual == usize::from(u16::MAX) + 1 && limit == usize::from(u16::MAX)
    ));
    assert!(matches!(
        builder.include("constant", &constant_pattern()),
        Err(BuildError::RepresentationOverflow {
            what: "knot",
            actual,
            limit,
        }) if actual == usize::from(u16::MAX) + 1 && limit == usize::from(u16::MAX)
    ));
}

#[test]
fn pattern_definition_conversion_accessors_and_invalid_exports_are_contextual() {
    let pattern = edge_pattern();
    assert_eq!(pattern.inputs()[0].name, "start");
    assert_eq!(pattern.outputs()[0].name, "active");
    let definition: PatternDef = pattern.clone().into();
    assert_eq!(definition, pattern.to_def());

    let invalid_id = PatternDef {
        id: "bad/id".into(),
        inner: def(
            vec![knot("constant", KnotKind::constant_bool(true))],
            vec![],
        ),
        inputs: vec![],
        outputs: vec![],
    };
    assert!(matches!(
        Pattern::try_from(invalid_id),
        Err(ValidationError::InvalidPatternId { .. })
    ));
    let inner_slash = PatternDef {
        id: "inner-slash".into(),
        inner: def(vec![knot("bad/id", KnotKind::constant_bool(true))], vec![]),
        inputs: vec![],
        outputs: vec![],
    };
    assert!(matches!(
        Pattern::try_from(inner_slash),
        Err(ValidationError::InvalidKnotId {
            reason: "pattern inner knot ids must contain no slash",
            ..
        })
    ));

    let unknown_knot = PatternDef {
        id: "unknown-knot".into(),
        inner: def(
            vec![knot("constant", KnotKind::constant_bool(true))],
            vec![],
        ),
        inputs: vec![],
        outputs: vec![PatternExportDef::new("out", "missing", "out")],
    };
    assert!(matches!(
        Pattern::try_from(unknown_knot),
        Err(ValidationError::UnknownKnot { knot_id }) if knot_id == "missing"
    ));
    let unknown_port = PatternDef {
        id: "unknown-port".into(),
        inner: def(
            vec![knot("constant", KnotKind::constant_bool(true))],
            vec![],
        ),
        inputs: vec![],
        outputs: vec![PatternExportDef::new("out", "constant", "missing")],
    };
    assert!(matches!(
        Pattern::try_from(unknown_port),
        Err(ValidationError::UnknownPort { port, .. }) if port == "missing"
    ));
    let duplicate_output = PatternDef {
        id: "duplicate-output".into(),
        inner: def(
            vec![knot("constant", KnotKind::constant_bool(true))],
            vec![],
        ),
        inputs: vec![],
        outputs: vec![
            PatternExportDef::new("out", "constant", "out"),
            PatternExportDef::new("out", "constant", "out"),
        ],
    };
    assert!(matches!(
        Pattern::try_from(duplicate_output),
        Err(ValidationError::DuplicateExport { export }) if export == "out"
    ));
}

#[test]
fn weave_definition_consumes_a_validated_weave() {
    let weave = Weave::try_from(def(
        vec![knot("constant", KnotKind::constant_bool(true))],
        vec![],
    ))
    .expect("valid weave");
    let definition: WeaveDef = weave.clone().into();
    assert_eq!(
        Weave::try_from(definition).expect("round-trip validation"),
        weave
    );
}

#[test]
fn definitions_report_unknown_thread_targets_after_validating_sources() {
    assert!(matches!(
        Weave::try_from(def(
            vec![knot(
                "source",
                KnotKind::constant(ONE, SignalDomain::Bool),
            )],
            vec![thread("source", "out", "missing", "in")],
        )),
        Err(ValidationError::UnknownKnot { knot_id }) if knot_id == "missing"
    ));
}

#[test]
fn authoring_macro_expands_the_plain_knot_declaration_form() {
    let graph = weave! {
        id: "plain-macro";
        knots {
            source = KnotKind::signal_in(SignalDomain::Bool);
            sink = KnotKind::signal_out("plain.macro", SignalDomain::Bool);
        }
        threads {
            source.out -> sink.in;
        }
    }
    .expect("macro emits a valid graph");
    assert_eq!(graph.id(), "plain-macro");
}

#[test]
fn budget_reports_all_soft_warning_kinds_and_hard_limits() {
    let weave = Weave::try_from(def(
        vec![
            knot("source", KnotKind::signal_in(SignalDomain::Bool)),
            knot("left", KnotKind::signal_out("left", SignalDomain::Bool)),
            knot("right", KnotKind::signal_out("right", SignalDomain::Bool)),
        ],
        vec![
            thread("source", "out", "left", "in"),
            thread("source", "out", "right", "in"),
        ],
    ))
    .expect("valid fan-out graph");
    let soft = Budget {
        soft_knots: 1,
        soft_threads: 1,
        soft_chain_depth: 0,
        soft_fan_out: 1,
        ..Budget::default()
    };
    let report = validate_report(&weave, &soft).expect("soft limits do not fail validation");
    assert!(!report.ok());
    let message = report.to_string();
    assert!(message.contains("soft knot budget"));
    assert!(message.contains("soft thread budget"));
    assert!(message.contains("soft fan-out"));
    assert!(message.contains("soft chain depth"));
    assert!(message.contains("; "));

    let ok = validate_report(&weave, &Budget::default()).expect("default budget is generous");
    assert!(ok.ok());
    assert_eq!(ok.to_string(), "validate ok");
    assert!(matches!(
        validate(
            &weave,
            &Budget {
                max_fan_out: 1,
                ..Budget::default()
            },
        ),
        Err(ValidationError::BudgetExceeded {
            metric: "fan-out",
            at_knot: Some(knot),
            ..
        }) if knot == "source"
    ));
}

#[test]
fn budget_rejects_excessive_delay_paths() {
    let weave = Weave::try_from(def(
        vec![
            knot("source", KnotKind::signal_in(SignalDomain::Bool)),
            knot("delay", KnotKind::Delay { ticks: 2 }),
            knot("sink", KnotKind::signal_out("delayed", SignalDomain::Bool)),
        ],
        vec![
            thread("source", "out", "delay", "in"),
            thread("delay", "out", "sink", "in"),
        ],
    ))
    .expect("valid delayed graph");
    assert!(matches!(
        validate(
            &weave,
            &Budget {
                max_delay_path_sum: 1,
                ..Budget::default()
            },
        ),
        Err(ValidationError::BudgetExceeded {
            metric: "delay path sum",
            at_knot: Some(knot),
            ..
        }) if knot == "delay"
    ));
}

#[test]
fn definitions_reject_unrepresentable_knot_and_thread_counts() {
    let too_many_knots = def(
        vec![knot("duplicate", KnotKind::constant_bool(true)); usize::from(u16::MAX) + 1],
        vec![],
    );
    assert!(matches!(
        Weave::try_from(too_many_knots),
        Err(ValidationError::RepresentationOverflow {
            what: "knot",
            actual,
            limit,
        }) if actual == usize::from(u16::MAX) + 1 && limit == usize::from(u16::MAX)
    ));

    let too_many_threads = def(
        vec![knot("constant", KnotKind::constant_bool(true))],
        vec![thread("constant", "out", "constant", "out"); usize::from(u16::MAX) + 1],
    );
    assert!(matches!(
        Weave::try_from(too_many_threads),
        Err(ValidationError::RepresentationOverflow {
            what: "thread",
            actual,
            limit,
        }) if actual == usize::from(u16::MAX) + 1 && limit == usize::from(u16::MAX)
    ));
}

#[test]
fn comparison_domains_and_count_parameters_have_precise_errors() {
    assert!(matches!(
        Weave::try_from(def(
            vec![knot(
                "compare",
                KnotKind::compare(CompareOp::Lt, None, SignalDomain::Bool),
            )],
            vec![],
        )),
        Err(ValidationError::InvalidParameter {
            parameter: "domain",
            reason: "comparison operator does not support the selected domain",
            ..
        })
    ));

    #[cfg(feature = "signal-f32")]
    {
        assert!(matches!(
            Weave::try_from(def(
                vec![knot(
                    "too-large",
                    KnotKind::constant(i32::MAX as f32, SignalDomain::Count),
                )],
                vec![],
            )),
            Err(ValidationError::InvalidParameter {
                parameter: "value",
                reason: "must fit in i32 for Count domain",
                ..
            })
        ));
        assert!(matches!(
            Weave::try_from(def(
                vec![knot(
                    "fractional",
                    KnotKind::constant(1.5, SignalDomain::Count),
                )],
                vec![],
            )),
            Err(ValidationError::InvalidParameter {
                parameter: "value",
                reason: "must be a whole number for Count domain",
                ..
            })
        ));
    }

    assert!(Weave::try_from(def(
        vec![knot(
            "count",
            KnotKind::constant(from_count(1), SignalDomain::Count),
        )],
        vec![],
    ))
    .is_ok());
}

#[test]
fn every_public_authoring_error_formats_and_validation_wraps_as_a_source() {
    let validation_errors = vec![
        ValidationError::InvalidWeaveId {
            weave_id: "weave".into(),
            reason: "bad",
        },
        ValidationError::InvalidKnotId {
            knot_id: "knot".into(),
            reason: "bad",
        },
        ValidationError::EmptyWeave {
            weave_id: "weave".into(),
        },
        ValidationError::DuplicateKnotId {
            knot_id: "knot".into(),
        },
        ValidationError::UnknownKnot {
            knot_id: "knot".into(),
        },
        ValidationError::UnknownPort {
            knot_id: "knot".into(),
            port: "port".into(),
            expected: vec!["first".into(), "second".into()],
        },
        ValidationError::WrongPortDirection {
            knot_id: "knot".into(),
            port: "port".into(),
            expected: PortDir::In,
            actual: PortDir::Out,
        },
        ValidationError::FanIn {
            knot_id: "knot".into(),
            port: "port".into(),
        },
        ValidationError::Cycle {
            at_knot: Some("knot".into()),
        },
        ValidationError::Cycle { at_knot: None },
        ValidationError::UnconnectedRequired {
            knot_id: "knot".into(),
            port: "port".into(),
        },
        ValidationError::BudgetExceeded {
            metric: "knots",
            actual: 2,
            limit: 1,
            at_knot: Some("knot".into()),
        },
        ValidationError::BudgetExceeded {
            metric: "threads",
            actual: 2,
            limit: 1,
            at_knot: None,
        },
        ValidationError::NumericMismatch {
            expected: NumericPath::compiled(),
            actual: other_numeric_path(),
        },
        ValidationError::SignalDomainMismatch {
            from_knot: "from".into(),
            from_port: "out".into(),
            from_domain: SignalDomain::Bool,
            to_knot: "to".into(),
            to_port: "in".into(),
            to_domain: SignalDomain::Count,
        },
        ValidationError::UnresolvedSignalDomain {
            knot_id: "knot".into(),
            port: "port".into(),
        },
        ValidationError::InvalidParameter {
            knot_id: "knot".into(),
            parameter: "parameter",
            reason: "bad",
        },
        ValidationError::RepresentationOverflow {
            what: "knot",
            actual: 2,
            limit: 1,
        },
        ValidationError::InvalidPatternId {
            pattern_id: "pattern".into(),
            reason: "bad",
        },
        ValidationError::DuplicateExport {
            export: "export".into(),
        },
        ValidationError::DuplicatePatternInput {
            knot_id: "knot".into(),
            port: "port".into(),
            first_export: "first".into(),
            duplicate_export: "second".into(),
        },
        ValidationError::PatternInputAlreadyConnected {
            export: "export".into(),
            knot_id: "knot".into(),
            port: "port".into(),
        },
    ];
    for error in &validation_errors {
        assert!(!error.to_string().is_empty());
    }

    let build_errors = vec![
        BuildError::InvalidId {
            id: "id".into(),
            reason: "bad",
        },
        BuildError::DuplicateKnotId {
            knot_id: "knot".into(),
        },
        BuildError::ForeignHandle,
        BuildError::UnknownPort {
            knot_id: "knot".into(),
            port: "port".into(),
            expected: vec!["first".into(), "second".into()],
        },
        BuildError::WrongPortDirection {
            knot_id: "knot".into(),
            port: "port".into(),
            expected: PortDir::In,
            actual: PortDir::Out,
        },
        BuildError::UnknownExport {
            instance_id: "instance".into(),
            export: "export".into(),
            direction: PortDir::Out,
        },
        BuildError::NumericMismatch {
            expected: NumericPath::compiled(),
            actual: other_numeric_path(),
        },
        BuildError::SignalDomainMismatch {
            from_knot: "from".into(),
            from_port: "out".into(),
            from_domain: SignalDomain::Bool,
            to_knot: "to".into(),
            to_port: "in".into(),
            to_domain: SignalDomain::Count,
        },
        BuildError::RepresentationOverflow {
            what: "knot",
            actual: 2,
            limit: 1,
        },
        BuildError::from(validation_errors[0].clone()),
    ];
    for error in &build_errors {
        assert!(!error.to_string().is_empty());
    }
    assert!(build_errors
        .last()
        .expect("wrapped validation")
        .source()
        .is_some());
    assert!(BuildError::ForeignHandle.source().is_none());
}
