//! Runtime-facing errors: bind failures, bad dense handles, cookbook wrappers.

use core::fmt;

use crate::authoring::{BuildError, ValidationError};
use crate::foundation::{PortSlot, SignalDomain};
use std::string::String;

use crate::runtime_impl::handles::{CmdId, HostPathId, KnotHandle, SenseId};

/// Failure while turning an authored [`crate::authoring::Weave`] into a runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BindError {
    /// The consumed weave failed graph validation.
    InvalidWeave {
        /// Author weave id passed to bind.
        weave_id: String,
        /// Structural validation failure from [`ValidationError`].
        source: ValidationError,
    },
    /// A validated weave exceeded a dense runtime representation.
    CapacityExceeded {
        /// Author weave id passed to bind.
        weave_id: String,
        /// Dense resource that overflowed (for example `"knot"`).
        resource: &'static str,
        /// Observed count that exceeded the runtime cap.
        count: usize,
    },
    /// Validated graph data could not be resolved during binding.
    InvalidReference {
        /// Author weave id passed to bind.
        weave_id: String,
        /// Knot id whose port could not be interned.
        knot: String,
        /// Catalog port name that failed resolution.
        port: String,
    },
    /// The validated topology could not be ordered.
    InvalidTopology {
        /// Author weave id passed to bind.
        weave_id: String,
    },
}

impl fmt::Display for BindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWeave { weave_id, source } => {
                write!(f, "cannot bind weave '{weave_id}': {source}")
            }
            Self::CapacityExceeded {
                weave_id,
                resource,
                count,
            } => write!(
                f,
                "cannot bind weave '{weave_id}': {resource} count {count} exceeds runtime capacity"
            ),
            Self::InvalidReference {
                weave_id,
                knot,
                port,
            } => write!(
                f,
                "cannot bind weave '{weave_id}': unresolved port '{knot}.{port}'"
            ),
            Self::InvalidTopology { weave_id } => {
                write!(f, "cannot bind weave '{weave_id}': invalid topology")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BindError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidWeave { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Failure caused by using a dense handle with the wrong runtime or port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HandleError {
    /// Dense handle belongs to a different [`Runtime`](crate::runtime_impl::bind::Runtime).
    ForeignRuntime {
        /// Human-readable handle kind (`"sense"`, `"host path"`, …).
        handle: &'static str,
    },
    /// [`SenseId`] is out of range or does not reference a `SignalIn` knot.
    InvalidSense {
        /// Rejected dense sense handle.
        sense: SenseId,
    },
    /// [`HostPathId`] is out of range for this runtime.
    InvalidHostPath {
        /// Rejected dense host-path handle.
        path: HostPathId,
    },
    /// [`CmdId`] is out of range for this runtime.
    InvalidCommand {
        /// Rejected dense command handle.
        cmd: CmdId,
    },
    /// [`KnotHandle`] is out of range for this runtime.
    InvalidKnot {
        /// Rejected dense knot handle.
        knot: KnotHandle,
    },
    /// [`PortSlot`] is invalid for the given knot in this runtime.
    InvalidPort {
        /// Knot that owns the rejected port slot.
        knot: KnotHandle,
        /// Rejected catalog port slot.
        port: PortSlot,
    },
    /// A host-authored sense value violates its declared signal domain.
    DomainValue {
        /// Sense port that rejected the write.
        sense: SenseId,
        /// Declared domain for that `SignalIn` knot.
        domain: SignalDomain,
    },
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignRuntime { handle } => {
                write!(f, "{handle} handle belongs to a different runtime")
            }
            Self::InvalidSense { sense } => {
                write!(
                    f,
                    "sense handle {} is invalid for this runtime",
                    sense.get()
                )
            }
            Self::InvalidHostPath { path } => {
                write!(f, "host path handle {} is invalid", path.get())
            }
            Self::InvalidCommand { cmd } => {
                write!(f, "command handle {} is invalid", cmd.get())
            }
            Self::InvalidKnot { knot } => {
                write!(f, "knot handle {} is invalid for this runtime", knot.get())
            }
            Self::InvalidPort { knot, port } => write!(
                f,
                "port handle {} is invalid for knot {} in this runtime",
                port.get(),
                knot.get()
            ),
            Self::DomainValue { sense, domain } => write!(
                f,
                "sense value for handle {} is invalid for {domain:?} domain",
                sense.get()
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for HandleError {}

/// The kind of named endpoint a [`crate::Recipe`] requires from a bound runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeEndpoint {
    /// A host-writable [`crate::SenseId`] backed by a `SignalIn` knot.
    SignalIn,
    /// An interned [`crate::HostPathId`] backed by a `SignalOut` knot.
    SignalOut,
    /// An interned [`crate::CmdId`] backed by an `EmitCommand` knot.
    EmitCommand,
    /// A named knot used for checked tooling access.
    Knot,
    /// A catalog port on a named knot.
    Port,
}

impl fmt::Display for RecipeEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SignalIn => f.write_str("SignalIn"),
            Self::SignalOut => f.write_str("SignalOut"),
            Self::EmitCommand => f.write_str("EmitCommand"),
            Self::Knot => f.write_str("knot"),
            Self::Port => f.write_str("port"),
        }
    }
}

