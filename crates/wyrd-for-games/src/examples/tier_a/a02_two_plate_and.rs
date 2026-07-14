//! # A02: Two-plate door
//!
//! Combine two Boolean senses and expose a door request owned by the host.
//!
//! ```
//! use wyrd::{weave, tick_once, BindOpts, KnotKind, Runtime, ScriptedHost, SignalDomain, ONE, ZERO};
//!
//! let weave = weave! {
//!     id: "a02";
//!     knots {
//!         plate_a = KnotKind::signal_in(SignalDomain::Bool);
//!         plate_b = KnotKind::signal_in(SignalDomain::Bool);
//!         both = KnotKind::and2();
//!         door = KnotKind::signal_out("door.open", SignalDomain::Bool);
//!     }
//!     threads {
//!         plate_a.out -> both.in_0;
//!         plate_b.out -> both.in_1;
//!         both.out -> door.in;
//!     }
//! }?;
//!
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let a = runtime.required_sense("plate_a")?;
//! let b = runtime.required_sense("plate_b")?;
//! let door = runtime.required_path("door.open")?;
//! let mut host = ScriptedHost::new();
//!
//! host.push_frame([(a, ONE), (b, ZERO)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ZERO);
//!
//! host.push_frame([(a, ONE), (b, ONE)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].path, door);
//! assert_eq!(runtime.outbox().signals()[0].value, ONE);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The graph decides whether the door should open. The host applies that
//! request to its animation, collision, and persistence systems.
//!
//! Next: [`a03_bind_sample_loom`](super::a03_bind_sample_loom).
