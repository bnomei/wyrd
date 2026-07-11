//! Build-time and validate-time errors for author graphs.
//!
//! [`ValidationError`] means a definition cannot become a [`crate::Weave`].
//! [`BuildError`] covers handle misuse and authoring mistakes before that final
//! validation step (including wrapped validation failures from `build`).

use core::fmt;

use std::string::String;
use std::vec::Vec;

use crate::foundation::{NumericPath, PortDir, SignalDomain};

/// A graph definition is structurally invalid and cannot become a [`crate::Weave`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationError {
    InvalidWeaveId {
        weave_id: String,
        reason: &'static str,
    },
    InvalidKnotId {
        knot_id: String,
        reason: &'static str,
    },
    EmptyWeave {
        weave_id: String,
    },
    DuplicateKnotId {
        knot_id: String,
    },
    UnknownKnot {
        knot_id: String,
    },
    UnknownPort {
        knot_id: String,
        port: String,
        expected: Vec<String>,
    },
    WrongPortDirection {
        knot_id: String,
        port: String,
        expected: PortDir,
        actual: PortDir,
    },
    FanIn {
        knot_id: String,
        port: String,
    },
    Cycle {
        at_knot: Option<String>,
    },
    UnconnectedRequired {
        knot_id: String,
        port: String,
    },
    BudgetExceeded {
        metric: &'static str,
        actual: u32,
        limit: u32,
        at_knot: Option<String>,
    },
    NumericMismatch {
        expected: NumericPath,
        actual: NumericPath,
    },
    SignalDomainMismatch {
        from_knot: String,
        from_port: String,
        from_domain: SignalDomain,
        to_knot: String,
        to_port: String,
        to_domain: SignalDomain,
    },
    UnresolvedSignalDomain {
        knot_id: String,
        port: String,
    },
    InvalidParameter {
        knot_id: String,
        parameter: &'static str,
        reason: &'static str,
    },
    RepresentationOverflow {
        what: &'static str,
        actual: usize,
        limit: usize,
    },
    InvalidPatternId {
        pattern_id: String,
        reason: &'static str,
    },
    DuplicateExport {
        export: String,
    },
    DuplicatePatternInput {
        knot_id: String,
        port: String,
        first_export: String,
        duplicate_export: String,
    },
    PatternInputAlreadyConnected {
        export: String,
        knot_id: String,
        port: String,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWeaveId { weave_id, reason } => {
                write!(f, "invalid weave id '{weave_id}': {reason}")
            }
            Self::InvalidKnotId { knot_id, reason } => {
                write!(f, "invalid knot id '{knot_id}': {reason}")
            }
            Self::EmptyWeave { weave_id } => write!(f, "weave '{weave_id}' has no knots"),
            Self::DuplicateKnotId { knot_id } => write!(f, "duplicate knot id '{knot_id}'"),
            Self::UnknownKnot { knot_id } => write!(f, "unknown knot '{knot_id}'"),
            Self::UnknownPort {
                knot_id,
                port,
                expected,
            } => write!(
                f,
                "unknown port '{knot_id}.{port}'; expected one of {}",
                Join(expected)
            ),
            Self::WrongPortDirection {
                knot_id,
                port,
                expected,
                actual,
            } => write!(
                f,
                "port '{knot_id}.{port}' is {actual:?}, expected {expected:?}"
            ),
            Self::FanIn { knot_id, port } => {
                write!(f, "input '{knot_id}.{port}' has more than one source")
            }
            Self::Cycle {
                at_knot: Some(knot),
            } => write!(f, "cycle in weave at knot '{knot}'"),
            Self::Cycle { at_knot: None } => f.write_str("cycle in weave"),
            Self::UnconnectedRequired { knot_id, port } => {
                write!(f, "required input '{knot_id}.{port}' is unconnected")
            }
            Self::BudgetExceeded {
                metric,
                actual,
                limit,
                at_knot,
            } => {
                write!(f, "{metric} budget exceeded: {actual} > {limit}")?;
                if let Some(knot) = at_knot {
                    write!(f, " at knot '{knot}'")?;
                }
                Ok(())
            }
            Self::NumericMismatch { expected, actual } => write!(
                f,
                "numeric path mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::SignalDomainMismatch {
                from_knot,
                from_port,
                from_domain,
                to_knot,
                to_port,
                to_domain,
            } => write!(
                f,
                "signal domain mismatch: '{from_knot}.{from_port}' is {from_domain:?}, but '{to_knot}.{to_port}' requires {to_domain:?}"
            ),
            Self::UnresolvedSignalDomain { knot_id, port } => write!(
                f,
                "signal domain for '{knot_id}.{port}' could not be resolved"
            ),
            Self::InvalidParameter {
                knot_id,
                parameter,
                reason,
            } => write!(
                f,
                "invalid parameter '{parameter}' on knot '{knot_id}': {reason}"
            ),
            Self::RepresentationOverflow {
                what,
                actual,
                limit,
            } => write!(
                f,
                "{what} count {actual} exceeds representation limit {limit}"
            ),
            Self::InvalidPatternId { pattern_id, reason } => {
                write!(f, "invalid pattern id '{pattern_id}': {reason}")
            }
            Self::DuplicateExport { export } => write!(f, "duplicate pattern export '{export}'"),
            Self::DuplicatePatternInput {
                knot_id,
                port,
                first_export,
                duplicate_export,
            } => write!(
                f,
                "pattern input '{knot_id}.{port}' is exported twice as '{first_export}' and '{duplicate_export}'"
            ),
            Self::PatternInputAlreadyConnected {
                export,
                knot_id,
                port,
            } => write!(
                f,
                "pattern input export '{export}' targets internally connected input '{knot_id}.{port}'"
            ),
        }
    }
}

