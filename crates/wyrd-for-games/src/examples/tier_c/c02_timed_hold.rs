//! # C02: Timed hold
//!
//! Require a plate to stay occupied for two frames before unlocking.
//!
//! ```
//! use wyrd::{weave, tick_once, BindOpts, KnotKind, Runtime, ScriptedHost, SignalDomain, TimerMode, ONE, ZERO};
//!
//! let weave = weave! {
//!     id: "c02";
//!     knots {
//!         plate = KnotKind::signal_in(SignalDomain::Bool);
//!         hold = KnotKind::timer(TimerMode::FedCountdown, 2);
//!         unlocked = KnotKind::signal_out("unlocked", SignalDomain::Bool);
//!     }
//!     threads { plate.out -> hold.feed; hold.active -> unlocked.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let plate = runtime.required_sense("plate")?;
//! let mut host = ScriptedHost::new();
//!
//! for (input, expected) in [(ONE, ZERO), (ONE, ONE), (ZERO, ZERO)] {
//!     host.push_frame([(plate, input)]);
//!     tick_once(&mut host, &mut runtime)?;
//!     assert_eq!(runtime.outbox().signals()[0].value, expected);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `FedCountdown` resets when feeding stops, which makes it a compact “hold to
//! complete” primitive.
//!
//! Next: [`c03_press_n_then_window`](super::c03_press_n_then_window).
