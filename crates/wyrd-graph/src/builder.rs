use std::string::String;
use std::vec::Vec;

use wyrd_core::{port_slot, KnotId, KnotKind, NumericPath, PortSlot, Result, WyrdError};

use crate::pattern::{expand_pattern, Pattern, PatternExports};
use crate::weave::{KnotDef, PortRefAuthor, ThreadDef, Weave};

/// Rustic builder. Records wires; validate is the loud phase.
pub struct WeaveBuilder {
    id: String,
    knots: Vec<KnotDef>,
    threads: Vec<ThreadDef>,
    numeric: NumericPath,
    /// Author name → dense index assigned at push (preview of KnotId).
    names: Vec<String>,
}

impl WeaveBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            knots: Vec::new(),
            threads: Vec::new(),
            numeric: NumericPath::compiled(),
            names: Vec::new(),
        }
    }

    pub fn numeric(mut self, path: NumericPath) -> Self {
        self.numeric = path;
        self
    }

    /// Add knot by author name. Returns (builder, provisional KnotId = index).
    pub fn knot(mut self, id: impl Into<String>, kind: KnotKind) -> Result<(Self, KnotId)> {
        let id = id.into();
        if self.names.iter().any(|n| n == &id) {
            return Err(WyrdError::DuplicateKnotId);
        }
        let kid = KnotId(self.names.len() as u16);
        self.names.push(id.clone());
        self.knots.push(KnotDef { id, kind });
        Ok((self, kid))
    }

    /// Wire by author names + catalog port names.
    pub fn wire_named(
        mut self,
        from_knot: &str,
        from_port: &str,
        to_knot: &str,
        to_port: &str,
    ) -> Self {
        self.threads.push(ThreadDef {
            from: PortRefAuthor::new(from_knot, from_port),
            to: PortRefAuthor::new(to_knot, to_port),
        });
        self
    }

    /// Wire two author port refs (e.g. from PatternExports).
    pub fn wire_ports(mut self, from: PortRefAuthor, to: PortRefAuthor) -> Self {
        self.threads.push(ThreadDef { from, to });
        self
    }

    /// Wire using KnotIds + PortSlots (preferred after handles exist).
    pub fn wire(mut self, from: (KnotId, PortSlot), to: (KnotId, PortSlot)) -> Result<Self> {
        let fk = self
            .names
            .get(from.0 .0 as usize)
            .ok_or(WyrdError::UnknownKnot)?;
        let tk = self
            .names
            .get(to.0 .0 as usize)
            .ok_or(WyrdError::UnknownKnot)?;
        let from_kind = &self.knots[from.0 .0 as usize].kind;
        let to_kind = &self.knots[to.0 .0 as usize].kind;
        let from_name = port_name(from_kind, from.1).ok_or(WyrdError::UnknownPort)?;
        let to_name = port_name(to_kind, to.1).ok_or(WyrdError::UnknownPort)?;
        self.threads.push(ThreadDef {
            from: PortRefAuthor::new(fk.as_str(), from_name),
            to: PortRefAuthor::new(tk.as_str(), to_name),
        });
        Ok(self)
    }

    /// And arity-2 convenience: creates And knot and wires a.out→in_0, b.out→in_1.
    pub fn and2(
        self,
        id: impl Into<String>,
        a: KnotId,
        b: KnotId,
    ) -> Result<(Self, KnotId)> {
        let (bld, and_id) = self.knot(id, KnotKind::and2())?;
        let bld = bld.wire((a, PortSlot(0)), (and_id, PortSlot(0)))?; // a out → in_0
        let bld = bld.wire((b, PortSlot(0)), (and_id, PortSlot(1)))?;
        Ok((bld, and_id))
    }

    /// Expand pattern under `instance_id/` into this builder (flat). Returns export map.
    pub fn include(
        mut self,
        instance_id: impl Into<String>,
        pattern: &Pattern,
    ) -> Result<(Self, PatternExports)> {
        let instance_id = instance_id.into();
        if pattern.inner.numeric != self.numeric {
            return Err(WyrdError::NumericMismatch);
        }
        let (knots, threads, exports) = expand_pattern(&instance_id, pattern)?;
        for k in knots {
            if self.names.iter().any(|n| n == &k.id) {
                return Err(WyrdError::DuplicateKnotId);
            }
            self.names.push(k.id.clone());
            self.knots.push(k);
        }
        self.threads.extend(threads);
        Ok((self, exports))
    }

    pub fn build(self) -> Result<Weave> {
        if self.knots.is_empty() {
            return Err(WyrdError::Empty);
        }
        Ok(Weave {
            id: self.id,
            knots: self.knots,
            threads: self.threads,
            numeric: self.numeric,
        })
    }
}

