//! Shared Wyrd vocabulary: monomorphic [`Signal`], dense runtime ids, closed
//! port tables, and the author-facing [`KnotKind`] catalog.
//!
//! This crate is always `#![no_std]` and forbids `unsafe`. Use the `signal-f32`
//! or `signal-i32` feature for the wire numeric path; enable exactly one.
//! Author graphs with open string host paths; bind later interns them.

#![no_std]
#![forbid(unsafe_code)]

extern crate no_std_compat as std;

pub mod ids;
pub mod kind;
pub mod ports;
pub mod signal;

pub use ids::{HostTime, KnotId, PortSlot, Seed, ThreadId};
pub use kind::{CalcOp, CompareOp, FlagPriority, KnotKind, NumericPath, SignalDomain, TimerMode};
pub use ports::{port_domain, port_slot, ports_of, PortDir, PortDomain, PortInfo};
pub use signal::{from_count, from_level, is_truthy, Signal, ONE, ZERO};

/// Path-local arithmetic on [`Signal`] (prefer `Calc` knots inside Weaves).
///
/// These ops implement the dual numeric path (f32 mul/div vs i32 Q16). Graphs
/// should express math with `KnotKind::Calc` so bind can specialize; host and
/// test code may call these helpers directly.
pub mod signal_ops {
    pub use crate::signal::{div, mul, sat_add, sat_sub};
}
