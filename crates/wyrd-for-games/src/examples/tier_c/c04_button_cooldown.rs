//! # C04: Button cooldown
//!
//! Emit a one-frame shot on a press while exposing a longer cooling cue.
//!
//! ```
//! use wyrd::{weave, tick_once, BindOpts, KnotKind, Runtime, ScriptedHost, SignalDomain, TimerMode, ONE, ZERO};
//! # fn value(runtime: &Runtime, path: &str) -> wyrd::Signal { let id = runtime.path_id(path).unwrap(); runtime.outbox().signals().iter().find(|s| s.path == id).unwrap().value }
//!
//! let weave = weave! {
//!     id: "c04";
//!     knots {
//!         button = KnotKind::signal_in(SignalDomain::Bool);
//!         edge = KnotKind::rising_from_zero();
//!         cooldown = KnotKind::timer(TimerMode::PulseHold, 2);
//!         shot = KnotKind::signal_out("shot", SignalDomain::Bool);
//!         cooling = KnotKind::signal_out("cooling", SignalDomain::Bool);
//!     }
//!     threads {
//!         button.out -> edge.in; edge.out -> cooldown.start;
//!         edge.out -> shot.in; cooldown.active -> cooling.in;
//!     }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let button = runtime.required_sense("button")?;
//! let mut host = ScriptedHost::new();
//!
//! host.push_frame([(button, ONE)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(value(&runtime, "shot"), ONE);
//! assert_eq!(value(&runtime, "cooling"), ONE);
//! host.push_frame([(button, ONE)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(value(&runtime, "shot"), ZERO);
//! assert_eq!(value(&runtime, "cooling"), ONE);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Wyrd graphs are DAGs. The host can suppress new input while `cooling`
//! rather than wiring a timer back into itself.
//!
//! Next: [`c05_axis_digital`](super::c05_axis_digital).
