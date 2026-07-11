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
    /// `validate_def` rejects an empty weave id.
    InvalidWeaveId {
        /// Author weave id from the definition.
        weave_id: String,
        /// Which naming rule the id violated.
        reason: &'static str,
    },
    /// `validate_def` or `Pattern::try_from` rejects a knot id that breaks naming rules.
    InvalidKnotId {
        /// Author knot id from the definition.
        knot_id: String,
        /// Which naming rule the id violated.
        reason: &'static str,
    },
    /// `validate_def` finds no knots in the weave definition.
    EmptyWeave {
        /// Author weave id of the empty definition.
        weave_id: String,
    },
    /// `validate_def` finds two knots with the same id.
    DuplicateKnotId {
        /// Knot id that appears more than once.
        knot_id: String,
    },
    /// A thread or pattern export references a knot id not present in the definition.
    UnknownKnot {
        /// Knot id that could not be resolved.
        knot_id: String,
    },
    /// A thread or pattern export names a port that the knot kind does not declare.
    UnknownPort {
        /// Knot that owns the referenced port.
        knot_id: String,
        /// Port name that is not in the knot catalog.
        port: String,
        /// Catalog port names valid for this knot kind.
        expected: Vec<String>,
    },
    /// A referenced port exists but is used as the wrong direction (in vs out).
    WrongPortDirection {
        /// Knot that owns the referenced port.
        knot_id: String,
        /// Port name used with the wrong direction.
        port: String,
        /// Direction required by the caller (thread endpoint or export check).
        expected: PortDir,
        /// Direction declared by the knot catalog for this port.
        actual: PortDir,
    },
    /// Two threads target the same input port on one knot.
    FanIn {
        /// Knot receiving multiple sources on one input.
        knot_id: String,
        /// Input port with more than one incoming thread.
        port: String,
    },
    /// The directed thread graph contains a cycle.
    Cycle {
        /// One knot on the cycle, when topological sort can identify it.
        at_knot: Option<String>,
    },
    /// A required input port has no thread and is not satisfied by a pattern export.
    UnconnectedRequired {
        /// Knot with the dangling required input.
        knot_id: String,
        /// Required input port left unwired.
        port: String,
    },
    /// `validate` or `validate_report` exceeds a hard resource budget.
    BudgetExceeded {
        /// Budget dimension that was exceeded (knots, threads, fan-out, etc.).
        metric: &'static str,
        /// Observed value that crossed the limit.
        actual: u32,
        /// Hard budget ceiling for this metric.
        limit: u32,
        /// Knot where the overrun was detected, when applicable.
        at_knot: Option<String>,
    },
    /// The weave's numeric path does not match the compiled crate feature.
    NumericMismatch {
        /// Numeric path required by the compiled build.
        expected: NumericPath,
        /// Numeric path stored on the weave definition.
        actual: NumericPath,
    },
    /// Connected ports or tied variable ports resolve to incompatible signal domains.
    SignalDomainMismatch {
        /// Source knot of the conflicting connection or port pair.
        from_knot: String,
        /// Source port involved in the mismatch.
        from_port: String,
        /// Domain inferred or fixed on the source side.
        from_domain: SignalDomain,
        /// Sink knot of the conflicting connection or port pair.
        to_knot: String,
        /// Sink port involved in the mismatch.
        to_port: String,
        /// Domain required or inferred on the sink side.
        to_domain: SignalDomain,
    },
    /// Active port domain inference could not fix a domain before validation finished.
    UnresolvedSignalDomain {
        /// Knot whose port domain stayed unknown.
        knot_id: String,
        /// Port that could not be assigned a signal domain.
        port: String,
    },
    /// A knot kind parameter or port constraint fails catalog rules.
    InvalidParameter {
        /// Knot carrying the invalid parameter value.
        knot_id: String,
        /// Parameter or port attribute that failed validation.
        parameter: &'static str,
        /// Why the parameter value or constraint is rejected.
        reason: &'static str,
    },
    /// Knot or thread count exceeds the dense `u16` representation limit.
    RepresentationOverflow {
        /// Representation resource kind (`knot` or `thread`).
        what: &'static str,
        /// Count present in the definition.
        actual: usize,
        /// Maximum storable in the dense weave representation.
        limit: usize,
    },
    /// `Pattern::try_from` rejects the pattern catalog id.
    InvalidPatternId {
        /// Pattern catalog id from the definition.
        pattern_id: String,
        /// Which naming rule the id violated.
        reason: &'static str,
    },
    /// `Pattern::try_from` finds two exports with the same name.
    DuplicateExport {
        /// Export name declared more than once.
        export: String,
    },
    /// Two input exports map to the same inner physical port.
    DuplicatePatternInput {
        /// Inner knot owning the doubly exported input.
        knot_id: String,
        /// Inner port exported under two names.
        port: String,
        /// First export name bound to this port.
        first_export: String,
        /// Second export name bound to the same port.
        duplicate_export: String,
    },
    /// An input export targets a port already wired inside the pattern.
    PatternInputAlreadyConnected {
        /// Export name that aliases an internally connected input.
        export: String,
        /// Inner knot already receiving an internal thread on this port.
        knot_id: String,
        /// Inner input port that is not available for external wiring.
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
    /// A weave, knot, or pattern instance id fails non-empty / character rules during authoring.
    InvalidId {
        /// Author id that failed naming rules.
        id: String,
        /// Which naming rule the id violated.
        reason: &'static str,
    },
    /// `WeaveBuilder::knot` or `include` would introduce a duplicate knot id.
    DuplicateKnotId {
        /// Knot id already present in the builder.
        knot_id: String,
    },
    /// A knot handle or port belongs to a different [`crate::WeaveBuilder`] than the current one.
    ForeignHandle,
    /// `input`, `output`, or `slot_of` names a port not declared for that knot kind.
    UnknownPort {
        /// Knot whose catalog was consulted.
        knot_id: String,
        /// Port name that is not declared for this kind.
        port: String,
        /// Catalog port names valid for this knot kind.
        expected: Vec<String>,
    },
    /// The named port exists but was requested with the wrong direction.
    WrongPortDirection {
        /// Knot that owns the referenced port.
        knot_id: String,
        /// Port name used with the wrong direction.
        port: String,
        /// Direction requested by the authoring call.
        expected: PortDir,
        /// Direction declared by the knot catalog for this port.
        actual: PortDir,
    },
    /// `PatternInstance::input` or `output` names an export not defined by the pattern.
    UnknownExport {
        /// Pattern instance prefix from `include`.
        instance_id: String,
        /// Export name missing from the pattern definition.
        export: String,
        /// Whether an input or output export was requested.
        direction: PortDir,
    },
    /// `include` finds the pattern's numeric path differs from the parent builder.
    NumericMismatch {
        /// Numeric path configured on the parent builder.
        expected: NumericPath,
        /// Numeric path stored on the included pattern.
        actual: NumericPath,
    },
    /// `connect` would join ports with incompatible fixed signal domains.
    SignalDomainMismatch {
        /// Source knot of the rejected connection.
        from_knot: String,
        /// Source port of the rejected connection.
        from_port: String,
        /// Fixed domain on the output side.
        from_domain: SignalDomain,
        /// Sink knot of the rejected connection.
        to_knot: String,
        /// Sink port of the rejected connection.
        to_port: String,
        /// Fixed domain required on the input side.
        to_domain: SignalDomain,
    },
    /// Authoring would exceed knot-index or owner-token representation limits.
    RepresentationOverflow {
        /// Representation resource kind (`knot` or builder owner token).
        what: &'static str,
        /// Count or token value that crossed the limit.
        actual: usize,
        /// Maximum storable in the authoring representation.
        limit: usize,
    },
    /// A fallible authoring step failed graph validation (via [`From<ValidationError>`]).
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
