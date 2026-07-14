//! # B04: Count up to a threshold
//!
//! Turn rising input presses into a persistent count and compare it with a goal.
//!
//! ```
//! use wyrd::{
//!     from_count, tick_once, weave, BindOpts, CompareOp, KnotKind, Runtime, ScriptedHost,
//!     SignalDomain, ONE, ZERO,
//! };
//!
//! let weave = weave! {
//!     id: "b04";
//!     knots {
//!         increment = KnotKind::signal_in(SignalDomain::Bool);
//!         count = KnotKind::counter();
//!         ready = KnotKind::compare(CompareOp::Gte, Some(from_count(2)), SignalDomain::Count);
//!         output = KnotKind::signal_out("ready", SignalDomain::Bool);
//!     }
//!     threads { increment.out -> count.inc; count.count -> ready.lhs; ready.out -> output.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let increment = runtime.required_sense("increment")?;
//! let mut host = ScriptedHost::new();
//!
//! for value in [ONE, ZERO, ONE] {
//!     host.push_frame([(increment, value)]);
//!     tick_once(&mut host, &mut runtime)?;
//! }
//! assert_eq!(runtime.outbox().signals()[0].value, ONE);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The counter owns edge detection, so holding the input does not repeatedly
//! increment it.
//!
//! Next: [`b05_delayed_pulse`](super::b05_delayed_pulse).
