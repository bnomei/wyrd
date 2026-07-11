//! Author model for Weaves: builder, `weave!` macro, patterns, validate, codecs.
//!
//! Graphs are authored with open string knot ids and port names, then validated
//! into an immutable [`Weave`]. Execution never happens here — bind in
//! `wyrd-runtime` consumes the weave into dense buffers.
//!
//! Optional features: `serde` on defs, `serde-ron` / `serde-json` for load/save
//! with validate-on-decode.

#![no_std]
#![forbid(unsafe_code)]

extern crate no_std_compat as std;

mod builder;
mod error;
mod macros;
mod pattern;
mod validate;
mod weave;

#[cfg(feature = "serde-ron")]
mod serde_ron;

#[cfg(feature = "serde-json")]
mod serde_json_codec;

pub use builder::{slot_of, InputPort, KnotHandle, OutputPort, PatternInstance, WeaveBuilder};
pub use error::{BuildError, ValidationError};
pub use pattern::{Pattern, PatternDef, PatternExportDef};
pub use validate::{validate, validate_report, Budget, BudgetWarning, ValidateReport};
pub use weave::{KnotDef, PortRefDef, ThreadDef, Weave, WeaveDef};

#[cfg(feature = "serde-ron")]
pub use serde_ron::{from_ron, to_ron, RonCodecError};

#[cfg(feature = "serde-json")]
pub use serde_json_codec::{from_json, to_json, JsonCodecError};

pub use wyrd_core::{
    from_count, from_level, is_truthy, CalcOp, CompareOp, FlagPriority, KnotId, KnotKind,
    NumericPath, PortSlot, Signal, SignalDomain, TimerMode, ONE, ZERO,
};
