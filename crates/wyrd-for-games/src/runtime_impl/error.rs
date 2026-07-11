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
        weave_id: String,
        source: ValidationError,
    },
    /// A validated weave exceeded a dense runtime representation.
    CapacityExceeded {
        weave_id: String,
        resource: &'static str,
        count: usize,
    },
    /// Validated graph data could not be resolved during binding.
    InvalidReference {
        weave_id: String,
        knot: String,
        port: String,
    },
    /// The validated topology could not be ordered.
    InvalidTopology { weave_id: String },
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
    ForeignRuntime {
        handle: &'static str,
    },
    InvalidSense {
        sense: SenseId,
    },
    InvalidHostPath {
        path: HostPathId,
    },
    InvalidCommand {
        cmd: CmdId,
    },
    InvalidKnot {
        knot: KnotHandle,
    },
    InvalidPort {
        knot: KnotHandle,
        port: PortSlot,
    },
    /// A host-authored sense value violates its declared signal domain.
    DomainValue {
        sense: SenseId,
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

/// Error returned by executable cookbook recipes.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CookbookError {
    Build(BuildError),
    Validation(ValidationError),
    Bind(BindError),
    Handle(HandleError),
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

impl fmt::Display for CookbookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build(source) => write!(f, "cookbook graph build failed: {source}"),
            Self::Validation(source) => write!(f, "cookbook validation failed: {source}"),
            Self::Bind(source) => write!(f, "cookbook bind failed: {source}"),
            Self::Handle(source) => write!(f, "cookbook handle failed: {source}"),
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
        }
    }
}
