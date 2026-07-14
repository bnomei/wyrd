//! # C06: Remap a range
//!
//! Convert a normalized count into a gameplay-scale count.
//!
//! ```
//! use wyrd::{
//!     from_count, tick_once, weave, BindOpts, KnotKind, Runtime, ScriptedHost, SignalDomain, ONE,
//!     ZERO,
//! };
//!
//! let weave = weave! {
//!     id: "c06";
//!     knots {
//!         input as "in" = KnotKind::signal_in(SignalDomain::Count);
//!         map = KnotKind::Map {
//!             domain: SignalDomain::Count,
//!             in_min: ZERO,
//!             in_max: ONE,
//!             out_min: ZERO,
//!             out_max: from_count(10),
//!         };
//!         output = KnotKind::signal_out("scaled", SignalDomain::Count);
//!     }
//!     threads { input.out -> map.in; map.out -> output.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let input = runtime.required_sense("in")?;
//! let mut host = ScriptedHost::new();
//! host.push_frame([(input, ONE)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, from_count(10));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Map preserves the declared signal domain and validates both ranges.
//!
//! Next: [`c07_digitize_steps`](super::c07_digitize_steps).