/// Failure while resolving one of a recipe's required named endpoints.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecipeResolveError {
    /// The required endpoint was absent from the bound runtime.
    Missing {
        /// Required endpoint category.
        endpoint: RecipeEndpoint,
        /// Author knot id, host path, command name, or `knot.port` reference.
        name: String,
    },
    /// A named endpoint exists but does not satisfy the requested contract.
    Invalid {
        /// Required endpoint category.
        endpoint: RecipeEndpoint,
        /// Author knot id, host path, command name, or `knot.port` reference.
        name: String,
        /// Stable explanation of the incompatible endpoint.
        reason: &'static str,
    },
}

impl fmt::Display for RecipeResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { endpoint, name } => {
                write!(f, "required {endpoint} endpoint '{name}' is missing")
            }
            Self::Invalid {
                endpoint,
                name,
                reason,
            } => write!(
                f,
                "required {endpoint} endpoint '{name}' is invalid: {reason}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RecipeResolveError {}

/// Failure while constructing, binding, or resolving a [`crate::Recipe`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecipeError {
    /// The recipe could not construct its validated weave.
    Build(BuildError),
    /// The recipe's weave could not bind into a runtime.
    Bind(BindError),
    /// A required typed endpoint could not be resolved from the bound runtime.
    Resolve(RecipeResolveError),
}

impl From<BuildError> for RecipeError {
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}

impl From<BindError> for RecipeError {
    fn from(value: BindError) -> Self {
        Self::Bind(value)
    }
}

impl From<RecipeResolveError> for RecipeError {
    fn from(value: RecipeResolveError) -> Self {
        Self::Resolve(value)
    }
}

impl fmt::Display for RecipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build(source) => write!(f, "cannot build recipe: {source}"),
            Self::Bind(source) => write!(f, "cannot bind recipe: {source}"),
            Self::Resolve(source) => write!(f, "cannot resolve recipe ports: {source}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RecipeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Build(source) => Some(source),
            Self::Bind(source) => Some(source),
            Self::Resolve(source) => Some(source),
        }
    }
}

/// Failure while driving or asserting a typed [`crate::Scenario`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ScenarioError {
    /// Binding the recipe for the scenario failed.
    Recipe(RecipeError),
    /// A typed frame write or endpoint lookup used an invalid runtime handle.
    Handle(HandleError),
    /// An expectation ran before the scenario had produced a sample for the path.
    MissingSignal {
        /// Host path selected through the recipe's typed ports.
        path: String,
        /// Scenario frame that was being inspected.
        tick: u64,
    },
    /// A signal sample did not have the expected value.
    UnexpectedSignal {
        /// Host path selected through the recipe's typed ports.
        path: String,
        /// Expected sample.
        expected: crate::Signal,
        /// Sample produced by the runtime.
        actual: crate::Signal,
        /// Scenario frame that produced the sample.
        tick: u64,
    },
    /// A signal sample was present but falsey.
    ExpectedTruthy {
        /// Host path selected through the recipe's typed ports.
        path: String,
        /// Falsey sample produced by the runtime.
        actual: crate::Signal,
        /// Scenario frame that produced the sample.
        tick: u64,
    },
    /// A command was emitted a different number of times than expected.
    UnexpectedEmits {
        /// Command selected through the recipe's typed ports.
        command: String,
        /// Expected number of emits in the current frame.
        expected: usize,
        /// Actual number of emits in the current frame.
        actual: usize,
        /// Scenario frame that produced the emits.
        tick: u64,
    },
}

impl From<RecipeError> for ScenarioError {
    fn from(value: RecipeError) -> Self {
        Self::Recipe(value)
    }
}

impl From<HandleError> for ScenarioError {
    fn from(value: HandleError) -> Self {
        Self::Handle(value)
    }
}

impl fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Recipe(source) => write!(f, "cannot start scenario: {source}"),
            Self::Handle(source) => write!(f, "scenario handle error: {source}"),
            Self::MissingSignal { path, tick } => {
                write!(f, "SignalOut path '{path}' has no sample in scenario frame {tick}")
            }
            Self::UnexpectedSignal {
                path,
                expected,
                actual,
                tick,
            } => write!(
                f,
                "SignalOut path '{path}' in scenario frame {tick} was {actual:?}, expected {expected:?}"
            ),
            Self::ExpectedTruthy { path, actual, tick } => write!(
                f,
                "SignalOut path '{path}' in scenario frame {tick} was falsey ({actual:?})"
            ),
            Self::UnexpectedEmits {
                command,
                expected,
                actual,
                tick,
            } => write!(
                f,
                "EmitCommand '{command}' in scenario frame {tick} fired {actual} times, expected {expected}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ScenarioError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Recipe(source) => Some(source),
            Self::Handle(source) => Some(source),
            _ => None,
        }
    }
}

