//! Learn Wyrd through small, ordered, executable examples.
//!
//! The lessons form a non-interactive, Rustlings-style path: start at
//! [`tier_a::a01_hello_invert`] and follow the “Next” link at the end of each
//! page. Every example is a doctest built from Wyrd's public API, so the code
//! shown here is also the code checked by CI.
//!
//! | Tier | What you learn |
//! | --- | --- |
//! | [`tier_a`] | Weaves, binding, frames, host ticks, and validation |
//! | [`tier_b`] | Reusable patterns and common stateful building blocks |
//! | [`tier_c`] | Puzzle-oriented compositions and typed generation |
//! | [`tier_d`] | A chamber-sized graph with host-owned world effects |
//!
//! Wyrd decides signals and emits requests. Your game remains responsible for
//! movement, collision, animation, scene changes, and persistence.

pub mod tier_a;
pub mod tier_b;
pub mod tier_c;
pub mod tier_d;
