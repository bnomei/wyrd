//! Author graph definitions and the validated immutable [`Weave`].
//!
//! [`WeaveDef`] is the editable/serializable form. [`Weave`] is produced only
//! after structural validation (unique ids, port directions, no fan-in, DAG).
//! Runtime bind consumes a `Weave`; it is not executable by itself.

use std::string::String;
use std::vec::Vec;

use crate::foundation::{KnotKind, NumericPath};

use crate::ValidationError;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Serializable author reference to a named knot port.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PortRefDef {
    /// Author knot id (must exist in the same weave or pattern inner graph).
    pub knot: String,
    /// Catalog port name on that knot (for example `"out"`, `"in_0"`).
    pub port: String,
}

impl PortRefDef {
    /// Build a port reference from author knot and catalog port names.
    pub fn new(knot: impl Into<String>, port: impl Into<String>) -> Self {
        Self {
            knot: knot.into(),
            port: port.into(),
        }
    }
}

/// Serializable knot definition.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct KnotDef {
    /// Unique author id within the weave (non-empty, no `/` in pattern inners).
    pub id: String,
    /// Knot kind and parameters from the closed catalog.
    pub kind: KnotKind,
}

/// Serializable directed connection.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ThreadDef {
    /// Source port (must be an output on the referenced knot).
    pub from: PortRefDef,
    /// Destination port (must be an input on the referenced knot).
    pub to: PortRefDef,
}

/// Editable and serializable graph definition. Convert it to [`Weave`] before execution.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WeaveDef {
    /// Author weave id (also mixed into `Random` PRNG seed at bind).
    pub id: String,
    /// Declared numeric path; must match the compiled feature at validate.
    pub numeric: NumericPath,
    /// Knot definitions in author insertion order.
    pub knots: Vec<KnotDef>,
    /// Directed connections between catalog ports.
    pub threads: Vec<ThreadDef>,
}

/// Immutable, structurally validated graph ready for runtime bind.
#[derive(Clone, Debug, PartialEq)]
pub struct Weave {
    id: String,
    numeric: NumericPath,
    knots: Vec<KnotDef>,
    threads: Vec<ThreadDef>,
}

impl Weave {
    /// Start a [`crate::WeaveBuilder`] for this weave id.
    pub fn builder(id: impl Into<String>) -> Result<crate::WeaveBuilder, crate::BuildError> {
        crate::WeaveBuilder::new(id)
    }

    /// Compose a generated graph through a scoped typed [`crate::Composer`].
    ///
    /// Common helpers use [`crate::BoolWire`], [`crate::LevelWire`], and
    /// [`crate::CountWire`] so fixed-domain mistakes cannot be expressed. The
    /// closure may use `knot`/`input`/`output`/`thread` for any catalog entry;
    /// all lowering and final validation remains owned by `WeaveBuilder`.
    pub fn compose(
        id: impl Into<String>,
        compose: impl FnOnce(&mut crate::Composer) -> Result<(), crate::BuildError>,
    ) -> Result<Self, crate::ComposeError> {
        let mut composer = crate::Composer::new(id)?;
        compose(&mut composer)?;
        Ok(composer.build()?)
    }

    /// Author weave id (also mixed into Random PRNG seed at bind).
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Declared numeric path (must match the compiled feature).
    pub fn numeric(&self) -> NumericPath {
        self.numeric
    }

    /// Ordered knot definitions (dense runtime indices follow this order at bind).
    pub fn knots(&self) -> &[KnotDef] {
        &self.knots
    }

    /// Directed threads in author order.
    pub fn threads(&self) -> &[ThreadDef] {
        &self.threads
    }

    /// Clone into the serializable definition form.
    pub fn to_def(&self) -> WeaveDef {
        WeaveDef {
            id: self.id.clone(),
            numeric: self.numeric,
            knots: self.knots.clone(),
            threads: self.threads.clone(),
        }
    }

    pub(crate) fn from_validated(def: WeaveDef) -> Self {
        Self {
            id: def.id,
            numeric: def.numeric,
            knots: def.knots,
            threads: def.threads,
        }
    }
}

impl TryFrom<WeaveDef> for Weave {
    type Error = ValidationError;

    fn try_from(def: WeaveDef) -> Result<Self, Self::Error> {
        crate::authoring::validate::validate_def(&def)?;
        Ok(Self::from_validated(def))
    }
}

impl From<Weave> for WeaveDef {
    fn from(weave: Weave) -> Self {
        Self {
            id: weave.id,
            numeric: weave.numeric,
            knots: weave.knots,
            threads: weave.threads,
        }
    }
}
