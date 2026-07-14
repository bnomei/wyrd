//! # A01: Hello, invert
//!
//! Build the smallest useful [`Weave`](crate::Weave): a true constant flows
//! through `Not` and becomes a false signal for the host.
//!
//! ```
//! use wyrd::{weave, BindOpts, HostTime, KnotKind, Runtime, SignalDomain, ONE, ZERO};
//!
//! let weave = weave! {
//!     id: "a01";
//!     knots {
//!         source = KnotKind::constant(ONE, SignalDomain::Bool);
//!         invert = KnotKind::not();
//!         output = KnotKind::signal_out("debug.inverted", SignalDomain::Bool);
//!     }
//!     threads { source.out -> invert.in; invert.out -> output.in; }
//! }?;
//!
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! runtime.begin_frame(HostTime { tick: 0 });
//! runtime.loom();
//!
//! let output = runtime.required_path("debug.inverted")?;
//! assert_eq!(runtime.outbox().signals()[0].path, output);
//! assert_eq!(runtime.outbox().signals()[0].value, ZERO);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `Runtime::loom` settles the immutable graph once. The host then reads the
//! typed outbox; Wyrd does not mutate game objects itself.
//!
//! Next: [`a02_two_plate_and`](super::a02_two_plate_and).
