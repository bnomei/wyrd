use wyrd_graph::{
    weave, BuildError, KnotDef, KnotKind, NumericPath, Pattern, PatternDef, PatternExportDef,
    PortRefDef, SignalDomain, Weave, WeaveBuilder, WeaveDef,
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
