//! # A04: Tick through a host
//!
//! Use [`tick_once`](crate::tick_once) to sample a host, settle Wyrd, and apply
//! the resulting outbox in one call.
//!
//! ```
//! use wyrd::{weave, tick_once, BindOpts, KnotKind, Runtime, ScriptedHost, SignalDomain, ONE, ZERO};
//!
//! let weave = weave! {
//!     id: "a04";
//!     knots {
//!         input as "in" = KnotKind::signal_in(SignalDomain::Bool);
//!         output = KnotKind::signal_out("lamp", SignalDomain::Bool);
//!     }
//!     threads { input.out -> output.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let input = runtime.required_sense("in")?;
//! let mut host = ScriptedHost::new();
//!
//! host.push_frame([(input, ZERO)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ZERO);
//!
//! host.push_frame([(input, ONE)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ONE);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `ScriptedHost` makes the boundary visible in tests. A game adapter
//! implements the same `Host` contract with engine state.
//!
//! Next: [`a05_invalid_map_validation`](super::a05_invalid_map_validation).
