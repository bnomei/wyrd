//! # C07: Digitize into steps
//!
//! Quantize a continuous level into a small number of stable bins.
//!
//! ```
//! use wyrd::{
//!     tick_once, weave, BindOpts, KnotKind, Runtime, ScriptedHost, SignalDomain, ONE, ZERO,
//! };
//!
//! let weave = weave! {
//!     id: "c07";
//!     knots {
//!         input as "in" = KnotKind::signal_in(SignalDomain::Level);
//!         bins = KnotKind::Digitize {
//!             domain: SignalDomain::Level,
//!             steps: 2,
//!             in_min: ZERO,
//!             in_max: ONE,
//!             out_min: ZERO,
//!             out_max: ONE,
//!         };
//!         output = KnotKind::signal_out("bin", SignalDomain::Level);
//!     }
//!     threads { input.out -> bins.in; bins.out -> output.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let input = runtime.required_sense("in")?;
//! let mut host = ScriptedHost::new();
//! host.push_frame([(input, ZERO)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ZERO);
//! host.push_frame([(input, ONE)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ONE);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Digitize is useful for levers, dials, and other continuous inputs that map
//! onto discrete puzzle states.
//!
//! Next: [`c08_on_start_once`](super::c08_on_start_once).