fn port_name(kind: &KnotKind, slot: PortSlot) -> Option<&'static str> {
    wyrd_core::ports_of(kind)
        .iter()
        .find(|p| p.slot == slot)
        .map(|p| p.name)
}

/// Resolve catalog port name → slot for a kind.
pub fn slot_of(kind: &KnotKind, name: &str) -> Result<PortSlot> {
    port_slot(kind, name).ok_or(WyrdError::UnknownPort)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Pattern;
    use crate::weave::PortRefAuthor;
    use std::vec;
    use wyrd_core::{KnotKind, NumericPath, ONE, ZERO};

    #[test]
    fn numeric_and_empty_build() {
        let b = WeaveBuilder::new("x").numeric(NumericPath::compiled());
        assert_eq!(b.build(), Err(WyrdError::Empty));
    }

    #[test]
    fn duplicate_knot_on_builder() {
        let (b, _) = WeaveBuilder::new("x")
            .knot("a", KnotKind::constant(ONE))
            .unwrap();
        assert_eq!(
            b.knot("a", KnotKind::constant(ZERO)).map(|_| ()),
            Err(WyrdError::DuplicateKnotId)
        );
    }

    #[test]
    fn wire_unknown_knot_and_port() {
        let (b, a) = WeaveBuilder::new("x")
            .knot("a", KnotKind::constant(ONE))
            .unwrap();
        let bad = KnotId(99);
        assert_eq!(
            b.wire((bad, PortSlot(0)), (a, PortSlot(0))).map(|_| ()),
            Err(WyrdError::UnknownKnot)
        );

        let (b, a) = WeaveBuilder::new("x2")
            .knot("a", KnotKind::constant(ONE))
            .unwrap();
        let bad = KnotId(99);
        assert_eq!(
            b.wire((a, PortSlot(0)), (bad, PortSlot(0))).map(|_| ()),
            Err(WyrdError::UnknownKnot)
        );

        let (b, a) = WeaveBuilder::new("y")
            .knot("a", KnotKind::constant(ONE))
            .unwrap();
        let (b, n) = b.knot("n", KnotKind::not()).unwrap();
        assert_eq!(
            b.wire((a, PortSlot(7)), (n, PortSlot(0))).map(|_| ()),
            Err(WyrdError::UnknownPort)
        );
    }

    #[test]
    fn and2_and_slot_of() {
        let (b, a) = WeaveBuilder::new("d")
            .knot("a", KnotKind::signal_in())
            .unwrap();
        let (b, pb) = b.knot("b", KnotKind::signal_in()).unwrap();
        let (b, both) = b.and2("both", a, pb).unwrap();
        let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
        let w = b
            .wire_named("both", "out", "out", "in")
            .build()
            .unwrap();
        assert_eq!(w.knots.len(), 4);
        let _ = both;
        assert_eq!(slot_of(&KnotKind::not(), "in").unwrap(), PortSlot(0));
        assert_eq!(slot_of(&KnotKind::not(), "nope"), Err(WyrdError::UnknownPort));
    }

    #[test]
    fn include_numeric_mismatch_and_dup() {
        let (b, _) = Weave::builder("pat")
            .knot("edge", KnotKind::rising_from_zero())
            .unwrap();
        let inner = b.build().unwrap();
        let mut pat = Pattern {
            id: "p".into(),
            inner,
            exports_in: vec![("start".into(), "edge".into(), "in".into())],
            exports_out: vec![("out".into(), "edge".into(), "out".into())],
        };
        #[cfg(feature = "signal-f32")]
        {
            pat.inner.numeric = NumericPath::I32Q16;
        }
        #[cfg(feature = "signal-i32")]
        {
            pat.inner.numeric = NumericPath::F32;
        }
        let b = WeaveBuilder::new("host").knot("x", KnotKind::signal_in()).unwrap().0;
        assert_eq!(
            b.include("i1", &pat).map(|_| ()),
            Err(WyrdError::NumericMismatch)
        );

        // restore numeric, then collide with existing id after first include
        pat.inner.numeric = NumericPath::compiled();
        let (b, _) = WeaveBuilder::new("host2")
            .knot("i1/edge", KnotKind::signal_in())
            .unwrap();
        assert_eq!(
            b.include("i1", &pat).map(|_| ()),
            Err(WyrdError::DuplicateKnotId)
        );

        let _ = PortRefAuthor::new("a", "b");
    }
}
