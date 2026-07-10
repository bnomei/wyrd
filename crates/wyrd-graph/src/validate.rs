use core::fmt;

use std::collections::BTreeMap;
use std::string::String;
use std::vec;
use std::vec::Vec;

use wyrd_core::{
    port_slot, ports_of, KnotKind, NumericPath, PortDir, Result, WyrdError,
};

use crate::weave::Weave;

/// Soft/hard budgets (D-math-shape / vision table defaults).
///
/// Hard fields fail [`validate`]. Soft fields are recorded for hosts/tools
/// (see `validate_report` when present); they do not fail bind by default.
#[derive(Clone, Debug)]
pub struct Budget {
    /// Hard max knots (default 256).
    pub max_knots: u16,
    /// Hard max threads (default 512).
    pub max_threads: u16,
    /// Soft knot ceiling (default 64) — warning via [`validate_report`].
    pub soft_knots: u16,
    /// Soft thread ceiling (default 128).
    pub soft_threads: u16,
    /// Hard longest path length in edges (default 16).
    pub max_chain_depth: u16,
    /// Soft chain depth (default 8).
    pub soft_chain_depth: u16,
    /// Hard max outbound threads from any single knot (default 8).
    pub max_fan_out: u16,
    /// Soft fan-out (default 4).
    pub soft_fan_out: u16,
    /// Hard max sum of Delay.ticks along any root→sink path (default 32).
    pub max_delay_path_sum: u16,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_knots: 256,
            max_threads: 512,
            soft_knots: 64,
            soft_threads: 128,
            max_chain_depth: 16,
            soft_chain_depth: 8,
            max_fan_out: 8,
            soft_fan_out: 4,
            max_delay_path_sum: 32,
        }
    }
}

/// Soft budget exceeded (does not fail bind / [`validate`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BudgetWarning {
    SoftKnots { count: u16, soft: u16 },
    SoftThreads { count: u16, soft: u16 },
    SoftChainDepth {
        depth: u16,
        soft: u16,
        at_knot: String,
    },
    SoftFanOut {
        fan_out: u16,
        soft: u16,
        at_knot: String,
    },
}

impl fmt::Display for BudgetWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BudgetWarning::SoftKnots { count, soft } => {
                write!(f, "soft knot budget: {count} knots (soft {soft})")
            }
            BudgetWarning::SoftThreads { count, soft } => {
                write!(f, "soft thread budget: {count} threads (soft {soft})")
            }
            BudgetWarning::SoftChainDepth {
                depth,
                soft,
                at_knot,
            } => write!(
                f,
                "soft chain depth: {depth} edges at knot '{at_knot}' (soft {soft})"
            ),
            BudgetWarning::SoftFanOut {
                fan_out,
                soft,
                at_knot,
            } => write!(
                f,
                "soft fan-out: {fan_out} from knot '{at_knot}' (soft {soft})"
            ),
        }
    }
}

/// Successful validation plus any soft-budget warnings for hosts/tools.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValidateReport {
    pub warnings: Vec<BudgetWarning>,
}

impl ValidateReport {
    pub fn ok(&self) -> bool {
        self.warnings.is_empty()
    }
}

impl fmt::Display for ValidateReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.warnings.is_empty() {
            return f.write_str("validate ok");
        }
        for (i, w) in self.warnings.iter().enumerate() {
            if i > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{w}")?;
        }
        Ok(())
    }
}

/// Validate author Weave: hard fails as `Err`; soft budgets → empty or populated report.
pub fn validate(weave: &Weave, budget: &Budget) -> Result<()> {
    validate_report(weave, budget).map(|_| ())
}

