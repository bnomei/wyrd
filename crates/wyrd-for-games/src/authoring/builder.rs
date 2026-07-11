//! Typed incremental authoring of a [`Weave`] with owner-scoped handles.
//!
//! Handles and ports are tied to one [`WeaveBuilder`] instance so foreign
//! endpoints cannot be connected across builders. Endpoint resolution checks
//! catalog port names and directions before threads are recorded. Final
//! structural checks (cycles, fan-in, budgets) run in [`WeaveBuilder::build`].

use core::sync::atomic::{AtomicUsize, Ordering};

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

use crate::foundation::{
    port_domain, port_slot, ports_of, KnotKind, NumericPath, PortDir, PortDomain, PortSlot,
};

use crate::authoring::pattern::{expand, Pattern};
use crate::{BuildError, KnotDef, PortRefDef, ThreadDef, ValidationError, Weave, WeaveDef};

static NEXT_OWNER: AtomicUsize = AtomicUsize::new(1);

/// Opaque knot reference valid only for the builder that created it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct KnotHandle {
    owner: usize,
    index: u16,
}

/// Catalog-checked input endpoint for connecting a thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputPort {
    owner: usize,
    knot: u16,
    name: String,
}

/// Catalog-checked output endpoint for connecting a thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputPort {
    owner: usize,
    knot: u16,
    name: String,
}

/// Expanded pattern include: export names map to parent-builder ports.
#[derive(Clone, Debug)]
pub struct PatternInstance {
    id: String,
    inputs: BTreeMap<String, InputPort>,
    outputs: BTreeMap<String, OutputPort>,
}

impl PatternInstance {
    /// Instance id used as the expansion prefix (`id/inner_knot`).
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Resolve a pattern input export to a parent-builder input port.
    pub fn input(&self, export: &str) -> Result<InputPort, BuildError> {
        self.inputs
            .get(export)
            .cloned()
            .ok_or_else(|| BuildError::UnknownExport {
                instance_id: self.id.clone(),
                export: String::from(export),
                direction: PortDir::In,
            })
    }

    /// Resolve a pattern output export to a parent-builder output port.
    pub fn output(&self, export: &str) -> Result<OutputPort, BuildError> {
        self.outputs
            .get(export)
            .cloned()
            .ok_or_else(|| BuildError::UnknownExport {
                instance_id: self.id.clone(),
                export: String::from(export),
                direction: PortDir::Out,
            })
    }
}

/// Incremental weave construction; [`Self::build`] validates into a [`Weave`].
pub struct WeaveBuilder {
    owner: usize,
    id: String,
    knots: Vec<KnotDef>,
    threads: Vec<ThreadDef>,
    numeric: NumericPath,
    names: BTreeMap<String, u16>,
}

impl WeaveBuilder {
    /// Start a weave with a non-empty author id and a fresh owner token.
    pub fn new(id: impl Into<String>) -> Result<Self, BuildError> {
        let id = id.into();
        if id.is_empty() {
            return Err(BuildError::InvalidId {
                id,
                reason: "weave ids must be non-empty",
            });
        }
        let owner = NEXT_OWNER.fetch_add(1, Ordering::Relaxed);
        validate_owner(owner)?;
        Ok(Self {
            owner,
            id,
            knots: Vec::new(),
            threads: Vec::new(),
            numeric: NumericPath::compiled(),
            names: BTreeMap::new(),
        })
    }

    /// Override the numeric path tag (must match the compiled feature at validate).
    pub fn set_numeric(&mut self, path: NumericPath) -> Result<&mut Self, BuildError> {
        self.numeric = path;
        Ok(self)
    }

    /// Add a knot with a unique non-empty author id.
    pub fn knot(
        &mut self,
        id: impl Into<String>,
        kind: KnotKind,
    ) -> Result<KnotHandle, BuildError> {
        let id = id.into();
        if id.is_empty() {
            return Err(BuildError::InvalidId {
                id,
                reason: "knot ids must be non-empty",
            });
        }
        if self.names.contains_key(&id) {
            return Err(BuildError::DuplicateKnotId { knot_id: id });
        }
        let index =
            u16::try_from(self.knots.len()).map_err(|_| BuildError::RepresentationOverflow {
                what: "knot",
                actual: self.knots.len(),
                limit: u16::MAX as usize,
            })?;
        self.names.insert(id.clone(), index);
        self.knots.push(KnotDef { id, kind });
        Ok(KnotHandle {
            owner: self.owner,
            index,
        })
    }

    /// Catalog-checked input port on a knot owned by this builder.
    pub fn input(&self, knot: &KnotHandle, name: &str) -> Result<InputPort, BuildError> {
        self.endpoint(knot, name, PortDir::In)
            .map(|name| InputPort {
                owner: self.owner,
                knot: knot.index,
                name,
            })
    }

    /// Catalog-checked output port on a knot owned by this builder.
    pub fn output(&self, knot: &KnotHandle, name: &str) -> Result<OutputPort, BuildError> {
        self.endpoint(knot, name, PortDir::Out)
            .map(|name| OutputPort {
                owner: self.owner,
                knot: knot.index,
                name,
            })
    }

