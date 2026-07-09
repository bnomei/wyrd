use std::collections::BTreeMap;
use std::vec;
use std::vec::Vec;

use wyrd_core::{
    port_slot, ports_of, KnotKind, NumericPath, PortDir, Result, WyrdError,
};

use crate::weave::Weave;

/// Soft/hard budgets (D-math-shape defaults).
#[derive(Clone, Debug)]
pub struct Budget {
    pub max_knots: u16,
    pub max_threads: u16,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_knots: 256,
            max_threads: 512,
        }
    }
}

/// Validate author Weave: names unique, ports known, fan-in ≤ 1, DAG, budgets, numeric.
pub fn validate(weave: &Weave, budget: &Budget) -> Result<()> {
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

    // Required inputs connected (with Compare rhs_const exception)
    for (ti, k) in weave.knots.iter().enumerate() {
        for p in ports_of(&k.kind) {
            if p.dir != PortDir::In || !p.required {
                // Compare rhs special-case
                if let KnotKind::Compare { rhs_const, .. } = &k.kind {
                    if p.name == "rhs" && rhs_const.is_some() {
                        continue;
                    }
                }
                continue;
            }
            // required flag on PortInfo
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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Weave;
    use wyrd_core::{KnotKind, ONE};

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
}
