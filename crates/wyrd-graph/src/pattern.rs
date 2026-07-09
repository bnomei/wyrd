//! Pattern expand-at-load (D-pattern). Runtime never sees Pattern.

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

use wyrd_core::{NumericPath, Result, WyrdError};

use crate::weave::{KnotDef, PortRefAuthor, ThreadDef, Weave};

/// Reusable Weave fragment with named exports (inner knot/port endpoints).
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub id: String,
    pub inner: Weave,
    /// (export_name, inner_knot_id, inner_port_name)
    pub exports_in: Vec<(String, String, String)>,
    /// (export_name, inner_knot_id, inner_port_name)
    pub exports_out: Vec<(String, String, String)>,
}

/// Resolved export endpoints after stamp (prefixed knot ids).
#[derive(Clone, Debug, Default)]
pub struct PatternExports {
    instance_id: String,
    ins: BTreeMap<String, PortRefAuthor>,
    outs: BTreeMap<String, PortRefAuthor>,
}

impl PatternExports {
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Inner (prefixed) knot + port for an input export.
    pub fn port_in(&self, export: &str) -> Result<&PortRefAuthor> {
        self.ins.get(export).ok_or(WyrdError::UnknownPort)
    }

    /// Inner (prefixed) knot + port for an output export.
    pub fn port_out(&self, export: &str) -> Result<&PortRefAuthor> {
        self.outs.get(export).ok_or(WyrdError::UnknownPort)
    }
}

/// Expand `pattern` under `instance_id/` prefix into flat knots + threads.
/// Also builds export → PortRefAuthor map for parent wiring.
pub fn expand_pattern(instance_id: &str, pattern: &Pattern) -> Result<(Vec<KnotDef>, Vec<ThreadDef>, PatternExports)> {
    if instance_id.is_empty() {
        return Err(WyrdError::Msg("empty pattern instance id"));
    }
    if pattern.inner.knots.is_empty() {
        return Err(WyrdError::Empty);
    }

    let prefix = {
        let mut s = String::from(instance_id);
        s.push('/');
        s
    };

    let mut knots = Vec::with_capacity(pattern.inner.knots.len());
    let mut name_set = BTreeMap::new();
    for k in &pattern.inner.knots {
        let mut id = prefix.clone();
        id.push_str(&k.id);
        if name_set.insert(k.id.as_str(), ()).is_some() {
            return Err(WyrdError::DuplicateKnotId);
        }
        knots.push(KnotDef {
            id,
            kind: k.kind.clone(),
        });
    }

    let mut threads = Vec::with_capacity(pattern.inner.threads.len());
    for t in &pattern.inner.threads {
        threads.push(ThreadDef {
            from: PortRefAuthor::new(prefixed(&prefix, &t.from.knot), t.from.port.as_str()),
            to: PortRefAuthor::new(prefixed(&prefix, &t.to.knot), t.to.port.as_str()),
        });
    }

    let mut exports = PatternExports {
        instance_id: String::from(instance_id),
        ins: BTreeMap::new(),
        outs: BTreeMap::new(),
    };

    for (export, knot, port) in &pattern.exports_in {
        if !pattern.inner.knots.iter().any(|k| k.id == *knot) {
            return Err(WyrdError::UnknownKnot);
        }
        exports.ins.insert(
            export.clone(),
            PortRefAuthor::new(prefixed(&prefix, knot), port.as_str()),
        );
    }
    for (export, knot, port) in &pattern.exports_out {
        if !pattern.inner.knots.iter().any(|k| k.id == *knot) {
            return Err(WyrdError::UnknownKnot);
        }
        exports.outs.insert(
            export.clone(),
            PortRefAuthor::new(prefixed(&prefix, knot), port.as_str()),
        );
    }

    // Numeric path of pattern should match when merged (checked by parent validate).
    let _ = pattern.inner.numeric;

    Ok((knots, threads, exports))
}