/// Like [`validate`], but returns soft warnings on success.
pub fn validate_report(weave: &Weave, budget: &Budget) -> Result<ValidateReport> {
    if weave.knots.is_empty() {
        return Err(WyrdError::Empty);
    }
    if weave.knots.len() > budget.max_knots as usize {
        return Err(WyrdError::Budget);
    }
    if weave.threads.len() > budget.max_threads as usize {
        return Err(WyrdError::Budget);
    }
    if weave.numeric != NumericPath::compiled() {
        return Err(WyrdError::NumericMismatch);
    }

    let mut index: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, k) in weave.knots.iter().enumerate() {
        if index.insert(k.id.as_str(), i).is_some() {
            return Err(WyrdError::DuplicateKnotId);
        }
        // Port tables must exist (e.g. arity)
        if ports_of(&k.kind).is_empty() {
            return Err(WyrdError::UnknownPort);
        }
    }

    // fan-in: (knot_idx, port_slot) → count of inbound
    let mut fan: BTreeMap<(usize, u8), u8> = BTreeMap::new();
    // edges for cycle: from_knot_idx -> to_knot_idx
    let mut edges: Vec<(usize, usize)> = Vec::new();

    for t in &weave.threads {
        let fi = *index.get(t.from.knot.as_str()).ok_or(WyrdError::UnknownKnot)?;
        let ti = *index.get(t.to.knot.as_str()).ok_or(WyrdError::UnknownKnot)?;
        let fk = &weave.knots[fi].kind;
        let tk = &weave.knots[ti].kind;

        let fs = port_slot(fk, t.from.port.as_str()).ok_or(WyrdError::UnknownPort)?;
        let ts = port_slot(tk, t.to.port.as_str()).ok_or(WyrdError::UnknownPort)?;

        let from_info = ports_of(fk)
            .iter()
            .find(|p| p.slot == fs)
            .ok_or(WyrdError::UnknownPort)?;
        let to_info = ports_of(tk)
            .iter()
            .find(|p| p.slot == ts)
            .ok_or(WyrdError::UnknownPort)?;

        if from_info.dir != PortDir::Out || to_info.dir != PortDir::In {
            return Err(WyrdError::UnknownPort);
        }

        let key = (ti, ts.0);
        let c = fan.entry(key).or_insert(0);
        *c = c.saturating_add(1);
        if *c > 1 {
            return Err(WyrdError::FanIn);
        }

        edges.push((fi, ti));
    }

    // Fan-out hard + soft (edge endpoints are always valid knot indices).
    let mut fout = vec![0u16; weave.knots.len()];
    for &(a, _) in &edges {
        fout[a] = fout[a].saturating_add(1);
        if fout[a] > budget.max_fan_out {
            return Err(WyrdError::Budget);
        }
    }

    // Required inputs connected (Compare `rhs` optional when `rhs_const` is set).
    for (ti, k) in weave.knots.iter().enumerate() {
        for p in ports_of(&k.kind) {
            if p.dir != PortDir::In {
                continue;
            }
            let need = if let KnotKind::Compare { rhs_const, .. } = &k.kind {
                if p.name == "rhs" {
                    rhs_const.is_none()
                } else {
                    p.required
                }
            } else {
                p.required
            };
            if !need {
                continue;
            }
            if !fan.contains_key(&(ti, p.slot.0)) {
                // Sense outputs don't need inbound; required Ins do
                // SignalIn/Constant/OnStart have no required ins
                return Err(WyrdError::UnconnectedRequired);
            }
        }
    }

    // DAG: Kahn topo
    let n = weave.knots.len();
    let mut indeg = vec![0u32; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (a, b) in edges {
        if a != b {
            adj[a].push(b);
            indeg[b] += 1;
        } else {
            return Err(WyrdError::Cycle);
        }
    }
    let mut q: Vec<usize> = indeg
        .iter()
        .enumerate()
        .filter_map(|(i, d)| if *d == 0 { Some(i) } else { None })
        .collect();
    let mut seen = 0usize;
    while let Some(u) = q.pop() {
        seen += 1;
        for &v in &adj[u] {
            indeg[v] -= 1;
            if indeg[v] == 0 {
                q.push(v);
            }
        }
    }
    if seen != n {
        return Err(WyrdError::Cycle);
    }

    // Longest path (edge count) + delay-tick path sum on the DAG.
    // delay_contrib(v) = Delay.ticks if knot is Delay, else 0.
    // path delay sum includes every Delay on the path (including endpoints).
    let mut depth = vec![0u16; n];
    let mut delay_sum = vec![0u32; n];
    for (i, k) in weave.knots.iter().enumerate() {
        delay_sum[i] = delay_ticks(&k.kind) as u32;
    }
    // Process in topo order (reuse Kahn with fresh indeg from adj).
    let mut indeg2 = vec![0u32; n];
    for a in 0..n {
        for &b in &adj[a] {
            indeg2[b] += 1;
        }
    }
    let mut q2: Vec<usize> = indeg2
        .iter()
        .enumerate()
        .filter_map(|(i, d)| if *d == 0 { Some(i) } else { None })
        .collect();
    while let Some(u) = q2.pop() {
        if depth[u] > budget.max_chain_depth {
            return Err(WyrdError::Budget);
        }
        if delay_sum[u] > budget.max_delay_path_sum as u32 {
            return Err(WyrdError::Budget);
        }
        for &v in &adj[u] {
            let nd = depth[u].saturating_add(1);
            if nd > depth[v] {
                depth[v] = nd;
            }
            let ds = delay_sum[u].saturating_add(delay_ticks(&weave.knots[v].kind) as u32);
            if ds > delay_sum[v] {
                delay_sum[v] = ds;
            }
            indeg2[v] -= 1;
            if indeg2[v] == 0 {
                q2.push(v);
            }
        }
    }
    // Depth/delay are checked when each node is dequeued (preds complete).

    // Soft warnings (never fail).
    let mut warnings = Vec::new();
    let knot_count = weave.knots.len() as u16;
    if knot_count > budget.soft_knots {
        warnings.push(BudgetWarning::SoftKnots {
            count: knot_count,
            soft: budget.soft_knots,
        });
    }
    let thread_count = weave.threads.len() as u16;
    if thread_count > budget.soft_threads {
        warnings.push(BudgetWarning::SoftThreads {
            count: thread_count,
            soft: budget.soft_threads,
        });
    }
    for (i, &fo) in fout.iter().enumerate() {
        if fo > budget.soft_fan_out {
            warnings.push(BudgetWarning::SoftFanOut {
                fan_out: fo,
                soft: budget.soft_fan_out,
                at_knot: String::from(weave.knots[i].id.as_str()),
            });
        }
    }
    for (i, &d) in depth.iter().enumerate() {
        if d > budget.soft_chain_depth {
            warnings.push(BudgetWarning::SoftChainDepth {
                depth: d,
                soft: budget.soft_chain_depth,
                at_knot: String::from(weave.knots[i].id.as_str()),
            });
        }
    }

    Ok(ValidateReport { warnings })
}

fn delay_ticks(kind: &KnotKind) -> u16 {
    match kind {
        KnotKind::Delay { ticks } => *ticks,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Weave;
    use std::format;
    use std::vec;
    use wyrd_core::{KnotKind, NumericPath, ONE};

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
        use crate::weave::KnotDef;
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
        use wyrd_core::CompareOp;
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
        use wyrd_core::CompareOp;
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
}
