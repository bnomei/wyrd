//! # C09: Emit once
//!
//! Turn a held condition into one command request on its rising edge.
//!
//! ```
//! use wyrd::{weave, tick_once, BindOpts, KnotKind, Runtime, ScriptedHost, SignalDomain, ONE};
//!
//! let weave = weave! {
//!     id: "c09";
//!     knots {
//!         allowed = KnotKind::signal_in(SignalDomain::Bool);
//!         edge = KnotKind::rising_from_zero();
//!         ping = KnotKind::emit_command("sfx.ping");
//!     }
//!     threads { allowed.out -> edge.in; edge.out -> ping.trigger; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let allowed = runtime.required_sense("allowed")?;
//! let mut host = ScriptedHost::new();
//!
//! host.push_frame([(allowed, ONE)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().emits().len(), 1);
//! host.push_frame([(allowed, ONE)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert!(runtime.outbox().emits().is_empty());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Feed `EmitCommand` events, not persistent levels, unless repeated commands
//! are intentional.
//!
//! Next: [`c10_or_any_of_keys`](super::c10_or_any_of_keys).
