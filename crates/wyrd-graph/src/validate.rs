//! Structural validation and soft/hard resource budgets for weaves.
//!
//! Definition validation (`validate_def`) enforces ids, ports, fan-in, required
//! connections, numeric path match, and acyclicity. Post-validation budget
//! checks (`validate` / `validate_report`) enforce hard limits and collect soft
//! warnings for tooling. Runtime bind re-runs budget validation with bind opts.

use core::fmt;

use std::collections::{BTreeMap, BTreeSet};
use std::prelude::v1::vec;
use std::string::String;
use std::vec::Vec;

use wyrd_core::{ports_of, KnotKind, NumericPath, PortDir, Signal};

use crate::{ValidationError, Weave, WeaveDef};

/// Hard and soft resource limits applied after structural validation.
#[derive(Clone, Debug)]
pub struct Budget {
    pub max_knots: u16,
    pub max_threads: u16,
    pub soft_knots: u16,
    pub soft_threads: u16,
    pub max_chain_depth: u16,
    pub soft_chain_depth: u16,
    pub max_fan_out: u16,
    pub soft_fan_out: u16,
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

/// Soft-limit warning; graph remains valid for bind.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BudgetWarning {
    SoftKnots {
        count: u16,
        soft: u16,
    },
    SoftThreads {
        count: u16,
        soft: u16,
    },
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
            Self::SoftKnots { count, soft } => {
                write!(f, "soft knot budget: {count} knots (soft {soft})")
            }
            Self::SoftThreads { count, soft } => {
                write!(f, "soft thread budget: {count} threads (soft {soft})")
            }
            Self::SoftChainDepth {
                depth,
                soft,
                at_knot,
            } => write!(
                f,
                "soft chain depth: {depth} edges at knot '{at_knot}' (soft {soft})"
            ),
            Self::SoftFanOut {
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

/// Soft budget warnings from a successful hard-limit pass.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValidateReport {
    pub warnings: Vec<BudgetWarning>,
}
impl ValidateReport {
    /// True when no soft warnings were recorded.
    pub fn ok(&self) -> bool {
        self.warnings.is_empty()
    }
}

impl fmt::Display for ValidateReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.warnings.is_empty() {
            return f.write_str("validate ok");
        }
        for (i, warning) in self.warnings.iter().enumerate() {
            if i != 0 {
                f.write_str("; ")?;
            }
            warning.fmt(f)?;
        }
        Ok(())
    }
}

/// Hard budget check only; discards soft warnings.
pub fn validate(weave: &Weave, budget: &Budget) -> Result<(), ValidationError> {
    validate_report(weave, budget).map(|_| ())
}

/// Hard budget check plus soft-limit warnings for tooling.
pub fn validate_report(weave: &Weave, budget: &Budget) -> Result<ValidateReport, ValidationError> {
    let knots = weave.knots();
    let threads = weave.threads();
    budget_limit("knots", knots.len(), budget.max_knots as usize, None)?;
    budget_limit("threads", threads.len(), budget.max_threads as usize, None)?;

    let index: BTreeMap<&str, usize> = knots
        .iter()
        .enumerate()
        .map(|(i, knot)| (knot.id.as_str(), i))
        .collect();
    let mut adj = vec![Vec::new(); knots.len()];
    let mut indeg = vec![0u16; knots.len()];
    let mut fan_out = vec![0u16; knots.len()];
    for thread in threads {
        let from = index[thread.from.knot.as_str()];
        let to = index[thread.to.knot.as_str()];
        adj[from].push(to);
        indeg[to] = indeg[to].saturating_add(1);
        fan_out[from] = fan_out[from].saturating_add(1);
        budget_limit(
            "fan-out",
            fan_out[from] as usize,
            budget.max_fan_out as usize,
            Some(&knots[from].id),
        )?;
    }

    let mut queue: Vec<usize> = indeg
        .iter()
        .enumerate()
        .filter_map(|(i, d)| (*d == 0).then_some(i))
        .collect();
    let mut depth = vec![0u16; knots.len()];
    let mut delay_sum: Vec<u32> = knots
        .iter()
        .map(|k| u32::from(delay_ticks(&k.kind)))
        .collect();
    while let Some(node) = queue.pop() {
        budget_limit(
            "chain depth",
            depth[node] as usize,
            budget.max_chain_depth as usize,
            Some(&knots[node].id),
        )?;
        budget_limit(
            "delay path sum",
            delay_sum[node] as usize,
            budget.max_delay_path_sum as usize,
            Some(&knots[node].id),
        )?;
        for &next in &adj[node] {
            depth[next] = depth[next].max(depth[node].saturating_add(1));
            delay_sum[next] = delay_sum[next]
                .max(delay_sum[node].saturating_add(u32::from(delay_ticks(&knots[next].kind))));
            indeg[next] -= 1;
            if indeg[next] == 0 {
                queue.push(next);
            }
        }
    }

    let mut warnings = Vec::new();
    if knots.len() > budget.soft_knots as usize {
        warnings.push(BudgetWarning::SoftKnots {
            count: knots.len() as u16,
            soft: budget.soft_knots,
        });
    }
    if threads.len() > budget.soft_threads as usize {
        warnings.push(BudgetWarning::SoftThreads {
            count: threads.len() as u16,
            soft: budget.soft_threads,
        });
    }
    for (i, &count) in fan_out.iter().enumerate() {
        if count > budget.soft_fan_out {
            warnings.push(BudgetWarning::SoftFanOut {
                fan_out: count,
                soft: budget.soft_fan_out,
                at_knot: knots[i].id.clone(),
            });
        }
    }
    for (i, &count) in depth.iter().enumerate() {
        if count > budget.soft_chain_depth {
            warnings.push(BudgetWarning::SoftChainDepth {
                depth: count,
                soft: budget.soft_chain_depth,
                at_knot: knots[i].id.clone(),
            });
        }
    }
    Ok(ValidateReport { warnings })
}

