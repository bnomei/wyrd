//! Weave graph: author model + validate + rustic builder.

#![no_std]
#![forbid(unsafe_code)]

extern crate no_std_compat as std;

mod builder;
mod pattern;
mod validate;
mod weave;

#[cfg(feature = "serde-ron")]
mod serde_ron;

pub use builder::{slot_of, WeaveBuilder};
pub use pattern::{expand_pattern, merge_expanded, Pattern, PatternExports};
pub use validate::{validate, validate_report, Budget, BudgetWarning, ValidateReport};
pub use weave::{KnotDef, PortRefAuthor, ThreadDef, Weave};

#[cfg(feature = "serde-ron")]
pub use serde_ron::{from_ron, to_ron};

pub use wyrd_core::{
    from_count, from_level, is_truthy, CalcOp, CompareOp, FlagPriority, KnotId, KnotKind,
    NumericPath, PortSlot, Result, Signal, TimerMode, WyrdError, ONE, ZERO,
};
