use core::fmt;

/// Library error. Settle should not panic; bind/validate may return this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WyrdError {
    /// Stable static message (parse / rare cases). Prefer structured variants.
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
    /// Invalid pattern instance or inner id (slash / empty).
    InvalidPatternId,
    Parse,
    Serialize,
    /// Knot/author parameter out of range (e.g. Digitize steps=0).
    InvalidParam,
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
            WyrdError::InvalidPatternId => f.write_str("invalid pattern instance id"),
            WyrdError::Parse => f.write_str("parse error"),
            WyrdError::Serialize => f.write_str("serialize error"),
            WyrdError::InvalidParam => f.write_str("invalid parameter"),
        }
    }
}

pub type Result<T> = core::result::Result<T, WyrdError>;

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write;
    use std::string::String;

    fn disp(e: &WyrdError) -> String {
        let mut s = String::new();
        write!(&mut s, "{e}").unwrap();
        s
    }

    #[test]
    fn display_all_variants() {
        let cases: &[(WyrdError, &str)] = &[
            (WyrdError::Msg("x"), "x"),
            (WyrdError::UnknownKnot, "unknown knot"),
            (WyrdError::UnknownPort, "unknown port"),
            (WyrdError::UnknownPath, "unknown host path"),
            (WyrdError::DuplicateKnotId, "duplicate knot id"),
            (WyrdError::Cycle, "cycle in weave"),
            (WyrdError::FanIn, "fan-in > 1 on input port"),
            (WyrdError::UnconnectedRequired, "required port unconnected"),
            (WyrdError::Budget, "budget exceeded"),
            (WyrdError::NumericMismatch, "numeric path mismatch"),
            (WyrdError::Empty, "empty weave"),
            (WyrdError::InvalidPatternId, "invalid pattern instance id"),
            (WyrdError::Parse, "parse error"),
            (WyrdError::Serialize, "serialize error"),
            (WyrdError::InvalidParam, "invalid parameter"),
        ];
        for (err, msg) in cases {
            assert_eq!(disp(err), *msg);
        }
    }
}