/// An authoring operation failed before final graph validation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildError {
    InvalidId {
        id: String,
        reason: &'static str,
    },
    DuplicateKnotId {
        knot_id: String,
    },
    ForeignHandle,
    UnknownPort {
        knot_id: String,
        port: String,
        expected: Vec<String>,
    },
    WrongPortDirection {
        knot_id: String,
        port: String,
        expected: PortDir,
        actual: PortDir,
    },
    UnknownExport {
        instance_id: String,
        export: String,
        direction: PortDir,
    },
    NumericMismatch {
        expected: NumericPath,
        actual: NumericPath,
    },
    SignalDomainMismatch {
        from_knot: String,
        from_port: String,
        from_domain: SignalDomain,
        to_knot: String,
        to_port: String,
        to_domain: SignalDomain,
    },
    RepresentationOverflow {
        what: &'static str,
        actual: usize,
        limit: usize,
    },
    Validation(ValidationError),
}

impl From<ValidationError> for BuildError {
    fn from(value: ValidationError) -> Self {
        Self::Validation(value)
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId { id, reason } => write!(f, "invalid id '{id}': {reason}"),
            Self::DuplicateKnotId { knot_id } => write!(f, "duplicate knot id '{knot_id}'"),
            Self::ForeignHandle => f.write_str("handle belongs to a different weave builder"),
            Self::UnknownPort {
                knot_id,
                port,
                expected,
            } => write!(
                f,
                "unknown port '{knot_id}.{port}'; expected one of {}",
                Join(expected)
            ),
            Self::WrongPortDirection {
                knot_id,
                port,
                expected,
                actual,
            } => write!(
                f,
                "port '{knot_id}.{port}' is {actual:?}, expected {expected:?}"
            ),
            Self::UnknownExport {
                instance_id,
                export,
                direction,
            } => write!(
                f,
                "unknown {direction:?} export '{export}' on pattern instance '{instance_id}'"
            ),
            Self::NumericMismatch { expected, actual } => write!(
                f,
                "numeric path mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::SignalDomainMismatch {
                from_knot,
                from_port,
                from_domain,
                to_knot,
                to_port,
                to_domain,
            } => write!(
                f,
                "signal domain mismatch: '{from_knot}.{from_port}' is {from_domain:?}, but '{to_knot}.{to_port}' requires {to_domain:?}"
            ),
            Self::RepresentationOverflow {
                what,
                actual,
                limit,
            } => write!(
                f,
                "{what} count {actual} exceeds representation limit {limit}"
            ),
            Self::Validation(error) => error.fmt(f),
        }
    }
}

struct Join<'a>(&'a [String]);
impl fmt::Display for Join<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, item) in self.0.iter().enumerate() {
            if index != 0 {
                f.write_str(", ")?;
            }
            f.write_str(item)?;
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ValidationError {}

#[cfg(feature = "std")]
impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Validation(error) => Some(error),
            _ => None,
        }
    }
}
