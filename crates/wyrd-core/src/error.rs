use core::fmt;

/// Library error. Settle should not panic; bind/validate may return this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WyrdError {
    /// Generic message for early scaffolding (replace with structured variants).
    Msg(&'static str),
    UnknownKnot,
    UnknownPort,
    UnknownPath,
    DuplicateKnotId,
    Cycle,
    FanIn,
    UnconnectedRequired,
    Budget,
    NumericMismatch,
    Empty,
}

impl fmt::Display for WyrdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WyrdError::Msg(s) => f.write_str(s),
            WyrdError::UnknownKnot => f.write_str("unknown knot"),
            WyrdError::UnknownPort => f.write_str("unknown port"),
            WyrdError::UnknownPath => f.write_str("unknown host path"),
            WyrdError::DuplicateKnotId => f.write_str("duplicate knot id"),
            WyrdError::Cycle => f.write_str("cycle in weave"),
            WyrdError::FanIn => f.write_str("fan-in > 1 on input port"),
            WyrdError::UnconnectedRequired => f.write_str("required port unconnected"),
            WyrdError::Budget => f.write_str("budget exceeded"),
            WyrdError::NumericMismatch => f.write_str("numeric path mismatch"),
            WyrdError::Empty => f.write_str("empty weave"),
        }
    }
}

pub type Result<T> = core::result::Result<T, WyrdError>;
