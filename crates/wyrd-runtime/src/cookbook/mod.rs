//! Tutorial ladder — pedagogy only (not hot-path API).
//!
//! Ordered recipes from foundations through GBG / Zelda literacy machines.
//! Integration tests: `cargo test -p wyrd-runtime --test tutorial_ladder`.
//!
//! | Tier | Focus |
//! | --- | --- |
//! | **A** | Constant, And, bind/loom, Host tick, validate |
//! | **B** | Monostable Pattern, door, Flag, Counter, Delay |
//! | **C** | Latch, FedCountdown, cooldown, Map/Digitize, Emit-once, Or |
//!
//! # Example
//!
//! ```
//! use wyrd_runtime::cookbook::tier_a;
//! tier_a::run_a01_hello_invert().unwrap();
//! ```

pub mod helpers;
pub mod tier_a;
pub mod tier_b;
pub mod tier_c;

pub use helpers::{bind_default, signal_out_truthy, tick_senses};
