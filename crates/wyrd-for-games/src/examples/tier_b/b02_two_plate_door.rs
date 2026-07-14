//! # B02: A host-owned two-plate door
//!
//! Revisit the two-plate condition and make the ownership boundary explicit.
//!
//! ```
//! use wyrd::{weave, tick_once, BindOpts, KnotKind, Runtime, ScriptedHost, SignalDomain, ONE, ZERO};
//!
//! let weave = weave! {
//!     id: "b02";
//!     knots {
//!         plate_a = KnotKind::signal_in(SignalDomain::Bool);
//!         plate_b = KnotKind::signal_in(SignalDomain::Bool);
//!         both = KnotKind::and2();
//!         door = KnotKind::signal_out("door.open", SignalDomain::Bool);
//!     }
//!     threads { plate_a.out -> both.in_0; plate_b.out -> both.in_1; both.out -> door.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let a = runtime.required_sense("plate_a")?;
//! let b = runtime.required_sense("plate_b")?;
//! let mut host = ScriptedHost::new();
//!
//! host.push_frame([(a, ONE), (b, ZERO)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ZERO);
//! host.push_frame([(a, ONE), (b, ONE)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ONE);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The output is a request, not a door component. The host can animate or
//! reject it according to world rules.
//!
//! Next: [`b03_flag_toggle`](super::b03_flag_toggle).
