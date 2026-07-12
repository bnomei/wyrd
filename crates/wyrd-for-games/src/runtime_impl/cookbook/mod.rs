//! Tutorial ladder — pedagogy only (not hot-path API).
//!
//! Ordered recipes from foundations through puzzle-machine and chamber composition.
//! Each `run_*` has a full Weave under **Examples** in rustdoc (`cargo doc --open`).
//!
//! Start with the declarative path: [`crate::weave!`] defines a complete
//! topology and [`crate::pattern!`] names a reusable validated fragment. Tier A
//! then shows the host boundary: a [`crate::Recipe`] resolves its typed ports
//! once and a [`crate::Scenario`] owns deterministic frames. For generated
//! topology, C11 uses [`crate::Weave::compose`] and [`crate::Composer`] to keep
//! Bool, Level, and Count wires type checked while still lowering through the
//! normal builder and validator.
//!
//! `Composer::knot` / `Composer::thread` remain an advanced escape hatch for a
//! catalog operation without a semantic helper. Prefer the typed helpers in
//! examples and application code; the raw path is for deliberate full-catalog
//! composition, not ordinary recipe setup.
//!
//! Integration tests: `cargo test -p wyrd-for-games --test tutorial_ladder`.
//!
//! | Tier | Focus |
//! | --- | --- |
//! | **A** | `weave!`, typed Recipe + Scenario host boundary, validate |
//! | **B** | `pattern!`, typed two-plate recipe, Flag, Counter, Delay |
//! | **C** | Latch, FedCountdown, cooldown, Map/Digitize, Composer, Emit-once, Or |
//! | **D** | Chamber-scale combination: multiple conditions, a latched gate, mover target, transition request |
//!
//! # Example (A01)
//!
//! ```
//! wyrd::cookbook::tier_a::run_a01_hello_invert().unwrap();
//! ```
//!
//! Browse: `tier_a`, `tier_b`, `tier_c`, `tier_d` → open any `run_*` → **Examples**.

pub mod helpers;
pub mod tier_a;
pub mod tier_b;
pub mod tier_c;
pub mod tier_d;

pub use helpers::{bind_default, signal_out_truthy, tick_senses};

pub type Result<T = ()> = core::result::Result<T, crate::CookbookError>;
