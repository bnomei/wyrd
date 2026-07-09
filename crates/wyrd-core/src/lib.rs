//! Wyrd core: scalar Signal, dense ids, closed port schema, KnotKind.
//!
//! #![no_std] always; use `no-std-compat` as `std` for Vec/String under alloc.

#![no_std]
#![forbid(unsafe_code)]

extern crate no_std_compat as std;

pub mod error;
pub mod ids;
pub mod kind;
pub mod ports;
pub mod signal;

pub use error::{Result, WyrdError};
pub use ids::{CmdId, HostPathId, HostTime, KnotId, PortSlot, Seed, ThreadId};
pub use kind::{
    CalcOp, CompareOp, FlagPriority, KnotKind, NumericPath, TimerMode,
};
pub use ports::{port_slot, ports_of, PortDir, PortInfo};
pub use signal::{div, from_count, from_level, is_truthy, mul, sat_add, sat_sub, Signal, ONE, ZERO};
