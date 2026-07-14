//! # D02: Checkpoints and authored starting state
//!
//! Save Wyrd's continuation after the host applies a frame, load it into a
//! completely fresh runtime, and use semantic presets for intentional starts.
//! The game owns its wider save schema and host effects; Wyrd owns only its
//! deterministic graph continuation.
//!
//! ```
//! use wyrd::{
//!     weave, BindOpts, FlagPriority, HostTime, KnotKind, Runtime, RuntimePreset,
//!     RuntimePresetEntry, RuntimeStateEntry, SignalDomain, ONE,
//! };
//!
//! let weave = weave! {
//!     id: "checkpoint-example";
//!     knots {
//!         open = KnotKind::signal_in(SignalDomain::Bool);
//!         gate = KnotKind::flag(FlagPriority::SetWins, false);
//!         count = KnotKind::counter();
//!         gate_out = KnotKind::signal_out("gate.open", SignalDomain::Bool);
//!     }
//!     threads {
//!         open.out -> gate.set;
//!         open.out -> count.inc;
//!         gate.out -> gate_out.in;
//!     }
//! }?;
//!
//! let mut runtime = Runtime::bind(weave.clone(), BindOpts::default())?;
//! let open = runtime.required_sense("open")?;
//! runtime.begin_frame(HostTime { tick: 42 });
//! runtime.port_writer().set_sense(open, ONE)?;
//! runtime.loom();
//! // The game applies `runtime.outbox()` here. Checkpoint only after that
//! // host work has completed and before the next `begin_frame`.
//! let checkpoint = runtime.snapshot();
//!
//! let restored = Runtime::bind_restored(weave.clone(), BindOpts::default(), &checkpoint)?;
//! assert!(restored.outbox().signals().is_empty());
//! assert!(restored.inspect_checkpoint(&checkpoint)?.entries.iter().any(
//!     |entry| matches!(entry, RuntimeStateEntry::Flag { knot, value: true } if knot == "gate")
//! ));
//!
//! let mut preset = RuntimePreset::new();
//! preset.push(RuntimePresetEntry::Flag { knot: "gate".into(), value: true });
//! preset.push(RuntimePresetEntry::Counter { knot: "count".into(), value: 3 });
//! let authored_start = Runtime::bind_with_preset(weave, BindOpts::default(), &preset)?;
//! assert!(authored_start.inspect_state().entries.iter().any(
//!     |entry| matches!(entry, RuntimeStateEntry::Counter { knot, value: 3 } if knot == "count")
//! ));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Use a checkpoint for an exact midpoint continuation. Use a preset for an
//! authored, inspectable initial state; presets intentionally cannot set delay
//! rings, timer cursors, edge history, or RNG stream positions.
//!
//! You have completed the tiered examples. Return to the [`examples`](crate::examples)
//! index or continue into the API reference.