pub(crate) fn validate_def(def: &WeaveDef) -> Result<(), ValidationError> {
    validate_def_with_external_inputs(def, &BTreeSet::new())
}

pub(crate) fn validate_def_with_external_inputs(
    def: &WeaveDef,
    external: &BTreeSet<(String, String)>,
) -> Result<(), ValidationError> {
    if def.id.is_empty() {
        return Err(ValidationError::InvalidWeaveId {
            weave_id: def.id.clone(),
            reason: "must be non-empty",
        });
    }
    if def.knots.is_empty() {
        return Err(ValidationError::EmptyWeave {
            weave_id: def.id.clone(),
        });
    }
    if def.knots.len() > u16::MAX as usize {
        return Err(ValidationError::RepresentationOverflow {
            what: "knot",
            actual: def.knots.len(),
            limit: u16::MAX as usize,
        });
    }
    if def.threads.len() > u16::MAX as usize {
        return Err(ValidationError::RepresentationOverflow {
            what: "thread",
            actual: def.threads.len(),
            limit: u16::MAX as usize,
        });
    }
    if def.numeric != NumericPath::compiled() {
        return Err(ValidationError::NumericMismatch {
            expected: NumericPath::compiled(),
            actual: def.numeric,
        });
    }

    let mut index = BTreeMap::new();
    for (i, knot) in def.knots.iter().enumerate() {
        if knot.id.is_empty() {
            return Err(ValidationError::InvalidKnotId {
                knot_id: knot.id.clone(),
                reason: "must be non-empty",
            });
        }
        if index.insert(knot.id.as_str(), i).is_some() {
            return Err(ValidationError::DuplicateKnotId {
                knot_id: knot.id.clone(),
            });
        }
        validate_kind(knot)?;
    }
    let mut fan = BTreeSet::new();
    let mut adj = vec![Vec::new(); def.knots.len()];
    let mut indeg = vec![0u32; def.knots.len()];
    for thread in &def.threads {
        let &from =
            index
                .get(thread.from.knot.as_str())
                .ok_or_else(|| ValidationError::UnknownKnot {
                    knot_id: thread.from.knot.clone(),
                })?;
        let &to =
            index
                .get(thread.to.knot.as_str())
                .ok_or_else(|| ValidationError::UnknownKnot {
                    knot_id: thread.to.knot.clone(),
                })?;
        check_port(&def.knots[from], &thread.from.port, PortDir::Out)?;
        check_port(&def.knots[to], &thread.to.port, PortDir::In)?;
        if !fan.insert((to, thread.to.port.as_str())) {
            return Err(ValidationError::FanIn {
                knot_id: thread.to.knot.clone(),
                port: thread.to.port.clone(),
            });
        }
        adj[from].push(to);
        indeg[to] += 1;
    }
    for (i, knot) in def.knots.iter().enumerate() {
        for port in ports_of(&knot.kind) {
            if port.dir != PortDir::In || !required(&knot.kind, port.name, port.required) {
                continue;
            }
            let connected = fan.contains(&(i, port.name));
            let exported = external.contains(&(knot.id.clone(), String::from(port.name)));
            if !connected && !exported {
                return Err(ValidationError::UnconnectedRequired {
                    knot_id: knot.id.clone(),
                    port: String::from(port.name),
                });
            }
        }
    }
    let mut queue: Vec<usize> = indeg
        .iter()
        .enumerate()
        .filter_map(|(i, d)| (*d == 0).then_some(i))
        .collect();
    let mut seen = 0usize;
    while let Some(node) = queue.pop() {
        seen += 1;
        for &next in &adj[node] {
            indeg[next] -= 1;
            if indeg[next] == 0 {
                queue.push(next);
            }
        }
    }
    if seen != def.knots.len() {
        let at_knot = indeg
            .iter()
            .position(|d| *d != 0)
            .map(|i| def.knots[i].id.clone());
        return Err(ValidationError::Cycle { at_knot });
    }
    Ok(())
}

