//! Weave graph: author model + validate + rustic builder.

#![no_std]
#![forbid(unsafe_code)]

extern crate no_std_compat as std;

mod builder;
mod validate;
mod weave;

pub use builder::WeaveBuilder;
pub use validate::{validate, Budget};
pub use weave::{KnotDef, PortRefAuthor, ThreadDef, Weave};

pub use wyrd_core::{
    from_count, from_level, is_truthy, CalcOp, CompareOp, FlagPriority, KnotId, KnotKind,
    NumericPath, PortSlot, Result, Signal, TimerMode, WyrdError, ONE, ZERO,
};
