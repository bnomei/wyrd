//! # B03: Toggle a flag
//!
//! Store Boolean state with rising-edge toggle and reset inputs.
//!
//! ```
//! use wyrd::{weave, tick_once, BindOpts, FlagPriority, KnotKind, Runtime, ScriptedHost, SignalDomain, ONE, ZERO};
//!
//! let weave = weave! {
//!     id: "b03";
//!     knots {
//!         toggle = KnotKind::signal_in(SignalDomain::Bool);
//!         reset = KnotKind::signal_in(SignalDomain::Bool);
//!         flag = KnotKind::flag(FlagPriority::ResetWins, true);
//!         lamp = KnotKind::signal_out("lamp", SignalDomain::Bool);
//!     }
//!     threads { toggle.out -> flag.toggle; reset.out -> flag.reset; flag.out -> lamp.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let toggle = runtime.required_sense("toggle")?;
//! let reset = runtime.required_sense("reset")?;
//! let mut host = ScriptedHost::new();
//!
//! host.push_frame([(toggle, ONE), (reset, ZERO)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ONE);
//! host.push_frame([(toggle, ZERO), (reset, ONE)]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(runtime.outbox().signals()[0].value, ZERO);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `ResetWins` defines the result if set-like and reset inputs arrive together.
//!
//! Next: [`b04_counter_threshold`](super::b04_counter_threshold).
