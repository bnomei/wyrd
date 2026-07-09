//! Weave graph: author model + validate + rustic builder.

#![no_std]
#![forbid(unsafe_code)]

extern crate no_std_compat as std;

mod builder;
mod pattern;
mod validate;
mod weave;

pub use builder::{slot_of, WeaveBuilder};
pub use pattern::{expand_pattern, merge_expanded, Pattern, PatternExports};
pub use validate::{validate, Budget};
pub use weave::{KnotDef, PortRefAuthor, ThreadDef, Weave};

pub use wyrd_core::{
    from_count, from_level, is_truthy, CalcOp, CompareOp, FlagPriority, KnotId, KnotKind,
    NumericPath, PortSlot, Result, Signal, TimerMode, WyrdError, ONE, ZERO,
};
