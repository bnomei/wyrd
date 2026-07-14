//! # D01: Shrine chamber
//!
//! Combine a multi-object unlock, a latched gate, a continuous bridge target,
//! and a one-shot room-transition request in one chamber graph.
//!
//! ```
//! use wyrd::{
//!     from_level, tick_once, weave, BindOpts, FlagPriority, KnotKind, Runtime, ScriptedHost,
//!     SignalDomain, ONE, ZERO,
//! };
//! # fn value(runtime: &Runtime, path: &str) -> wyrd::Signal { let id = runtime.path_id(path).unwrap(); runtime.outbox().signals().iter().find(|s| s.path == id).unwrap().value }
//!
//! let weave = weave! {
//!     id: "shrine-chamber";
//!     knots {
//!         crate_on_sun_pad = KnotKind::signal_in(SignalDomain::Bool);
//!         player_on_moon_pad = KnotKind::signal_in(SignalDomain::Bool);
//!         relic_placed = KnotKind::signal_in(SignalDomain::Bool);
//!         pads_ready = KnotKind::and2();
//!         shrine_ready = KnotKind::and2();
//!         unlock_edge = KnotKind::rising_from_zero();
//!         gate_latch = KnotKind::flag(FlagPriority::ResetWins, false);
//!         gate_open = KnotKind::signal_out("shrine.gate.open", SignalDomain::Bool);
//!
//!         bridge_lever = KnotKind::signal_in(SignalDomain::Level);
//!         bridge_target = KnotKind::map(ZERO, ONE, ZERO, from_level(8.0), SignalDomain::Level);
//!         bridge_out = KnotKind::signal_out("shrine.bridge.target", SignalDomain::Level);
//!
//!         player_at_exit = KnotKind::signal_in(SignalDomain::Bool);
//!         exit_ready = KnotKind::and2();
//!         exit_edge = KnotKind::rising_from_zero();
//!         transition = KnotKind::emit_command("world.request_transition");
//!     }
//!     threads {
//!         crate_on_sun_pad.out -> pads_ready.in_0;
//!         player_on_moon_pad.out -> pads_ready.in_1;
//!         pads_ready.out -> shrine_ready.in_0;
//!         relic_placed.out -> shrine_ready.in_1;
//!         shrine_ready.out -> unlock_edge.in;
//!         unlock_edge.out -> gate_latch.set;
//!         gate_latch.out -> gate_open.in;
//!
//!         bridge_lever.out -> bridge_target.in;
//!         bridge_target.out -> bridge_out.in;
//!
//!         gate_latch.out -> exit_ready.in_0;
//!         player_at_exit.out -> exit_ready.in_1;
//!         exit_ready.out -> exit_edge.in;
//!         exit_edge.out -> transition.trigger;
//!     }
//! }?;
//!
//! let mut runtime = Runtime::bind(weave, BindOpts::default())?;
//! let crate_on_pad = runtime.required_sense("crate_on_sun_pad")?;
//! let player_on_pad = runtime.required_sense("player_on_moon_pad")?;
//! let relic = runtime.required_sense("relic_placed")?;
//! let lever = runtime.required_sense("bridge_lever")?;
//! let exit = runtime.required_sense("player_at_exit")?;
//! let mut host = ScriptedHost::new();
//!
//! host.push_frame([
//!     (crate_on_pad, ONE),
//!     (player_on_pad, ONE),
//!     (relic, ZERO),
//!     (lever, ONE),
//!     (exit, ZERO),
//! ]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(value(&runtime, "shrine.gate.open"), ZERO);
//! assert_eq!(value(&runtime, "shrine.bridge.target"), from_level(8.0));
//!
//! host.push_frame([
//!     (crate_on_pad, ONE),
//!     (player_on_pad, ONE),
//!     (relic, ONE),
//!     (lever, ONE),
//!     (exit, ZERO),
//! ]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(value(&runtime, "shrine.gate.open"), ONE);
//! assert!(runtime.outbox().emits().is_empty());
//!
//! host.push_frame([
//!     (crate_on_pad, ZERO),
//!     (player_on_pad, ZERO),
//!     (relic, ZERO),
//!     (lever, ZERO),
//!     (exit, ONE),
//! ]);
//! tick_once(&mut host, &mut runtime)?;
//! assert_eq!(value(&runtime, "shrine.gate.open"), ONE);
//! assert_eq!(value(&runtime, "shrine.bridge.target"), ZERO);
//! assert_eq!(runtime.outbox().emits().len(), 1);
//!
//! host.push_frame([
//!     (crate_on_pad, ZERO),
//!     (player_on_pad, ZERO),
//!     (relic, ZERO),
//!     (lever, ZERO),
//!     (exit, ONE),
//! ]);
//! tick_once(&mut host, &mut runtime)?;
//! assert!(runtime.outbox().emits().is_empty());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Wyrd owns the deterministic decisions. The host reads the gate and bridge
//! targets, performs movement and collision, handles the transition request,
//! and persists whatever progress the game design requires.
//!
//! You have completed the tiered examples. Return to the [`examples`](crate::examples)
//! index or continue into the API reference.
