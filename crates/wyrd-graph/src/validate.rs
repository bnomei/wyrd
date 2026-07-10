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
        // Join first so multi-warning Display has a single write (no `?` residual arms).
        let mut joined = String::new();
        for (i, w) in self.warnings.iter().enumerate() {
            if i > 0 {
                joined.push_str("; ");
            }
            // String Write never fails.
            let _ = fmt::Write::write_fmt(&mut joined, format_args!("{w}"));
        }
        f.write_str(&joined)
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
        match &k.kind {
            KnotKind::Digitize { steps, .. } if *steps == 0 => {
                return Err(WyrdError::InvalidParam);
            }
            KnotKind::Threshold {
                high,
                low,
                use_hysteresis,
            } if *use_hysteresis && low > high => {
                return Err(WyrdError::InvalidParam);
            }
            KnotKind::Clamp { min, max } if min > max => {
                return Err(WyrdError::InvalidParam);
            }
            _ => {}
        }
    }

    // fan-in: (knot_idx, port_slot) → count of inbound
    let mut fan: BTreeMap<(usize, u8), u8> = BTreeMap::new();
    // edges for cycle: from_knot_idx -> to_knot_idx
    let mut edges: Vec<(usize, usize)> = Vec::new();

    for t in &weave.threads {
        let Some(&fi) = index.get(t.from.knot.as_str()) else {
            return Err(WyrdError::UnknownKnot);
        };
        let Some(&ti) = index.get(t.to.knot.as_str()) else {
            return Err(WyrdError::UnknownKnot);
        };
        let fk = &weave.knots[fi].kind;
        let tk = &weave.knots[ti].kind;

        let Some(fs) = port_slot(fk, t.from.port.as_str()) else {
            return Err(WyrdError::UnknownPort);
        };
        let Some(ts) = port_slot(tk, t.to.port.as_str()) else {
            return Err(WyrdError::UnknownPort);
        };

        // Dense 0..n slots: port_slot success ⇒ table index is slot.0.
        let from_info = &ports_of(fk)[fs.0 as usize];
        let to_info = &ports_of(tk)[ts.0 as usize];

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

