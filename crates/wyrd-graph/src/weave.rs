use std::string::String;
use std::vec::Vec;

use wyrd_core::{KnotKind, NumericPath};

/// Author-facing port ref (strings → PortSlot at validate).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortRefAuthor {
    pub knot: String,
    pub port: String,
}

impl PortRefAuthor {
    pub fn new(knot: impl Into<String>, port: impl Into<String>) -> Self {
        Self {
            knot: knot.into(),
            port: port.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KnotDef {
    pub id: String,
    pub kind: KnotKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadDef {
    pub from: PortRefAuthor,
    pub to: PortRefAuthor,
}

/// Stringy author Weave (asset / builder product).
#[derive(Clone, Debug, PartialEq)]
pub struct Weave {
    pub id: String,
    pub knots: Vec<KnotDef>,
    pub threads: Vec<ThreadDef>,
    pub numeric: NumericPath,
}

impl Weave {
    pub fn builder(id: impl Into<String>) -> crate::builder::WeaveBuilder {
        crate::builder::WeaveBuilder::new(id)
    }
}
