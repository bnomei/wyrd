//! # A05: Read a validation error
//!
//! Invalid parameters fail while the graph is built, before a runtime exists.
//!
//! ```
//! use wyrd::{weave, BuildError, KnotKind, SignalDomain, ValidationError, ONE, ZERO, from_count};
//!
//! let result = weave! {
//!     id: "a05";
//!     knots {
//!         source = KnotKind::constant(ONE, SignalDomain::Level);
//!         map = KnotKind::Map {
//!             domain: SignalDomain::Level,
//!             in_min: from_count(5),
//!             in_max: from_count(1),
//!             out_min: ZERO,
//!             out_max: ONE,
//!         };
//!         output = KnotKind::signal_out("y", SignalDomain::Level);
//!     }
//!     threads { source.out -> map.in; map.out -> output.in; }
//! };
//!
//! assert!(matches!(
//!     result,
//!     Err(BuildError::Validation(ValidationError::InvalidParameter { .. }))
//! ));
//! ```
//!
//! Treat validation diagnostics as authoring feedback. Hosts should only bind
//! graphs that have already passed this boundary.
//!
//! Next: [`b01_monostable_pattern`](crate::examples::tier_b::b01_monostable_pattern).
