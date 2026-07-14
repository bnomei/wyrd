//! Runtime-facing errors: bind failures, bad dense handles, and scenarios.

use core::fmt;

use crate::authoring::{BuildError, ValidationError};
use crate::foundation::{NumericPath, PortSlot, SignalDomain};
use std::boxed::Box;
use std::string::String;

use crate::runtime_impl::handles::{CmdId, HostPathId, KnotHandle, SenseId};

/// Failure while restoring an opaque [`RuntimeState`](crate::RuntimeState).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RestoreError {
    /// The snapshot uses a format this runtime cannot read.
    UnsupportedVersion {
        /// Version carried by the rejected snapshot.
        found: u32,
        /// Version accepted by this runtime.
        supported: u32,
    },
    /// The snapshot belongs to an incompatible immutable executable graph.
    FingerprintMismatch {
        /// Fingerprint of the currently bound executable runtime.
        expected: u64,
        /// Fingerprint carried by the rejected snapshot.
        found: u64,
    },
    /// The checkpoint was produced by the other compile-time signal path.
    NumericPathMismatch {
        /// Numeric path required by the receiving runtime.
        expected: NumericPath,
        /// Numeric path carried by the checkpoint.
        found: NumericPath,
    },
    /// The checkpoint was captured at a frame phase that cannot be restored.
    InvalidPhase {
        /// Opaque wire phase value.
        found: u8,
    },
    /// A stored signal does not satisfy its declared runtime domain.
    InvalidSignal {
        /// Checkpoint field that contained the invalid value.
        field: &'static str,
        /// Dense knot slot whose semantic contract rejected it.
        knot: usize,
        /// Declared signal domain.
        domain: SignalDomain,
    },
    /// A timer value is outside the duration authored for its knot.
    InvalidTimer {
        /// Dense timer knot slot.
        knot: usize,
        /// Stored remaining ticks.
        remaining: u16,
        /// Authored timer duration.
        max: u16,
    },
    /// A delay head is outside its immutable ring extent.
    InvalidDelayHead {
        /// Dense delay knot slot.
        knot: usize,
        /// Stored ring head.
        head: u16,
        /// Immutable ring length.
        len: u16,
    },
    /// The xorshift stream must never be zero.
    InvalidRng,
    /// A mutable snapshot buffer does not fit this runtime's bound shape.
    ShapeMismatch {
        /// Snapshot buffer whose length differs.
        field: &'static str,
        /// Length required by the bound runtime.
        expected: usize,
        /// Length carried by the rejected snapshot.
        found: usize,
    },
    /// An owner-free outbox id cannot be rebuilt for this runtime.
    InvalidHandleIndex {
        /// Outbox field containing the invalid dense index.
        field: &'static str,
        /// Owner-free dense index carried by the snapshot.
        index: u16,
        /// Number of valid local handles in the bound runtime.
        len: usize,
    },
}

impl fmt::Display for RestoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion { found, supported } => write!(
                f,
                "runtime snapshot format version {found} is unsupported (expected {supported})"
            ),
            Self::FingerprintMismatch { expected, found } => write!(
                f,
                "runtime snapshot fingerprint {found:016x} does not match {expected:016x}"
            ),
            Self::NumericPathMismatch { expected, found } => write!(
                f,
                "runtime checkpoint numeric path {found:?} does not match {expected:?}"
            ),
            Self::InvalidPhase { found } => write!(f, "runtime checkpoint phase {found} is invalid"),
            Self::InvalidSignal { field, knot, domain } => write!(
                f,
                "runtime checkpoint field '{field}' has an invalid {domain:?} signal at knot {knot}"
            ),
            Self::InvalidTimer { knot, remaining, max } => write!(
                f,
                "runtime checkpoint timer at knot {knot} has {remaining} ticks remaining, maximum is {max}"
            ),
            Self::InvalidDelayHead { knot, head, len } => write!(
                f,
                "runtime checkpoint delay at knot {knot} has head {head}, length is {len}"
            ),
            Self::InvalidRng => f.write_str("runtime checkpoint has an invalid zero RNG state"),
            Self::ShapeMismatch {
                field,
                expected,
                found,
            } => write!(
                f,
                "runtime snapshot field '{field}' has length {found}, expected {expected}"
            ),
            Self::InvalidHandleIndex { field, index, len } => write!(
                f,
                "runtime snapshot field '{field}' has invalid index {index} for length {len}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RestoreError {}

/// Failure while binding a fresh runtime from a durable checkpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BindRestoreError {
    /// The weave could not be bound.
    Bind(Box<BindError>),
    /// The freshly bound runtime rejected the checkpoint.
    Restore(RestoreError),
}

impl fmt::Display for BindRestoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind(error) => write!(f, "cannot bind restored runtime: {error}"),
            Self::Restore(error) => write!(f, "cannot restore runtime checkpoint: {error}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BindRestoreError {}

/// Failure while applying a named authored runtime preset.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PresetError {
    /// The weave could not be bound.
    Bind(Box<BindError>),
    /// More than one entry targeted the same authored knot.
    Duplicate {
        /// Authored knot name listed more than once in the preset.
        knot: String,
    },
    /// An entry named no knot in this weave.
    Missing {
        /// Authored knot name absent from the bound weave.
        knot: String,
    },
    /// An entry targeted an incompatible knot kind.
    WrongKind {
        /// Authored knot name that does not match the expected kind.
        knot: String,
        /// Expected knot kind label (for example `"Flag"` or `"Counter"`).
        expected: &'static str,
    },
    /// A SignalIn entry violated its declared signal domain.
    InvalidSignal {
        /// Authored SignalIn knot name carrying the invalid value.
        knot: String,
        /// Declared signal domain that rejected the preset value.
        domain: SignalDomain,
    },
}

