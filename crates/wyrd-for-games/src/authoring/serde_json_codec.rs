//! JSON load/save for weaves with validate-on-decode (`serde-json` feature).
//!
//! Schema matches the RON codec (`WeaveDef`).

use core::fmt;

use std::string::String;

use crate::{ValidationError, Weave, WeaveDef};

/// JSON parse, serialize, or post-parse validation failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum JsonCodecError {
    Parse {
        source: serde_json::Error,
        line: usize,
        column: usize,
    },
    Validation(ValidationError),
    Serialize(serde_json::Error),
}

impl fmt::Display for JsonCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse {
                source,
                line,
                column,
            } => write!(
                f,
                "JSON parse error at line {line}, column {column}: {source}"
            ),
            Self::Validation(error) => write!(f, "invalid JSON weave: {error}"),
            Self::Serialize(error) => write!(f, "JSON serialization error: {error}"),
        }
    }
}

impl std::error::Error for JsonCodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse { source, .. } | Self::Serialize(source) => Some(source),
            Self::Validation(source) => Some(source),
        }
    }
}

impl From<ValidationError> for JsonCodecError {
    fn from(value: ValidationError) -> Self {
        Self::Validation(value)
    }
}

/// Parse JSON text as a [`WeaveDef`] and validate into a [`Weave`].
pub fn from_json(text: &str) -> Result<Weave, JsonCodecError> {
    let def: WeaveDef = serde_json::from_str(text).map_err(|source| {
        let line = source.line();
        let column = source.column();
        JsonCodecError::Parse {
            source,
            line,
            column,
        }
    })?;
    Ok(Weave::try_from(def)?)
}

/// Pretty-print a weave definition as JSON.
pub fn to_json(weave: &Weave) -> Result<String, JsonCodecError> {
    serde_json::to_string_pretty(&weave.to_def()).map_err(JsonCodecError::Serialize)
}
