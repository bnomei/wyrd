//! Tutorial ladder — pedagogy only (not hot-path API).
//!
//! Ordered recipes from foundations through puzzle-machine and chamber composition.
//! Each `run_*` has a full Weave under **Examples** in rustdoc (`cargo doc --open`).
//!
//! Integration tests: `cargo test -p wyrd-for-games --test tutorial_ladder`.
//!
//! | Tier | Focus |
//! | --- | --- |
//! | **A** | Constant, And, bind/loom, Host tick, validate |
//! | **B** | Monostable Pattern, door, Flag, Counter, Delay |
//! | **C** | Latch, FedCountdown, cooldown, Map/Digitize, Emit-once, Or |
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
