//! # B05: Delay a value
//!
//! Delay a signal by two settled frames.
//!
//! ```
//! use wyrd::{weave, tick_once, BindOpts, KnotKind, Runtime, ScriptedHost, SignalDomain, ONE, ZERO};
//!
//! let weave = weave! {
//!     id: "b05";
//!     knots {
//!         input as "in" = KnotKind::signal_in(SignalDomain::Level);
//!         delay = KnotKind::Delay { ticks: 2 };
//!         output = KnotKind::signal_out("y", SignalDomain::Level);
//!     }
//!     threads { input.out -> delay.in; delay.out -> output.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let input = runtime.required_sense("in")?;
//! let mut host = ScriptedHost::new();
//!
//! for expected in [ZERO, ZERO, ONE] {
//!     host.push_frame([(input, ONE)]);
//!     tick_once(&mut host, &mut runtime)?;
//!     assert_eq!(runtime.outbox().signals()[0].value, expected);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Delay is explicit state. It is useful for timing and also breaks what would
//! otherwise be an invalid combinational cycle.
//!
//! Next: [`c01_multi_switch_latch`](crate::examples::tier_c::c01_multi_switch_latch).