fn validate_kind(knot: &crate::KnotDef) -> Result<(), ValidationError> {
    if ports_of(&knot.kind).is_empty() {
        return Err(ValidationError::InvalidParameter {
            knot_id: knot.id.clone(),
            parameter: "arity",
            reason: "unsupported port arity",
        });
    }
    let non_finite = match &knot.kind {
        KnotKind::Constant { value } => non_finite_parameter("value", *value),
        KnotKind::Map {
            in_min,
            in_max,
            out_min,
            out_max,
        }
        | KnotKind::Digitize {
            in_min,
            in_max,
            out_min,
            out_max,
            ..
        } => non_finite_parameter("in_min", *in_min)
            .or_else(|| non_finite_parameter("in_max", *in_max))
            .or_else(|| non_finite_parameter("out_min", *out_min))
            .or_else(|| non_finite_parameter("out_max", *out_max)),
        KnotKind::Threshold { high, low, .. } => {
            non_finite_parameter("high", *high).or_else(|| non_finite_parameter("low", *low))
        }
        KnotKind::Clamp { min, max } => {
            non_finite_parameter("min", *min).or_else(|| non_finite_parameter("max", *max))
        }
        _ => None,
    };
    if let Some(parameter) = non_finite {
        return Err(ValidationError::InvalidParameter {
            knot_id: knot.id.clone(),
            parameter,
            reason: "must be finite",
        });
    }

    let invalid = match &knot.kind {
        KnotKind::Digitize { steps: 0, .. } => Some(("steps", "must be greater than zero")),
        KnotKind::Digitize { in_min, in_max, .. } | KnotKind::Map { in_min, in_max, .. }
            if in_min > in_max =>
        {
            Some(("in_min", "must not exceed in_max"))
        }
        KnotKind::Threshold {
            high,
            low,
            use_hysteresis: true,
        } if low > high => Some(("low", "must not exceed high when hysteresis is enabled")),
        KnotKind::Clamp { min, max } if min > max => Some(("min", "must not exceed max")),
        _ => None,
    };
    if let Some((parameter, reason)) = invalid {
        return Err(ValidationError::InvalidParameter {
            knot_id: knot.id.clone(),
            parameter,
            reason,
        });
    }
    Ok(())
}

#[cfg(feature = "signal-f32")]
fn non_finite_parameter(parameter: &'static str, value: Signal) -> Option<&'static str> {
    (!value.is_finite()).then_some(parameter)
}

#[cfg(feature = "signal-i32")]
fn non_finite_parameter(_parameter: &'static str, _value: Signal) -> Option<&'static str> {
    None
}

fn check_port(knot: &crate::KnotDef, name: &str, expected: PortDir) -> Result<(), ValidationError> {
    let ports = ports_of(&knot.kind);
    let info =
        ports
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| ValidationError::UnknownPort {
                knot_id: knot.id.clone(),
                port: String::from(name),
                expected: ports.iter().map(|p| String::from(p.name)).collect(),
            })?;
    if info.dir != expected {
        return Err(ValidationError::WrongPortDirection {
            knot_id: knot.id.clone(),
            port: String::from(name),
            expected,
            actual: info.dir,
        });
    }
    Ok(())
}

fn required(kind: &KnotKind, port: &str, catalog: bool) -> bool {
    match kind {
        KnotKind::Compare { rhs_const, .. } if port == "rhs" => rhs_const.is_none(),
        KnotKind::Random { require_gate: true } if port == "gate" => true,
        _ => catalog,
    }
}

fn budget_limit(
    metric: &'static str,
    actual: usize,
    limit: usize,
    knot: Option<&str>,
) -> Result<(), ValidationError> {
    if actual > limit {
        return Err(ValidationError::BudgetExceeded {
            metric,
            actual: actual as u32,
            limit: limit as u32,
            at_knot: knot.map(String::from),
        });
    }
    Ok(())
}

fn delay_ticks(kind: &KnotKind) -> u16 {
    match kind {
        KnotKind::Delay { ticks } => *ticks,
        _ => 0,
    }
}
