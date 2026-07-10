//! Tutorial ladder — pedagogy only (not hot-path API).
//!
//! Ordered recipes from foundations through GBG / Zelda literacy machines.
//! Each `run_*` has a full Weave under **Examples** in rustdoc (`cargo doc --open`).
//!
//! Integration tests: `cargo test -p wyrd-runtime --test tutorial_ladder`.
//!
//! | Tier | Focus |
//! | --- | --- |
//! | **A** | Constant, And, bind/loom, Host tick, validate |
//! | **B** | Monostable Pattern, door, Flag, Counter, Delay |
//! | **C** | Latch, FedCountdown, cooldown, Map/Digitize, Emit-once, Or |
//!
//! # Example (A01, full graph)
//!
//! ```
//! use wyrd_core::{HostTime, KnotKind, ONE};
//! use wyrd_graph::Weave;
//! use wyrd_runtime::cookbook::helpers::{bind_default, signal_out_truthy};
//!
//! // Constant(ONE) → Not → SignalOut
//! let (b, _) = Weave::builder("hello")
//!     .knot("c", KnotKind::constant(ONE))
//!     .unwrap();
//! let (b, _) = b.knot("n", KnotKind::not()).unwrap();
//! let (b, _) = b.knot("o", KnotKind::signal_out("debug.inverted")).unwrap();
//! let weave = b
//!     .wire_named("c", "out", "n", "in")
//!     .wire_named("n", "out", "o", "in")
//!     .build()
//!     .unwrap();
//!
//! let mut rt = bind_default(&weave).unwrap();
//! rt.begin_frame(HostTime { tick: 0 });
//! rt.loom(&weave).unwrap();
//! assert!(!signal_out_truthy(&rt, "debug.inverted"));
//! ```
//!
//! Browse: `tier_a`, `tier_b`, `tier_c` → open any `run_*` → **Examples**.

pub mod helpers;
pub mod tier_a;
pub mod tier_b;
pub mod tier_c;

pub use helpers::{bind_default, signal_out_truthy, tick_senses};