fn prefixed(prefix: &str, knot: &str) -> String {
    let mut s = String::from(prefix);
    s.push_str(knot);
    s
}

/// Stamp many patterns into a parent weave (flat). Parent numeric must match each pattern.
pub fn merge_expanded(
    parent: &mut Weave,
    instance_id: &str,
    pattern: &Pattern,
) -> Result<PatternExports> {
    if pattern.inner.numeric != parent.numeric {
        return Err(WyrdError::NumericMismatch);
    }
    let (knots, threads, exports) = expand_pattern(instance_id, pattern)?;
    for k in knots {
        if parent.knots.iter().any(|x| x.id == k.id) {
            return Err(WyrdError::DuplicateKnotId);
        }
        parent.knots.push(k);
    }
    parent.threads.extend(threads);
    Ok(exports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::{validate, Budget};
    use std::vec;
    use wyrd_core::{KnotKind, TimerMode, ONE};

    fn monostable_pattern() -> Pattern {
        let (b, _) = Weave::builder("pat.mono")
            .knot("edge", KnotKind::rising_from_zero())
            .unwrap();
        let (b, _) = b
            .knot("t", KnotKind::timer(TimerMode::PulseHold, 2))
            .unwrap();
        // Need an input sense inside? Parent wires into edge/in — edge needs external.
        // Inner: edge <- (external) , edge -> t.start, t.active is export
        // For standalone validate of pattern inner we need a dummy? Pattern inner may be incomplete
        // until parent wires exports_in. So we don't validate pattern alone without stubs.
        let inner = b
            .wire_named("edge", "out", "t", "start")
            .build()
            .unwrap();
        // Inner is incomplete (edge/in unconnected) until stamped and parent wires.
        Pattern {
            id: "pat.mono".into(),
            inner,
            exports_in: vec![("start".into(), "edge".into(), "in".into())],
            exports_out: vec![("active".into(), "t".into(), "active".into())],
        }
    }

    #[test]
    fn expand_prefixes_ids() {
        let p = monostable_pattern();
        let (knots, threads, exp) = expand_pattern("hold1", &p).unwrap();
        assert!(knots.iter().any(|k| k.id == "hold1/edge"));
        assert!(knots.iter().any(|k| k.id == "hold1/t"));
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].from.knot, "hold1/edge");
        assert_eq!(exp.port_in("start").unwrap().knot, "hold1/edge");
        assert_eq!(exp.port_out("active").unwrap().knot, "hold1/t");
        let _ = ONE;
    }

    #[test]
    fn stamp_twice_and_validate_parent() {
        let p = monostable_pattern();
        let (b, _) = Weave::builder("level")
            .knot("btn", KnotKind::signal_in())
            .unwrap();
        let (b, _) = b.knot("gate", KnotKind::signal_out("gate.open")).unwrap();
        let mut parent = b.build().unwrap();

        let e1 = merge_expanded(&mut parent, "hold1", &p).unwrap();
        let e2 = merge_expanded(&mut parent, "hold2", &p).unwrap();

        // wire btn -> hold1 start, hold1 active -> gate
        parent.threads.push(ThreadDef {
            from: PortRefAuthor::new("btn", "out"),
            to: e1.port_in("start").unwrap().clone(),
        });
        parent.threads.push(ThreadDef {
            from: e1.port_out("active").unwrap().clone(),
            to: PortRefAuthor::new("gate", "in"),
        });
        // hold2 unused inputs still need wiring for required ports
        parent.threads.push(ThreadDef {
            from: PortRefAuthor::new("btn", "out"),
            to: e2.port_in("start").unwrap().clone(),
        });

        // hold2/t active unconnected is ok (out). hold2/edge in is wired.
        // Fan-in: btn out fans to two edges — that's fan-out from btn, allowed.
        // Each edge in has one thread — OK.
        validate(&parent, &Budget::default()).unwrap();
        assert!(parent.knots.iter().any(|k| k.id == "hold2/t"));
    }
}