    /// Record a directed thread; both endpoints must belong to this builder.
    pub fn connect(&mut self, from: OutputPort, to: InputPort) -> Result<&mut Self, BuildError> {
        if from.owner != self.owner || to.owner != self.owner {
            return Err(BuildError::ForeignHandle);
        }
        let from_knot = self
            .knots
            .get(from.knot as usize)
            .ok_or(BuildError::ForeignHandle)?;
        let to_knot = self
            .knots
            .get(to.knot as usize)
            .ok_or(BuildError::ForeignHandle)?;
        if let (Some(from_domain), Some(to_domain)) = (
            fixed_port_domain(&from_knot.kind, &from.name),
            fixed_port_domain(&to_knot.kind, &to.name),
        ) {
            if from_domain != to_domain {
                return Err(BuildError::SignalDomainMismatch {
                    from_knot: from_knot.id.clone(),
                    from_port: from.name,
                    from_domain,
                    to_knot: to_knot.id.clone(),
                    to_port: to.name,
                    to_domain,
                });
            }
        }
        self.threads.push(ThreadDef {
            from: PortRefDef::new(from_knot.id.clone(), from.name),
            to: PortRefDef::new(to_knot.id.clone(), to.name),
        });
        Ok(self)
    }

    /// Expand a validated [`Pattern`] under `instance_id/` and return export ports.
    pub fn include(
        &mut self,
        instance_id: impl Into<String>,
        pattern: &Pattern,
    ) -> Result<PatternInstance, BuildError> {
        let instance_id = instance_id.into();
        if pattern.inner().numeric != self.numeric {
            return Err(BuildError::NumericMismatch {
                expected: self.numeric,
                actual: pattern.inner().numeric,
            });
        }
        let expanded = expand(&instance_id, pattern)?;
        for knot in &expanded.knots {
            if self.names.contains_key(&knot.id) {
                return Err(BuildError::DuplicateKnotId {
                    knot_id: knot.id.clone(),
                });
            }
        }
        for knot in expanded.knots {
            let index = u16::try_from(self.knots.len()).map_err(|_| {
                BuildError::RepresentationOverflow {
                    what: "knot",
                    actual: self.knots.len(),
                    limit: u16::MAX as usize,
                }
            })?;
            self.names.insert(knot.id.clone(), index);
            self.knots.push(knot);
        }
        self.threads.extend(expanded.threads);
        let inputs = expanded
            .inputs
            .into_iter()
            .map(|(name, port)| {
                let knot = self.names[port.knot.as_str()];
                (
                    name,
                    InputPort {
                        owner: self.owner,
                        knot,
                        name: port.port,
                    },
                )
            })
            .collect();
        let outputs = expanded
            .outputs
            .into_iter()
            .map(|(name, port)| {
                let knot = self.names[port.knot.as_str()];
                (
                    name,
                    OutputPort {
                        owner: self.owner,
                        knot,
                        name: port.port,
                    },
                )
            })
            .collect();
        Ok(PatternInstance {
            id: instance_id,
            inputs,
            outputs,
        })
    }

    /// Validate structure and produce an immutable [`Weave`].
    pub fn build(self) -> Result<Weave, ValidationError> {
        Weave::try_from(WeaveDef {
            id: self.id,
            numeric: self.numeric,
            knots: self.knots,
            threads: self.threads,
        })
    }

    fn endpoint(
        &self,
        knot: &KnotHandle,
        name: &str,
        expected: PortDir,
    ) -> Result<String, BuildError> {
        if knot.owner != self.owner {
            return Err(BuildError::ForeignHandle);
        }
        let knot_def = self
            .knots
            .get(knot.index as usize)
            .ok_or(BuildError::ForeignHandle)?;
        let ports = ports_of(&knot_def.kind);
        let info =
            ports
                .iter()
                .find(|port| port.name == name)
                .ok_or_else(|| BuildError::UnknownPort {
                    knot_id: knot_def.id.clone(),
                    port: String::from(name),
                    expected: ports.iter().map(|port| String::from(port.name)).collect(),
                })?;
        if info.dir != expected {
            return Err(BuildError::WrongPortDirection {
                knot_id: knot_def.id.clone(),
                port: String::from(name),
                expected,
                actual: info.dir,
            });
        }
        Ok(String::from(name))
    }
}

fn validate_owner(owner: usize) -> Result<(), BuildError> {
    if owner == usize::MAX {
        return Err(BuildError::RepresentationOverflow {
            what: "builder owner token",
            actual: owner,
            limit: usize::MAX - 1,
        });
    }
    Ok(())
}

fn fixed_port_domain(kind: &KnotKind, name: &str) -> Option<crate::foundation::SignalDomain> {
    let slot = port_slot(kind, name)?;
    match port_domain(kind, slot)? {
        PortDomain::Fixed(domain) => Some(domain),
        PortDomain::Variable(_) | PortDomain::Any => None,
    }
}

/// Resolve a catalog port name to its compact slot.
pub fn slot_of(kind: &KnotKind, name: &str) -> Result<PortSlot, BuildError> {
    port_slot(kind, name).ok_or_else(|| BuildError::UnknownPort {
        knot_id: String::from("<kind>"),
        port: String::from(name),
        expected: ports_of(kind)
            .iter()
            .map(|port| String::from(port.name))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::validate_owner;
    use crate::BuildError;

    #[test]
    fn owner_tokens_reserve_the_overflow_sentinel() {
        assert!(matches!(
            validate_owner(usize::MAX),
            Err(BuildError::RepresentationOverflow {
                what: "builder owner token",
                actual: usize::MAX,
                limit,
            }) if limit == usize::MAX - 1
        ));
        assert!(validate_owner(1).is_ok());
    }
}
