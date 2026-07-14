//! # C05: Turn an axis into digital events
//!
//! Produce both a held state and a one-frame crossing pulse from an analog
//! level.
//!
//! ```
//! use wyrd::{
//!     from_count, tick_once, weave, BindOpts, KnotKind, Runtime, ScriptedHost, SignalDomain, ONE,
//!     ZERO,
//! };
//! # fn value(runtime: &Runtime, path: &str) -> wyrd::Signal { let id = runtime.path_id(path).unwrap(); runtime.outbox().signals().iter().find(|s| s.path == id).unwrap().value }
//!
//! let weave = weave! {
//!     id: "c05";
//!     knots {
//!         axis = KnotKind::signal_in(SignalDomain::Level);
//!         threshold = KnotKind::Threshold {
//!             domain: SignalDomain::Level,
//!             high: from_count(5),
//!             low: ZERO,
//!             use_hysteresis: false,
//!         };
//!         pressed = KnotKind::signal_out("pressed", SignalDomain::Bool);
//!         crossed = KnotKind::signal_out("just_pressed", SignalDomain::Bool);
//!     }
//!     threads { axis.out -> threshold.in; threshold.out -> pressed.in; threshold.crossed_up -> crossed.in; }
//! }?;
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let axis = runtime.required_sense("axis")?;
//! let mut host = ScriptedHost::new();
//!
//! host.push_frame([(axis, from_count(4))]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(value(&runtime, "pressed"), ZERO);
//! host.push_frame([(axis, from_count(5))]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(value(&runtime, "pressed"), ONE);
//! assert_eq!(value(&runtime, "just_pressed"), ONE);
//! host.push_frame([(axis, from_count(5))]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(value(&runtime, "just_pressed"), ZERO);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Use the held output for state and the crossing output for one-shot actions.
//!
//! Next: [`c06_map_remap`](super::c06_map_remap).
