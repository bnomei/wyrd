//! Reusable graph fragments: validated patterns expand into parent weaves.
//!
//! A [`Pattern`] is an immutable inner weave plus named input/output exports.
//! Include renames knots under `instance_id/` so multiple instances do not
//! collide. Exported inputs may leave required ports unconnected in the inner
//! graph; the parent must wire them after expand.

use std::collections::{BTreeMap, BTreeSet};
use std::string::String;
use std::vec::Vec;

use wyrd_core::{ports_of, PortDir};

use crate::{KnotDef, PortRefDef, ThreadDef, ValidationError, WeaveDef};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Named export of an inner knot port for parent-weave wiring.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PatternExportDef {
    pub name: String,
    pub port: PortRefDef,
}

impl PatternExportDef {
    pub fn new(name: impl Into<String>, knot: impl Into<String>, port: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            port: PortRefDef::new(knot, port),
        }
    }
}

/// Editable pattern definition; convert with [`Pattern::try_from`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PatternDef {
    pub id: String,
    pub inner: WeaveDef,
    pub inputs: Vec<PatternExportDef>,
    pub outputs: Vec<PatternExportDef>,
}

/// Immutable, validated reusable graph fragment.
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    id: String,
    inner: WeaveDef,
    inputs: Vec<PatternExportDef>,
    outputs: Vec<PatternExportDef>,
}

impl Pattern {
    /// Pattern catalog id (no `/` in the id).
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Inner weave definition (knot ids must not contain `/` either).
    pub fn inner(&self) -> &WeaveDef {
        &self.inner
    }

    /// Input exports the parent must wire after include.
    pub fn inputs(&self) -> &[PatternExportDef] {
        &self.inputs
    }

    /// Output exports available as parent sources.
    pub fn outputs(&self) -> &[PatternExportDef] {
        &self.outputs
    }

    /// Clone into the serializable definition form.
    pub fn to_def(&self) -> PatternDef {
        PatternDef {
            id: self.id.clone(),
            inner: self.inner.clone(),
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
        }
    }
}

impl TryFrom<PatternDef> for Pattern {
    type Error = ValidationError;

    fn try_from(def: PatternDef) -> Result<Self, Self::Error> {
        if def.id.is_empty() || def.id.contains('/') {
            return Err(ValidationError::InvalidPatternId {
                pattern_id: def.id,
                reason: "must be non-empty and contain no slash",
            });
        }
        if let Some(knot) = def.inner.knots.iter().find(|knot| knot.id.contains('/')) {
            return Err(ValidationError::InvalidKnotId {
                knot_id: knot.id.clone(),
                reason: "pattern inner knot ids must contain no slash",
            });
        }
        let index: BTreeMap<&str, &KnotDef> =
            def.inner.knots.iter().map(|k| (k.id.as_str(), k)).collect();
        let mut names = BTreeSet::new();
        let mut external = BTreeSet::new();
        let internally_connected: BTreeSet<(String, String)> = def
            .inner
            .threads
            .iter()
            .map(|thread| (thread.to.knot.clone(), thread.to.port.clone()))
            .collect();
        let mut physical_inputs: BTreeMap<(String, String), String> = BTreeMap::new();
        for export in &def.inputs {
            if !names.insert(export.name.as_str()) {
                return Err(ValidationError::DuplicateExport {
                    export: export.name.clone(),
                });
            }
            check_export(&index, export, PortDir::In)?;
            let endpoint = (export.port.knot.clone(), export.port.port.clone());
            if internally_connected.contains(&endpoint) {
                return Err(ValidationError::PatternInputAlreadyConnected {
                    export: export.name.clone(),
                    knot_id: export.port.knot.clone(),
                    port: export.port.port.clone(),
                });
            }
            if let Some(first_export) =
                physical_inputs.insert(endpoint.clone(), export.name.clone())
            {
                return Err(ValidationError::DuplicatePatternInput {
                    knot_id: endpoint.0,
                    port: endpoint.1,
                    first_export,
                    duplicate_export: export.name.clone(),
                });
            }
            external.insert(endpoint);
        }
        names.clear();
        for export in &def.outputs {
            if !names.insert(export.name.as_str()) {
                return Err(ValidationError::DuplicateExport {
                    export: export.name.clone(),
                });
            }
            check_export(&index, export, PortDir::Out)?;
        }
        crate::validate::validate_def_with_external_inputs(&def.inner, &external)?;
        Ok(Self {
            id: def.id,
            inner: def.inner,
            inputs: def.inputs,
            outputs: def.outputs,
        })
    }
}

