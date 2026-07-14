//! # C03: Press N, then open a window
//!
//! Count presses, detect the first frame that reaches the goal, and hold a
//! reward signal for two frames.
//!
//! ```
//! use wyrd::{
//!     from_count, tick_once, weave, BindOpts, CompareOp, KnotKind, Runtime, ScriptedHost,
//!     SignalDomain, TimerMode, ONE, ZERO,
//! };
//!
//! let weave = weave! {
//!     id: "c03";
//!     knots {
//!         press = KnotKind::signal_in(SignalDomain::Bool);
//!         count = KnotKind::counter();
//!         goal = KnotKind::compare(CompareOp::Gte, Some(from_count(2)), SignalDomain::Count);
//!         reached = KnotKind::rising_from_zero();
//!         window = KnotKind::timer(TimerMode::PulseHold, 2);
//!         reward = KnotKind::signal_out("reward", SignalDomain::Bool);
//!     }
//!     threads {
//!         press.out -> count.inc; count.count -> goal.lhs; goal.out -> reached.in;
//!         reached.out -> window.start; window.active -> reward.in;
//!     }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let press = runtime.required_sense("press")?;
//! let mut host = ScriptedHost::new();
//!
//! let frames = [
//!     (ONE, ZERO),
//!     (ZERO, ZERO),
//!     (ONE, ONE),
//!     (ZERO, ONE),
//!     (ZERO, ZERO),
//! ];
//! for (input, expected) in frames {
//!     host.push_frame([(press, input)]);
//!     tick_once(&mut host, &mut runtime)?;
//!     assert_eq!(runtime.outbox().signals()[0].value, expected);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Separate level, edge, and duration explicitly; each knot owns one temporal
//! responsibility.
//!
//! Next: [`c04_button_cooldown`](super::c04_button_cooldown).
