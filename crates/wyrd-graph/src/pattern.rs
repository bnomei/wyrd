//! Pattern expand-at-load (D-pattern). Runtime never sees Pattern.
//! Nested Pattern stamps: pre-flatten when authoring; expand is one level only (v0).

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

use wyrd_core::{port_slot, ports_of, PortDir, Result, WyrdError};

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
/// Does not validate required ports (parent must wire exports_in then `validate`).
pub fn expand_pattern(
    instance_id: &str,
    pattern: &Pattern,
) -> Result<(Vec<KnotDef>, Vec<ThreadDef>, PatternExports)> {
    if instance_id.is_empty() || instance_id.contains('/') {
        return Err(WyrdError::Msg("invalid pattern instance id"));
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
    let mut inner_ids: BTreeMap<&str, &KnotDef> = BTreeMap::new();
    for k in &pattern.inner.knots {
        if k.id.contains('/') {
            return Err(WyrdError::Msg("inner knot id must not contain '/'"));
        }
        if inner_ids.insert(k.id.as_str(), k).is_some() {
            return Err(WyrdError::DuplicateKnotId);
        }
        let mut id = prefix.clone();
        id.push_str(&k.id);
        knots.push(KnotDef {
            id,
            kind: k.kind.clone(),
        });
    }

    let mut threads = Vec::with_capacity(pattern.inner.threads.len());
    for t in &pattern.inner.threads {
        if !inner_ids.contains_key(t.from.knot.as_str())
            || !inner_ids.contains_key(t.to.knot.as_str())
        {
            return Err(WyrdError::UnknownKnot);
        }
        let fk = inner_ids[t.from.knot.as_str()];
        let tk = inner_ids[t.to.knot.as_str()];
        if port_slot(&fk.kind, &t.from.port).is_none() || port_slot(&tk.kind, &t.to.port).is_none()
        {
            return Err(WyrdError::UnknownPort);
        }
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
        if exports.ins.contains_key(export.as_str()) {
            return Err(WyrdError::DuplicateKnotId);
        }
        let def = *inner_ids.get(knot.as_str()).ok_or(WyrdError::UnknownKnot)?;
        let info = ports_of(&def.kind)
            .iter()
            .find(|p| p.name == port.as_str())
            .ok_or(WyrdError::UnknownPort)?;
        if info.dir != PortDir::In {
            return Err(WyrdError::UnknownPort);
        }
        exports.ins.insert(
            export.clone(),
            PortRefAuthor::new(prefixed(&prefix, knot), port.as_str()),
        );
    }
    for (export, knot, port) in &pattern.exports_out {
        if exports.outs.contains_key(export.as_str()) {
            return Err(WyrdError::DuplicateKnotId);
        }
        let def = *inner_ids.get(knot.as_str()).ok_or(WyrdError::UnknownKnot)?;
        let info = ports_of(&def.kind)
            .iter()
            .find(|p| p.name == port.as_str())
            .ok_or(WyrdError::UnknownPort)?;
        if info.dir != PortDir::Out {
            return Err(WyrdError::UnknownPort);
        }
        exports.outs.insert(
            export.clone(),
            PortRefAuthor::new(prefixed(&prefix, knot), port.as_str()),
        );
    }

    Ok((knots, threads, exports))
}

fn prefixed(prefix: &str, knot: &str) -> String {
    let mut s = String::from(prefix);
    s.push_str(knot);
    s
}

/// Stamp pattern into parent weave (flat). Parent numeric must match pattern.inner.
/// Caller must wire exports then `validate`.
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
    use wyrd_core::{KnotKind, NumericPath, TimerMode};

    fn monostable_pattern() -> Pattern {
        let (b, _) = Weave::builder("pat.mono")
            .knot("edge", KnotKind::rising_from_zero())
            .unwrap();
        let (b, _) = b
            .knot("t", KnotKind::timer(TimerMode::PulseHold, 2))
            .unwrap();
        let inner = b
            .wire_named("edge", "out", "t", "start")
            .build()
            .unwrap();
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

        parent.threads.push(ThreadDef {
            from: PortRefAuthor::new("btn", "out"),
            to: e1.port_in("start").unwrap().clone(),
        });
        parent.threads.push(ThreadDef {
            from: e1.port_out("active").unwrap().clone(),
            to: PortRefAuthor::new("gate", "in"),
        });
        parent.threads.push(ThreadDef {
            from: PortRefAuthor::new("btn", "out"),
            to: e2.port_in("start").unwrap().clone(),
        });

        validate(&parent, &Budget::default()).unwrap();
        assert!(parent.knots.iter().any(|k| k.id == "hold2/t"));
    }

    #[test]
    fn include_builder_path() {
        let p = monostable_pattern();
        let (b, _) = Weave::builder("lvl").knot("btn", KnotKind::signal_in()).unwrap();
        let (b, exp) = b.include("h1", &p).unwrap();
        let start = exp.port_in("start").unwrap();
        let b = b.wire_named("btn", "out", &start.knot, &start.port);
        let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
        let active = exp.port_out("active").unwrap();
        let w = b
            .wire_named(&active.knot, &active.port, "out", "in")
            .build()
            .unwrap();
        validate(&w, &Budget::default()).unwrap();
    }

    #[test]
    fn empty_instance_id_fails() {
        let p = monostable_pattern();
        assert!(expand_pattern("", &p).is_err());
    }

    #[test]
    fn bad_export_port_fails() {
        let mut p = monostable_pattern();
        p.exports_in = vec![("start".into(), "edge".into(), "nope".into())];
        assert!(matches!(
            expand_pattern("x", &p),
            Err(WyrdError::UnknownPort)
        ));
    }

    #[test]
    fn export_wrong_dir_fails() {
        let mut p = monostable_pattern();
        // edge "out" is Out, not In
        p.exports_in = vec![("start".into(), "edge".into(), "out".into())];
        assert!(matches!(
            expand_pattern("x", &p),
            Err(WyrdError::UnknownPort)
        ));
    }

    #[test]
    fn numeric_mismatch_on_merge() {
        let p = monostable_pattern();
        let (b, _) = Weave::builder("p").knot("a", KnotKind::signal_in()).unwrap();
        let mut parent = b.build().unwrap();
        let mut p2 = p;
        p2.inner.numeric = match parent.numeric {
            NumericPath::F32 => NumericPath::I32Q16,
            NumericPath::I32Q16 => NumericPath::F32,
        };
        assert!(matches!(
            merge_expanded(&mut parent, "h", &p2),
            Err(WyrdError::NumericMismatch)
        ));
    }

    #[test]
    fn duplicate_instance_fails() {
        let p = monostable_pattern();
        let (b, _) = Weave::builder("p").knot("a", KnotKind::signal_in()).unwrap();
        let mut parent = b.build().unwrap();
        merge_expanded(&mut parent, "hold1", &p).unwrap();
        assert!(matches!(
            merge_expanded(&mut parent, "hold1", &p),
            Err(WyrdError::DuplicateKnotId)
        ));
    }
}
