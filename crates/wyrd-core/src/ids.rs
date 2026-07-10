//! Dense runtime identifiers after validate/bind (D-id-space).
//!
//! These are not engine `Entity` handles. Author graphs use string knot and
//! host-path names; bind interns them into the compact ids here so host sample
//! and apply never look up strings on the hot path.

/// Dense knot index after validate/bind.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct KnotId(u16);

/// Port within a knot's closed catalog table (D-port-schema).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct PortSlot(u8);

/// Interned `SignalOut` host path (D-hostpath).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct HostPathId(u16);

/// Interned `EmitCommand` name (D-hostpath).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct CmdId(u16);

/// Dense id for a host-writable `SignalIn` sense (same numeric space as its knot).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct SenseId(u16);

/// Optional diagnostics / tooling id (not used on the settle hot path).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct ThreadId(u16);

/// Host-owned PRNG seed for `Random` knots (mixed with weave id at bind).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct Seed(pub u64);

/// Core time: discrete tick only (`dt` stays host-side).
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct HostTime {
    pub tick: u64,
}

macro_rules! dense_id {
    ($ty:ident, $raw:ty) => {
        impl $ty {
            /// Return the compact numeric representation.
            pub const fn get(self) -> $raw {
                self.0
            }
        }

        impl TryFrom<usize> for $ty {
            type Error = core::num::TryFromIntError;

            fn try_from(value: usize) -> Result<Self, Self::Error> {
                Ok(Self(<$raw>::try_from(value)?))
            }
        }

        impl From<$ty> for usize {
            fn from(value: $ty) -> Self {
                usize::from(value.0)
            }
        }
    };
}

dense_id!(KnotId, u16);
dense_id!(PortSlot, u8);
dense_id!(HostPathId, u16);
dense_id!(CmdId, u16);
dense_id!(SenseId, u16);
dense_id!(ThreadId, u16);

impl PortSlot {
    /// Construct a slot from a catalog-sized `u8`; the raw field remains private.
    pub const fn new(value: u8) -> Self {
        Self(value)
    }
}
