//! Dense runtime ids (D-id-space). Not engine Entity.

/// Dense knot index after validate/bind.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct KnotId(pub u16);

/// Port within a knot's closed table (D-port-schema).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct PortSlot(pub u8);

/// Interned SignalOut path (D-hostpath).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct HostPathId(pub u16);

/// Interned EmitCommand name (D-hostpath).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct CmdId(pub u16);

/// Optional diagnostics id.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct ThreadId(pub u16);

/// Host-owned PRNG seed (Random v1+).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct Seed(pub u64);

/// Core time: tick only (dt stays host-side).
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct HostTime {
    pub tick: u64,
}
