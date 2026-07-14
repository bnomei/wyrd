//! # A03: Bind, sample, loom
//!
//! Drive a runtime directly when you do not need the [`Host`](crate::Host)
//! adapter.
//!
//! ```
//! use wyrd::{weave, BindOpts, HostTime, KnotKind, Runtime, SignalDomain, ONE, ZERO};
//!
//! let weave = weave! {
//!     id: "a03";
//!     knots {
//!         input as "in" = KnotKind::signal_in(SignalDomain::Bool);
//!         invert = KnotKind::not();
//!         output = KnotKind::signal_out("y", SignalDomain::Bool);
//!     }
//!     threads { input.out -> invert.in; invert.out -> output.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let input = runtime.required_sense("in")?;
//!
//! runtime.begin_frame(HostTime { tick: 0 });
//! runtime.port_writer().set_sense(input, ZERO)?;
//! runtime.loom();
//! assert_eq!(runtime.outbox().signals()[0].value, ONE);
//!
//! runtime.begin_frame(HostTime { tick: 1 });
//! runtime.port_writer().set_sense(input, ONE)?;
//! runtime.loom();
//! assert_eq!(runtime.outbox().signals()[0].value, ZERO);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Resolve names once after binding, then use dense typed handles on every
//! frame.
//!
//! Next: [`a04_host_tick_once`](super::a04_host_tick_once).