impl From<Pattern> for PatternDef {
    fn from(pattern: Pattern) -> Self {
        Self {
            id: pattern.id,
            inner: pattern.inner,
            inputs: pattern.inputs,
            outputs: pattern.outputs,
        }
    }
}

fn check_export(
    index: &BTreeMap<&str, &KnotDef>,
    export: &PatternExportDef,
    expected: PortDir,
) -> Result<(), ValidationError> {
    let knot =
        index
            .get(export.port.knot.as_str())
            .ok_or_else(|| ValidationError::UnknownKnot {
                knot_id: export.port.knot.clone(),
            })?;
    let ports = ports_of(&knot.kind);
    let info = ports
        .iter()
        .find(|p| p.name == export.port.port)
        .ok_or_else(|| ValidationError::UnknownPort {
            knot_id: knot.id.clone(),
            port: export.port.port.clone(),
            expected: ports.iter().map(|p| String::from(p.name)).collect(),
        })?;
    if info.dir != expected {
        return Err(ValidationError::WrongPortDirection {
            knot_id: knot.id.clone(),
            port: export.port.port.clone(),
            expected,
            actual: info.dir,
        });
    }
    Ok(())
}

pub(crate) struct ExpandedPattern {
    pub knots: Vec<KnotDef>,
    pub threads: Vec<ThreadDef>,
    pub inputs: BTreeMap<String, PortRefDef>,
    pub outputs: BTreeMap<String, PortRefDef>,
}

pub(crate) fn expand(
    instance_id: &str,
    pattern: &Pattern,
) -> Result<ExpandedPattern, crate::BuildError> {
    if instance_id.is_empty() || instance_id.contains('/') {
        return Err(crate::BuildError::InvalidId {
            id: String::from(instance_id),
            reason: "pattern instance ids must be non-empty and contain no slash",
        });
    }
    let prefix = format_prefix(instance_id);
    let knots = pattern
        .inner
        .knots
        .iter()
        .map(|knot| KnotDef {
            id: prefixed(&prefix, &knot.id),
            kind: knot.kind.clone(),
        })
        .collect();
    let threads = pattern
        .inner
        .threads
        .iter()
        .map(|thread| ThreadDef {
            from: PortRefDef::new(
                prefixed(&prefix, &thread.from.knot),
                thread.from.port.clone(),
            ),
            to: PortRefDef::new(prefixed(&prefix, &thread.to.knot), thread.to.port.clone()),
        })
        .collect();
    let inputs = pattern
        .inputs
        .iter()
        .map(|export| {
            (
                export.name.clone(),
                PortRefDef::new(
                    prefixed(&prefix, &export.port.knot),
                    export.port.port.clone(),
                ),
            )
        })
        .collect();
    let outputs = pattern
        .outputs
        .iter()
        .map(|export| {
            (
                export.name.clone(),
                PortRefDef::new(
                    prefixed(&prefix, &export.port.knot),
                    export.port.port.clone(),
                ),
            )
        })
        .collect();
    Ok(ExpandedPattern {
        knots,
        threads,
        inputs,
        outputs,
    })
}

fn format_prefix(instance_id: &str) -> String {
    let mut value = String::from(instance_id);
    value.push('/');
    value
}
fn prefixed(prefix: &str, knot: &str) -> String {
    let mut value = String::from(prefix);
    value.push_str(knot);
    value
}