/// Error returned by executable cookbook recipes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CookbookError {
    /// Graph build failed before validation.
    Build(BuildError),
    /// Structural validation failed.
    Validation(ValidationError),
    /// Bind into a dense runtime failed.
    Bind(BindError),
    /// Dense handle misuse after bind.
    Handle(HandleError),
    /// Typed scenario setup, execution, or expectation failed.
    Scenario(ScenarioError),
}

impl From<BuildError> for CookbookError {
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}

impl From<ValidationError> for CookbookError {
    fn from(value: ValidationError) -> Self {
        Self::Validation(value)
    }
}

impl From<BindError> for CookbookError {
    fn from(value: BindError) -> Self {
        Self::Bind(value)
    }
}

impl From<HandleError> for CookbookError {
    fn from(value: HandleError) -> Self {
        Self::Handle(value)
    }
}

impl From<ScenarioError> for CookbookError {
    fn from(value: ScenarioError) -> Self {
        Self::Scenario(value)
    }
}

impl fmt::Display for CookbookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build(source) => write!(f, "cookbook graph build failed: {source}"),
            Self::Validation(source) => write!(f, "cookbook validation failed: {source}"),
            Self::Bind(source) => write!(f, "cookbook bind failed: {source}"),
            Self::Handle(source) => write!(f, "cookbook handle failed: {source}"),
            Self::Scenario(source) => write!(f, "cookbook scenario failed: {source}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CookbookError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Build(source) => Some(source),
            Self::Validation(source) => Some(source),
            Self::Bind(source) => Some(source),
            Self::Handle(source) => Some(source),
            Self::Scenario(source) => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::string::ToString;
    use std::vec::Vec;

    #[test]
    fn every_runtime_error_variant_has_a_stable_diagnostic_and_source_chain() {
        let validation = ValidationError::EmptyWeave {
            weave_id: String::from("empty"),
        };
        let bind_errors = [
            BindError::InvalidWeave {
                weave_id: String::from("invalid"),
                source: validation.clone(),
            },
            BindError::CapacityExceeded {
                weave_id: String::from("capacity"),
                resource: "knot",
                count: 257,
            },
            BindError::InvalidReference {
                weave_id: String::from("reference"),
                knot: String::from("target"),
                port: String::from("input"),
            },
            BindError::InvalidTopology {
                weave_id: String::from("cycle"),
            },
        ];
        let diagnostics: Vec<String> = bind_errors.iter().map(ToString::to_string).collect();
        assert_eq!(
            diagnostics,
            [
                "cannot bind weave 'invalid': weave 'empty' has no knots",
                "cannot bind weave 'capacity': knot count 257 exceeds runtime capacity",
                "cannot bind weave 'reference': unresolved port 'target.input'",
                "cannot bind weave 'cycle': invalid topology",
            ]
        );
        assert!(Error::source(&bind_errors[0]).is_some());
        assert!(bind_errors[1..]
            .iter()
            .all(|error| Error::source(error).is_none()));

        let owner = 7;
        let handles = [
            HandleError::ForeignRuntime { handle: "sense" },
            HandleError::InvalidSense {
                sense: SenseId::new(owner, 1),
            },
            HandleError::InvalidHostPath {
                path: HostPathId::new(owner, 2),
            },
            HandleError::InvalidCommand {
                cmd: CmdId::new(owner, 3),
            },
            HandleError::InvalidKnot {
                knot: KnotHandle::new(owner, 4),
            },
            HandleError::InvalidPort {
                knot: KnotHandle::new(owner, 5),
                port: PortSlot::new(6),
            },
            HandleError::DomainValue {
                sense: SenseId::new(owner, 7),
                domain: SignalDomain::Count,
            },
        ];
        let diagnostics: Vec<String> = handles.iter().map(ToString::to_string).collect();
        assert_eq!(
            diagnostics,
            [
                "sense handle belongs to a different runtime",
                "sense handle 1 is invalid for this runtime",
                "host path handle 2 is invalid",
                "command handle 3 is invalid",
                "knot handle 4 is invalid for this runtime",
                "port handle 6 is invalid for knot 5 in this runtime",
                "sense value for handle 7 is invalid for Count domain",
            ]
        );
        assert!(handles.iter().all(|error| Error::source(error).is_none()));

        let cookbook_errors = [
            CookbookError::from(BuildError::ForeignHandle),
            CookbookError::from(validation),
            CookbookError::from(bind_errors[0].clone()),
            CookbookError::from(handles[0]),
        ];
        let diagnostics: Vec<String> = cookbook_errors.iter().map(ToString::to_string).collect();
        assert_eq!(
            diagnostics,
            [
                "cookbook graph build failed: handle belongs to a different weave builder",
                "cookbook validation failed: weave 'empty' has no knots",
                "cookbook bind failed: cannot bind weave 'invalid': weave 'empty' has no knots",
                "cookbook handle failed: sense handle belongs to a different runtime",
            ]
        );
        assert!(cookbook_errors
            .iter()
            .all(|error| Error::source(error).is_some()));
    }
}
