use std::string::String;
use std::vec::Vec;

use wyrd_core::{port_slot, KnotId, KnotKind, NumericPath, PortSlot, Result, WyrdError};

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

/// Resolve name → slot using kind table (builder helper).
#[allow(dead_code)]
pub fn slot_of(kind: &KnotKind, name: &str) -> Result<PortSlot> {
    port_slot(kind, name).ok_or(WyrdError::UnknownPort)
}
