//! # C01: Multi-switch latch
//!
//! Open a door after two switches are active together, then keep it open until
//! an explicit reset.
//!
//! ```
//! use wyrd::{weave, tick_once, BindOpts, FlagPriority, KnotKind, Runtime, ScriptedHost, SignalDomain, ONE, ZERO};
//!
//! let weave = weave! {
//!     id: "c01";
//!     knots {
//!         a = KnotKind::signal_in(SignalDomain::Bool);
//!         b = KnotKind::signal_in(SignalDomain::Bool);
//!         reset = KnotKind::signal_in(SignalDomain::Bool);
//!         both = KnotKind::and2();
//!         edge = KnotKind::rising_from_zero();
//!         latch = KnotKind::flag(FlagPriority::ResetWins, false);
//!         door = KnotKind::signal_out("door.open", SignalDomain::Bool);
//!     }
//!     threads {
//!         a.out -> both.in_0; b.out -> both.in_1; both.out -> edge.in;
//!         edge.out -> latch.set; reset.out -> latch.reset; latch.out -> door.in;
//!     }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let a = runtime.required_sense("a")?;
//! let b = runtime.required_sense("b")?;
//! let reset = runtime.required_sense("reset")?;
//! let mut host = ScriptedHost::new();
//!
//! for (samples, expected) in [
//!     ([(a, ONE), (b, ZERO), (reset, ZERO)], ZERO),
//!     ([(a, ONE), (b, ONE), (reset, ZERO)], ONE),
//!     ([(a, ZERO), (b, ZERO), (reset, ZERO)], ONE),
//!     ([(a, ZERO), (b, ZERO), (reset, ONE)], ZERO),
//! ] {
//!     host.push_frame(samples);
//!     tick_once(&mut host, &mut runtime)?;
//!     assert_eq!(runtime.outbox().signals()[0].value, expected);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The rising edge converts a condition into a one-frame event; the flag turns
//! that event into persistent state.
//!
//! Next: [`c02_timed_hold`](super::c02_timed_hold).
