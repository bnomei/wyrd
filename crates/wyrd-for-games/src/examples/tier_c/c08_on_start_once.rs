//! # C08: Initialize once
//!
//! Convert the runtime's first-frame pulse into persistent initialized state.
//!
//! ```
//! use wyrd::{weave, tick_once, BindOpts, FlagPriority, KnotKind, Runtime, ScriptedHost, SignalDomain, ONE};
//!
//! let weave = weave! {
//!     id: "c08";
//!     knots {
//!         start = KnotKind::OnStart;
//!         initialized = KnotKind::flag(FlagPriority::SetWins, false);
//!         output = KnotKind::signal_out("booted", SignalDomain::Bool);
//!     }
//!     threads { start.out -> initialized.set; initialized.out -> output.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let mut host = ScriptedHost::new();
//! host.push_frame([]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ONE);
//! host.push_frame([]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ONE);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `OnStart` itself pulses once; the flag is what remembers the event.
//!
//! Next: [`c09_emit_once`](super::c09_emit_once).
