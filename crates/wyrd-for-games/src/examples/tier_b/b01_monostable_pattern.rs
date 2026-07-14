//! # B01: A reusable monostable pattern
//!
//! Define a validated fragment once, then expand it into a larger weave.
//!
//! ```
//! use wyrd::{
//!     pattern, tick_once, weave, BindOpts, KnotKind, Runtime, ScriptedHost, SignalDomain,
//!     TimerMode, ONE, ZERO,
//! };
//!
//! let monostable = pattern! {
//!     id: "pattern.monostable";
//!     knots {
//!         edge = KnotKind::rising_from_zero();
//!         hold = KnotKind::timer(TimerMode::PulseHold, 2);
//!     }
//!     exports { input start = edge.in; output active = hold.active; }
//!     threads { edge.out -> hold.start; }
//! }?;
//! let weave = weave! {
//!     id: "b01";
//!     knots {
//!         button = KnotKind::signal_in(SignalDomain::Bool);
//!         lamp = KnotKind::signal_out("lamp", SignalDomain::Bool);
//!     }
//!     patterns { pulse = ("pulse", &monostable); }
//!     threads { button.out -> pulse.in("start"); pulse.out("active") -> lamp.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let button = runtime.required_sense("button")?;
//! let mut host = ScriptedHost::new();
//!
//! for value in [ZERO, ONE, ZERO] {
//!     host.push_frame([(button, value)]);
//!     tick_once(&mut host, &mut runtime)?;
//! }
//! assert_eq!(runtime.outbox().signals()[0].value, ONE);
//!
//! host.push_frame([(button, ZERO)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ZERO);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Patterns are expanded and validated at load time; they add no per-frame
//! abstraction cost.
//!
//! Next: [`b02_two_plate_door`](super::b02_two_plate_door).
