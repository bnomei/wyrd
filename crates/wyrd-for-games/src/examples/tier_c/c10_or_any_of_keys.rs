//! # C10: Accept any key
//!
//! Open a condition when either of two independent key senses is active.
//!
//! ```
//! use wyrd::{weave, tick_once, BindOpts, KnotKind, Runtime, ScriptedHost, SignalDomain, ONE, ZERO};
//!
//! let weave = weave! {
//!     id: "c10";
//!     knots {
//!         key_a = KnotKind::signal_in(SignalDomain::Bool);
//!         key_b = KnotKind::signal_in(SignalDomain::Bool);
//!         any = KnotKind::or2();
//!         output = KnotKind::signal_out("open", SignalDomain::Bool);
//!     }
//!     threads { key_a.out -> any.in_0; key_b.out -> any.in_1; any.out -> output.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let a = runtime.required_sense("key_a")?;
//! let b = runtime.required_sense("key_b")?;
//! let mut host = ScriptedHost::new();
//!
//! host.push_frame([(a, ZERO), (b, ONE)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ONE);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Keep input identities separate at the host boundary even when the graph
//! later combines them.
//!
//! Next: [`c11_typed_composer`](super::c11_typed_composer).
