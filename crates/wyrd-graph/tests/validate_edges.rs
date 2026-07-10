//! Validate edge cases (integration; kept out of validate.rs for coverage hygiene).

use std::format;
use std::vec;
use wyrd_core::{CompareOp, KnotKind, NumericPath, WyrdError, ONE};
use wyrd_graph::{
    validate, validate_report, Budget, BudgetWarning, KnotDef, PortRefAuthor, ThreadDef, Weave,
};



    #[test]
    fn hello_ok() {
        let (b, c) = Weave::builder("h")
            .knot("c", KnotKind::constant(ONE))
            .unwrap();
        let (b, n) = b.knot("n", KnotKind::not()).unwrap();
        let (b, _) = b.knot("o", KnotKind::signal_out("debug")).unwrap();
        let w = b
            .wire_named("c", "out", "n", "in")
            .wire_named("n", "out", "o", "in")
            .build()
            .unwrap();
        validate(&w, &Budget::default()).unwrap();
        let _ = (c, n);
    }

    #[test]
    fn fan_in_rejects() {
        let (b, _) = Weave::builder("h")
            .knot("a", KnotKind::constant(ONE))
            .unwrap();
        let (b, _) = b.knot("b", KnotKind::constant(ONE)).unwrap();
        let (b, _) = b.knot("n", KnotKind::not()).unwrap();
        let w = b
            .wire_named("a", "out", "n", "in")
            .wire_named("b", "out", "n", "in")
            .build()
            .unwrap();
        assert_eq!(validate(&w, &Budget::default()), Err(WyrdError::FanIn));
    }

    #[test]
    fn folklore_port_rejects() {
        let (b, _) = Weave::builder("h")
            .knot("a", KnotKind::signal_in())
            .unwrap();
        let (b, _) = b.knot("and", KnotKind::and2()).unwrap();
        let w = b
            .wire_named("a", "out", "and", "a") // folklore — must be in_0
            .build()
            .unwrap();
        assert_eq!(validate(&w, &Budget::default()), Err(WyrdError::UnknownPort));

        // Unknown *from* port name (fs path).
        let w_from = Weave {
            id: "hf".into(),
            knots: vec![
                KnotDef {
                    id: "a".into(),
                    kind: KnotKind::signal_in(),
                },
                KnotDef {
                    id: "n".into(),
                    kind: KnotKind::not(),
                },
            ],
            threads: vec![ThreadDef {
                from: PortRefAuthor::new("a", "nope"),
                to: PortRefAuthor::new("n", "in"),
            }],
            numeric: NumericPath::compiled(),
        };
        assert_eq!(
            validate(&w_from, &Budget::default()),
            Err(WyrdError::UnknownPort)
        );
    }

    #[test]
    fn empty_weave_rejects() {
        let w = Weave {
            id: "e".into(),
            knots: vec![],
            threads: vec![],
            numeric: NumericPath::compiled(),
        };
        assert_eq!(validate(&w, &Budget::default()), Err(WyrdError::Empty));
    }

    #[test]
    fn budget_knots_and_threads() {
        let (b, _) = Weave::builder("b")
            .knot("c", KnotKind::constant(ONE))
            .unwrap();
        let w = b.build().unwrap();
        let tight = Budget {
            max_knots: 0,
            ..Budget::default()
        };
        assert_eq!(validate(&w, &tight), Err(WyrdError::Budget));

        let (b, _) = Weave::builder("b2")
            .knot("a", KnotKind::constant(ONE))
            .unwrap();
        let (b, _) = b.knot("n", KnotKind::not()).unwrap();
        let w = b.wire_named("a", "out", "n", "in").build().unwrap();
        let tight_t = Budget {
            max_threads: 0,
            ..Budget::default()
        };
        assert_eq!(validate(&w, &tight_t), Err(WyrdError::Budget));
    }

    #[test]
    fn fan_out_budget_hard() {
        let (b, _) = Weave::builder("f")
            .knot("c", KnotKind::constant(ONE))
            .unwrap();
        let (b, _) = b.knot("n0", KnotKind::not()).unwrap();
        let (b, _) = b.knot("n1", KnotKind::not()).unwrap();
        let (b, _) = b.knot("n2", KnotKind::not()).unwrap();
        // one source fans out to 3 Not ins
        let w = b
            .wire_named("c", "out", "n0", "in")
            .wire_named("c", "out", "n1", "in")
            .wire_named("c", "out", "n2", "in")
            .build()
            .unwrap();
        let bud = Budget {
            max_fan_out: 2,
            ..Budget::default()
        };
        assert_eq!(validate(&w, &bud), Err(WyrdError::Budget));
    }

    #[test]
    fn chain_depth_budget_hard() {
        // c → n0 → n1 → n2 : depth 3 edges
        let (b, _) = Weave::builder("d")
            .knot("c", KnotKind::constant(ONE))
            .unwrap();
        let (b, _) = b.knot("n0", KnotKind::not()).unwrap();
        let (b, _) = b.knot("n1", KnotKind::not()).unwrap();
        let (b, _) = b.knot("n2", KnotKind::not()).unwrap();
        let w = b
            .wire_named("c", "out", "n0", "in")
            .wire_named("n0", "out", "n1", "in")
            .wire_named("n1", "out", "n2", "in")
            .build()
            .unwrap();
        let bud = Budget {
            max_chain_depth: 2,
            ..Budget::default()
        };
        assert_eq!(validate(&w, &bud), Err(WyrdError::Budget));
    }

    #[test]
    fn soft_warnings_do_not_fail_validate() {
        // 3 edges, soft_chain_depth 2 → warning, still Ok.
        let (b, _) = Weave::builder("d")
            .knot("c", KnotKind::constant(ONE))
            .unwrap();
        let (b, _) = b.knot("n0", KnotKind::not()).unwrap();
        let (b, _) = b.knot("n1", KnotKind::not()).unwrap();
        let (b, _) = b.knot("n2", KnotKind::not()).unwrap();
        let w = b
            .wire_named("c", "out", "n0", "in")
            .wire_named("n0", "out", "n1", "in")
            .wire_named("n1", "out", "n2", "in")
            .build()
            .unwrap();
        let bud = Budget {
            soft_chain_depth: 2,
            soft_knots: 2, // also soft knots (4 knots)
            ..Budget::default()
        };
        validate(&w, &bud).unwrap();
        let rep = validate_report(&w, &bud).unwrap();
        assert!(!rep.ok());
        assert!(rep.warnings.iter().any(|w| matches!(
            w,
            BudgetWarning::SoftChainDepth { depth: 3, soft: 2, .. }
        )));
        assert!(rep.warnings.iter().any(|w| matches!(
            w,
            BudgetWarning::SoftKnots { count: 4, soft: 2 }
        )));
        let s = format!("{rep}");
        assert!(s.contains("soft chain depth"));
        assert!(s.contains("soft knot"));
        assert!(s.contains("; "), "multi-warning Display joins with semicolon");
    }

    #[test]
    fn soft_threads_warns_and_ok_display() {
        let (b, _) = Weave::builder("t")
            .knot("a", KnotKind::constant(ONE))
            .unwrap();
        let (b, _) = b.knot("n0", KnotKind::not()).unwrap();
        let (b, _) = b.knot("n1", KnotKind::not()).unwrap();
        // 2 threads; soft_threads = 1
        let w = b
            .wire_named("a", "out", "n0", "in")
            .wire_named("n0", "out", "n1", "in")
            .build()
            .unwrap();
        let bud = Budget {
            soft_threads: 1,
            ..Budget::default()
        };
        let rep = validate_report(&w, &bud).unwrap();
        assert!(rep.warnings.iter().any(|w| matches!(
            w,
            BudgetWarning::SoftThreads { count: 2, soft: 1 }
        )));
        assert!(format!("{}", rep.warnings[0]).contains("soft thread"));

        let clean = validate_report(&w, &Budget::default()).unwrap();
        assert!(clean.ok());
        assert_eq!(format!("{clean}"), "validate ok");
    }

    #[test]
    fn soft_fan_out_warns() {
        let (b, _) = Weave::builder("f")
            .knot("c", KnotKind::constant(ONE))
            .unwrap();
        let (b, _) = b.knot("n0", KnotKind::not()).unwrap();
        let (b, _) = b.knot("n1", KnotKind::not()).unwrap();
        let (b, _) = b.knot("n2", KnotKind::not()).unwrap();
        let w = b
            .wire_named("c", "out", "n0", "in")
            .wire_named("c", "out", "n1", "in")
            .wire_named("c", "out", "n2", "in")
            .build()
            .unwrap();
        let bud = Budget {
            soft_fan_out: 2,
            max_fan_out: 8,
            ..Budget::default()
        };
        let rep = validate_report(&w, &bud).unwrap();
        assert!(rep.warnings.iter().any(|w| matches!(
            w,
            BudgetWarning::SoftFanOut {
                fan_out: 3,
                soft: 2,
                at_knot
            } if at_knot == "c"
        )));
        assert!(format!("{}", rep.warnings[0]).contains("'c'"));
    }

    #[test]
    fn unknown_from_or_to_knot_rejects() {
        let w = Weave {
            id: "u".into(),
            knots: vec![KnotDef {
                id: "c".into(),
                kind: KnotKind::constant(ONE),
            }],
            threads: vec![ThreadDef {
                from: PortRefAuthor::new("missing", "out"),
                to: PortRefAuthor::new("c", "out"), // wrong dir/port too, but knot first
            }],
            numeric: NumericPath::compiled(),
        };
        assert_eq!(validate(&w, &Budget::default()), Err(WyrdError::UnknownKnot));

        let w2 = Weave {
            id: "u2".into(),
            knots: vec![KnotDef {
                id: "c".into(),
                kind: KnotKind::constant(ONE),
            }],
            threads: vec![ThreadDef {
                from: PortRefAuthor::new("c", "out"),
                to: PortRefAuthor::new("gone", "in"),
            }],
            numeric: NumericPath::compiled(),
        };
        assert_eq!(validate(&w2, &Budget::default()), Err(WyrdError::UnknownKnot));
    }

    #[test]
    fn delay_path_sum_budget_hard() {
        let (b, _) = Weave::builder("d")
            .knot("c", KnotKind::constant(ONE))
            .unwrap();
        let (b, _) = b.knot("d0", KnotKind::Delay { ticks: 10 }).unwrap();
        let (b, _) = b.knot("d1", KnotKind::Delay { ticks: 10 }).unwrap();
        let (b, _) = b.knot("o", KnotKind::signal_out("y")).unwrap();
        let w = b
            .wire_named("c", "out", "d0", "in")
            .wire_named("d0", "out", "d1", "in")
            .wire_named("d1", "out", "o", "in")
            .build()
            .unwrap();
        let bud = Budget {
            max_delay_path_sum: 15,
            ..Budget::default()
        };
        assert_eq!(validate(&w, &bud), Err(WyrdError::Budget));
    }

    #[test]
    fn numeric_mismatch_rejects() {
        let (b, _) = Weave::builder("n")
            .knot("c", KnotKind::constant(ONE))
            .unwrap();
        let mut w = b.build().unwrap();
        #[cfg(feature = "signal-f32")]
        {
            w.numeric = NumericPath::I32Q16;
        }
        #[cfg(feature = "signal-i32")]
        {
            w.numeric = NumericPath::F32;
        }
        assert_eq!(validate(&w, &Budget::default()), Err(WyrdError::NumericMismatch));
    }

    #[test]
    fn duplicate_knot_id_rejects() {
        let w = Weave {
            id: "d".into(),
            knots: vec![
                KnotDef {
                    id: "a".into(),
                    kind: KnotKind::constant(ONE),
                },
                KnotDef {
                    id: "a".into(),
                    kind: KnotKind::constant(ONE),
                },
            ],
            threads: vec![],
            numeric: NumericPath::compiled(),
        };
        assert_eq!(
            validate(&w, &Budget::default()),
            Err(WyrdError::DuplicateKnotId)
        );
    }

    #[test]
    fn unsupported_arity_rejects() {
        let (b, _) = Weave::builder("a")
            .knot("x", KnotKind::And { arity: 5 })
            .unwrap();
        let w = b.build().unwrap();
        assert_eq!(validate(&w, &Budget::default()), Err(WyrdError::UnknownPort));
    }

    #[test]
    fn wrong_port_direction_rejects() {
        // Wire in → in (not Out→In).
        let (b, _) = Weave::builder("d")
            .knot("a", KnotKind::signal_in())
            .unwrap();
        let (b, _) = b.knot("n", KnotKind::not()).unwrap();
        let (b, _) = b.knot("m", KnotKind::not()).unwrap();
        // valid wire a.out→n.in, then n.in→m.in is wrong dir on from (in is In)
        let w = b
            .wire_named("a", "out", "n", "in")
            .wire_named("n", "in", "m", "in")
            .build()
            .unwrap();
        assert_eq!(validate(&w, &Budget::default()), Err(WyrdError::UnknownPort));
    }

    #[test]
    fn unconnected_required_rejects() {
        let (b, _) = Weave::builder("u")
            .knot("n", KnotKind::not())
            .unwrap();
        let w = b.build().unwrap();
        assert_eq!(
            validate(&w, &Budget::default()),
            Err(WyrdError::UnconnectedRequired)
        );
    }

    #[test]
    fn self_loop_and_multi_cycle() {
        let (b, _) = Weave::builder("c")
            .knot("n", KnotKind::not())
            .unwrap();
        let w = b.wire_named("n", "out", "n", "in").build().unwrap();
        assert_eq!(validate(&w, &Budget::default()), Err(WyrdError::Cycle));

        let (b, _) = Weave::builder("c2")
            .knot("a", KnotKind::not())
            .unwrap();
        let (b, _) = b.knot("b", KnotKind::not()).unwrap();
        let w = b
            .wire_named("a", "out", "b", "in")
            .wire_named("b", "out", "a", "in")
            .build()
            .unwrap();
        assert_eq!(validate(&w, &Budget::default()), Err(WyrdError::Cycle));
    }

    #[test]
    fn compare_rhs_const_skips_required_wire() {
        let (b, _) = Weave::builder("cmp")
            .knot("l", KnotKind::constant(ONE))
            .unwrap();
        let (b, _) = b
            .knot("c", KnotKind::compare(CompareOp::Eq, Some(1)))
            .unwrap();
        let (b, _) = b.knot("o", KnotKind::signal_out("y")).unwrap();
        let w = b
            .wire_named("l", "out", "c", "lhs")
            // rhs not wired — allowed when rhs_const is Some
            .wire_named("c", "out", "o", "in")
            .build()
            .unwrap();
        validate(&w, &Budget::default()).unwrap();
    }

    #[test]
    fn compare_rhs_wired_required_when_no_const() {
        let (b, _) = Weave::builder("cmp")
            .knot("l", KnotKind::constant(ONE))
            .unwrap();
        let (b, _) = b.knot("c", KnotKind::compare(CompareOp::Eq, None)).unwrap();
        let (b, _) = b.knot("o", KnotKind::signal_out("y")).unwrap();
        let w = b
            .wire_named("l", "out", "c", "lhs")
            .wire_named("c", "out", "o", "in")
            .build()
            .unwrap();
        // rhs required and missing
        assert_eq!(
            validate(&w, &Budget::default()),
            Err(WyrdError::UnconnectedRequired)
        );
    }