impl fmt::Display for PresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind(error) => write!(f, "cannot bind preset runtime: {error}"),
            Self::Duplicate { knot } => write!(f, "runtime preset names '{knot}' more than once"),
            Self::Missing { knot } => write!(f, "runtime preset knot '{knot}' is missing"),
            Self::WrongKind { knot, expected } => {
                write!(f, "runtime preset knot '{knot}' is not a {expected}")
            }
            Self::InvalidSignal { knot, domain } => write!(
                f,
                "runtime preset signal '{knot}' is invalid for {domain:?}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PresetError {}

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
    Bind(Box<BindError>),
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
        Self::Bind(Box::new(value))
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
            Self::Bind(source) => Some(source.as_ref()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::string::ToString;
    use std::vec::Vec;

    #[test]
    fn every_runtime_error_variant_has_a_stable_diagnostic_and_source_chain() {
        let restore_errors = [
            RestoreError::UnsupportedVersion {
                found: 0,
                supported: 1,
            },
            RestoreError::FingerprintMismatch {
                expected: 1,
                found: 2,
            },
            RestoreError::ShapeMismatch {
                field: "counter",
                expected: 3,
                found: 4,
            },
            RestoreError::InvalidHandleIndex {
                field: "out_emits.command_index",
                index: 5,
                len: 6,
            },
        ];
        let diagnostics: Vec<String> = restore_errors.iter().map(ToString::to_string).collect();
        assert_eq!(
            diagnostics,
            [
                "runtime snapshot format version 0 is unsupported (expected 1)",
                "runtime snapshot fingerprint 0000000000000002 does not match 0000000000000001",
                "runtime snapshot field 'counter' has length 4, expected 3",
                "runtime snapshot field 'out_emits.command_index' has invalid index 5 for length 6",
            ]
        );
        assert!(restore_errors
            .iter()
            .all(|error| Error::source(error).is_none()));

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

        let endpoints = [
            (RecipeEndpoint::SignalIn, "SignalIn"),
            (RecipeEndpoint::SignalOut, "SignalOut"),
            (RecipeEndpoint::EmitCommand, "EmitCommand"),
            (RecipeEndpoint::Knot, "knot"),
            (RecipeEndpoint::Port, "port"),
        ];
        assert!(endpoints
            .iter()
            .all(|(endpoint, label)| endpoint.to_string() == *label));

        let resolve_errors = [
            RecipeResolveError::Missing {
                endpoint: RecipeEndpoint::SignalOut,
                name: String::from("door.open"),
            },
            RecipeResolveError::Invalid {
                endpoint: RecipeEndpoint::Port,
                name: String::from("door.in"),
                reason: "the port has the wrong domain",
            },
        ];
        assert_eq!(
            resolve_errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            [
                "required SignalOut endpoint 'door.open' is missing",
                "required port endpoint 'door.in' is invalid: the port has the wrong domain",
            ]
        );
        assert!(resolve_errors
            .iter()
            .all(|error| Error::source(error).is_none()));

        let recipe_errors = [
            RecipeError::from(BuildError::ForeignHandle),
            RecipeError::from(bind_errors[0].clone()),
            RecipeError::from(resolve_errors[0].clone()),
        ];
        assert_eq!(
            recipe_errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            [
                "cannot build recipe: handle belongs to a different weave builder",
                "cannot bind recipe: cannot bind weave 'invalid': weave 'empty' has no knots",
                "cannot resolve recipe ports: required SignalOut endpoint 'door.open' is missing",
            ]
        );
        assert!(recipe_errors
            .iter()
            .all(|error| Error::source(error).is_some()));

        let scenario_errors = [
            ScenarioError::from(recipe_errors[0].clone()),
            ScenarioError::from(HandleError::ForeignRuntime { handle: "sense" }),
            ScenarioError::MissingSignal {
                path: String::from("door.open"),
                tick: 4,
            },
            ScenarioError::UnexpectedSignal {
                path: String::from("door.open"),
                expected: crate::ONE,
                actual: crate::ZERO,
                tick: 5,
            },
            ScenarioError::ExpectedTruthy {
                path: String::from("door.open"),
                actual: crate::ZERO,
                tick: 6,
            },
            ScenarioError::UnexpectedEmits {
                command: String::from("chime"),
                expected: 2,
                actual: 1,
                tick: 7,
            },
        ];
        let diagnostics: Vec<String> = scenario_errors.iter().map(ToString::to_string).collect();
        assert!(diagnostics[0].starts_with("cannot start scenario: cannot build recipe"));
        assert!(diagnostics[1].starts_with("scenario handle error: sense"));
        assert!(diagnostics[2].contains("no sample in scenario frame 4"));
        assert!(diagnostics[3].contains("was") && diagnostics[3].contains("expected"));
        assert!(diagnostics[4].contains("was falsey"));
        assert!(diagnostics[5].contains("fired 1 times, expected 2"));
        assert!(Error::source(&scenario_errors[0]).is_some());
        assert!(Error::source(&scenario_errors[1]).is_some());
        assert!(scenario_errors[2..]
            .iter()
            .all(|error| Error::source(error).is_none()));
    }
}
