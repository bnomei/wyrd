//! Closed port tables per KnotKind (D-port-schema).

use crate::ids::PortSlot;
use crate::kind::{KnotKind, TimerMode};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PortDir {
    In,
    Out,
}

#[derive(Copy, Clone, Debug)]
pub struct PortInfo {
    pub slot: PortSlot,
    pub dir: PortDir,
    pub name: &'static str,
    pub required: bool,
}

macro_rules! p {
    ($slot:expr, $name:expr, In, $req:expr) => {
        PortInfo {
            slot: PortSlot($slot),
            dir: PortDir::In,
            name: $name,
            required: $req,
        }
    };
    ($slot:expr, $name:expr, Out) => {
        PortInfo {
            slot: PortSlot($slot),
            dir: PortDir::Out,
            name: $name,
            required: false,
        }
    };
}

const OUT_ONLY: &[PortInfo] = &[p!(0, "out", Out)];
const IN_OUT: &[PortInfo] = &[p!(0, "in", In, true), p!(1, "out", Out)];
const COMPARE: &[PortInfo] = &[
    p!(0, "lhs", In, true),
    p!(1, "rhs", In, false), // required unless rhs_const
    p!(2, "out", Out),
];
const FLAG: &[PortInfo] = &[
    p!(0, "set", In, false),
    p!(1, "reset", In, false),
    p!(2, "toggle", In, false),
    p!(3, "out", Out),
];
const COUNTER: &[PortInfo] = &[
    p!(0, "inc", In, false),
    p!(1, "dec", In, false),
    p!(2, "reset", In, false),
    p!(3, "count", Out),
];
const TIMER_PULSE: &[PortInfo] = &[p!(0, "start", In, true), p!(1, "active", Out)];
const TIMER_FED: &[PortInfo] = &[p!(0, "feed", In, true), p!(1, "active", Out)];
const CALC: &[PortInfo] = &[
    p!(0, "a", In, true),
    p!(1, "b", In, true),
    p!(2, "out", Out),
];
const MAP_LIKE: &[PortInfo] = &[p!(0, "in", In, true), p!(1, "out", Out)];
const SIGNAL_OUT: &[PortInfo] = &[p!(0, "in", In, true)];
const EMIT: &[PortInfo] = &[
    p!(0, "trigger", In, true),
    p!(1, "enable", In, false),
    p!(2, "payload", In, false),
];

// Precomputed And/Or arities 1..=8
const AND1: &[PortInfo] = &[p!(0, "in_0", In, true), p!(1, "out", Out)];
const AND2: &[PortInfo] = &[
    p!(0, "in_0", In, true),
    p!(1, "in_1", In, true),
    p!(2, "out", Out),
];
const AND3: &[PortInfo] = &[
    p!(0, "in_0", In, true),
    p!(1, "in_1", In, true),
    p!(2, "in_2", In, true),
    p!(3, "out", Out),
];
const AND4: &[PortInfo] = &[
    p!(0, "in_0", In, true),
    p!(1, "in_1", In, true),
    p!(2, "in_2", In, true),
    p!(3, "in_3", In, true),
    p!(4, "out", Out),
];

/// Static port table for a kind. Empty for unsupported arity.
pub fn ports_of(kind: &KnotKind) -> &'static [PortInfo] {
    match kind {
        KnotKind::Constant { .. } | KnotKind::SignalIn | KnotKind::OnStart => OUT_ONLY,
        KnotKind::Not | KnotKind::RisingFromZero => IN_OUT,
        KnotKind::And { arity } | KnotKind::Or { arity } => match *arity {
            1 => AND1,
            2 => AND2,
            3 => AND3,
            4 => AND4,
            _ => &[], // validate will reject unsupported arity for now
        },
        KnotKind::Compare { .. } => COMPARE,
        KnotKind::Flag { .. } => FLAG,
        KnotKind::Counter => COUNTER,
        KnotKind::Timer { mode, .. } => match mode {
            TimerMode::PulseHold => TIMER_PULSE,
            TimerMode::FedCountdown => TIMER_FED,
        },
        KnotKind::Delay { .. } => IN_OUT,
        KnotKind::Calc { .. } => CALC,
        KnotKind::Map { .. } | KnotKind::Abs | KnotKind::Neg => MAP_LIKE,
        KnotKind::SignalOut { .. } => SIGNAL_OUT,
        KnotKind::EmitCommand { .. } => EMIT,
    }
}

/// Resolve catalog port name → slot.
pub fn port_slot(kind: &KnotKind, name: &str) -> Option<PortSlot> {
    ports_of(kind)
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.slot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn and2_ports() {
        let k = KnotKind::and2();
        assert_eq!(port_slot(&k, "in_0"), Some(PortSlot(0)));
        assert_eq!(port_slot(&k, "in_1"), Some(PortSlot(1)));
        assert_eq!(port_slot(&k, "out"), Some(PortSlot(2)));
        assert_eq!(port_slot(&k, "a"), None);
    }

    #[test]
    fn emit_trigger_not_in() {
        let k = KnotKind::emit_command("x");
        assert_eq!(port_slot(&k, "trigger"), Some(PortSlot(0)));
        assert_eq!(port_slot(&k, "in"), None);
    }
}
