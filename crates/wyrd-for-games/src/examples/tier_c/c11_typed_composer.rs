//! # C11: Generate topology with typed wires
//!
//! Use [`Weave::compose`](crate::Weave::compose) when data generates a graph.
//! The composer prevents mixing Boolean, level, and count wires.
//!
//! ```
//! use wyrd::{tick_once, from_count, from_level, BindOpts, CompareOp, Runtime, ScriptedHost, Weave, ONE};
//! # fn value(runtime: &Runtime, path: &str) -> wyrd::Signal { let id = runtime.path_id(path).unwrap(); runtime.outbox().signals().iter().find(|s| s.path == id).unwrap().value }
//!
//! let weave = Weave::compose("c11", |composer| {
//!     let button = composer.bool_input("button")?;
//!     let level = composer.level_input("level")?;
//!     let count = composer.count_input("count")?;
//!     let press = composer.rising("press", &button)?;
//!     let ready = composer.pulse_hold("ready", 2, &press)?;
//!     let count_ready = composer.compare_constant(
//!         "count-ready", CompareOp::Gte, &count, from_count(2),
//!     )?;
//!     composer.signal_out("ready-out", "composer.ready", &ready)?;
//!     composer.signal_out("level-out", "composer.level", &level)?;
//!     composer.signal_out("count-out", "composer.count_ready", &count_ready)
//! })?;
//!
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let button = runtime.required_sense("button")?;
//! let level = runtime.required_sense("level")?;
//! let count = runtime.required_sense("count")?;
//! let mut host = ScriptedHost::new();
//! host.push_frame([(button, ONE), (level, from_level(0.5)), (count, from_count(2))]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(value(&runtime, "composer.ready"), ONE);
//! assert_eq!(value(&runtime, "composer.level"), from_level(0.5));
//! assert_eq!(value(&runtime, "composer.count_ready"), ONE);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The composed result is the same validated `Weave` used by declarative
//! recipes. Prefer semantic helpers; raw knot wiring is the advanced escape
//! hatch.
//!
//! Next: [`d01_shrine_chamber`](crate::examples::tier_d::d01_shrine_chamber).
