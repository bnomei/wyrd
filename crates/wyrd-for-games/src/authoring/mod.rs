//! Author model for Weaves: builder, `weave!` macro, patterns, validate, codecs.
//!
//! Graphs are authored with open string knot ids and port names, then validated
//! into an immutable [`Weave`]. Execution never happens here — bind in
//! [`crate::Runtime`] consumes the weave into dense buffers.
//!
//! Optional features: `serde` on defs, `serde-ron` / `serde-json` for load/save
//! with validate-on-decode.

pub(crate) mod builder;
pub(crate) mod composer;
pub(crate) mod error;
mod macros;
pub(crate) mod pattern;
pub(crate) mod validate;
pub(crate) mod weave;

#[cfg(feature = "serde-ron")]
pub(crate) mod serde_ron;

#[cfg(feature = "serde-json")]
pub(crate) mod serde_json_codec;

pub use builder::{slot_of, InputPort, KnotHandle, OutputPort, PatternInstance, WeaveBuilder};
pub use composer::{
    Bool, BoolWire, ComposeError, Composer, Count, CountWire, Level, LevelWire, NumericWireDomain,
    ThresholdWires, Wire, WireDomain,
};
pub use error::{BuildError, ValidationError};
pub use pattern::{Pattern, PatternDef, PatternExportDef};
pub use validate::{validate, validate_report, Budget, BudgetWarning, ValidateReport};
pub use weave::{KnotDef, PortRefDef, ThreadDef, Weave, WeaveDef};

#[cfg(feature = "serde-ron")]
pub use serde_ron::{from_ron, to_ron, RonCodecError};

#[cfg(feature = "serde-json")]
pub use serde_json_codec::{from_json, to_json, JsonCodecError};
