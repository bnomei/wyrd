//! RON load/save for weaves with validate-on-decode (`serde-ron` feature).

use core::fmt;

use std::string::String;

use crate::{ValidationError, Weave, WeaveDef};

/// RON parse, serialize, or post-parse validation failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum RonCodecError {
    Parse {
        source: ron::error::SpannedError,
        line: usize,
        column: usize,
    },
    Validation(ValidationError),
    Serialize(ron::Error),
}

impl fmt::Display for RonCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse {
                source,
                line,
                column,
            } => write!(
                f,
                "RON parse error at line {line}, column {column}: {source}"
            ),
            Self::Validation(error) => write!(f, "invalid RON weave: {error}"),
            Self::Serialize(error) => write!(f, "RON serialization error: {error}"),
        }
    }
}

impl std::error::Error for RonCodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse { source, .. } => Some(source),
            Self::Validation(source) => Some(source),
            Self::Serialize(source) => Some(source),
        }
    }
}

impl From<ValidationError> for RonCodecError {
    fn from(value: ValidationError) -> Self {
        Self::Validation(value)
    }
}

/// Parse RON text as a [`WeaveDef`] and validate into a [`Weave`].
pub fn from_ron(text: &str) -> Result<Weave, RonCodecError> {
    let def: WeaveDef = ron::from_str(text).map_err(|source: ron::error::SpannedError| {
        let line = source.span.start.line;
        let column = source.span.start.col;
        RonCodecError::Parse {
            source,
            line,
            column,
        }
    })?;
    Ok(Weave::try_from(def)?)
}

/// Pretty-print a weave definition as RON.
pub fn to_ron(weave: &Weave) -> Result<String, RonCodecError> {
    ron::ser::to_string_pretty(&weave.to_def(), ron::ser::PrettyConfig::default())
        .map_err(RonCodecError::Serialize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;
    use std::string::ToString;

    struct FailingWriter;

    impl fmt::Write for FailingWriter {
        fn write_str(&mut self, _: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }

    #[test]
    fn serialize_error_formats_and_retains_its_source() {
        let source =
            ron::ser::to_writer_pretty(FailingWriter, &"codec", ron::ser::PrettyConfig::default())
                .expect_err("writer failure must be preserved as a RON error");
        let error = RonCodecError::Serialize(source);

        assert!(error.to_string().starts_with("RON serialization error: "));
        assert!(error.source().is_some());
    }
}
