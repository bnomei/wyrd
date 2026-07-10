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

const fn pin(slot: u8, name: &'static str, required: bool) -> PortInfo {
    PortInfo {
        slot: PortSlot(slot),
        dir: PortDir::In,
        name,
        required,
    }
}

const fn pout(slot: u8, name: &'static str) -> PortInfo {
    PortInfo {
        slot: PortSlot(slot),
        dir: PortDir::Out,
        name,
        required: false,
    }
}

const OUT_ONLY: &[PortInfo] = &[pout(0, "out")];
const IN_OUT: &[PortInfo] = &[pin(0, "in", true), pout(1, "out")];
const COMPARE: &[PortInfo] = &[
    pin(0, "lhs", true),
    // Catalog-required; validate relaxes when `rhs_const` is Some.
    pin(1, "rhs", true),
    pout(2, "out"),
];
const FLAG: &[PortInfo] = &[
    pin(0, "set", false),
    pin(1, "reset", false),
    pin(2, "toggle", false),
    pout(3, "out"),
];
const COUNTER: &[PortInfo] = &[
    pin(0, "inc", false),
    pin(1, "dec", false),
    pin(2, "reset", false),
    pout(3, "count"),
];
const TIMER_PULSE: &[PortInfo] = &[pin(0, "start", true), pout(1, "active")];
const TIMER_FED: &[PortInfo] = &[pin(0, "feed", true), pout(1, "active")];
const CALC: &[PortInfo] = &[pin(0, "a", true), pin(1, "b", true), pout(2, "out")];
const SELECT: &[PortInfo] = &[
    pin(0, "sel", true),
    pin(1, "a", true),
    pin(2, "b", true),
    pout(3, "out"),
];
const THRESHOLD: &[PortInfo] = &[
    pin(0, "in", true),
    pout(1, "out"),
    pout(2, "crossed_up"),
    pout(3, "crossed_down"),
];
const MAP_LIKE: &[PortInfo] = &[pin(0, "in", true), pout(1, "out")];
const SIGNAL_OUT: &[PortInfo] = &[pin(0, "in", true)];
const EMIT: &[PortInfo] = &[
    pin(0, "trigger", true),
    pin(1, "enable", false),
    pin(2, "payload", false),
];

// Precomputed And/Or arities 1..=4
const AND1: &[PortInfo] = &[pin(0, "in_0", true), pout(1, "out")];
const AND2: &[PortInfo] = &[pin(0, "in_0", true), pin(1, "in_1", true), pout(2, "out")];
const AND3: &[PortInfo] = &[
    pin(0, "in_0", true),
    pin(1, "in_1", true),
    pin(2, "in_2", true),
    pout(3, "out"),
];
const AND4: &[PortInfo] = &[
    pin(0, "in_0", true),
    pin(1, "in_1", true),
    pin(2, "in_2", true),
    pin(3, "in_3", true),
    pout(4, "out"),
];

/// Static port table for a kind. Empty for unsupported arity.
pub fn ports_of(kind: &KnotKind) -> &'static [PortInfo] {
    match kind {
        KnotKind::Constant { .. } => OUT_ONLY,
        KnotKind::SignalIn => OUT_ONLY,
        KnotKind::OnStart => OUT_ONLY,
        KnotKind::Not => IN_OUT,
        KnotKind::RisingFromZero => IN_OUT,
        KnotKind::And { arity } => and_or_ports(*arity),
        KnotKind::Or { arity } => and_or_ports(*arity),
        KnotKind::Compare { .. } => COMPARE,
        KnotKind::Flag { .. } => FLAG,
        KnotKind::Counter => COUNTER,
        KnotKind::Timer { mode, .. } => match mode {
            TimerMode::PulseHold => TIMER_PULSE,
            TimerMode::FedCountdown => TIMER_FED,
        },
        KnotKind::Delay { .. } => IN_OUT,
        KnotKind::Calc { .. } => CALC,
        KnotKind::Map { .. } => MAP_LIKE,
        KnotKind::Abs => MAP_LIKE,
        KnotKind::Neg => MAP_LIKE,
        KnotKind::Select => SELECT,
        KnotKind::Digitize { .. } => MAP_LIKE,
        KnotKind::Threshold { .. } => THRESHOLD,
        KnotKind::SignalOut { .. } => SIGNAL_OUT,
        KnotKind::EmitCommand { .. } => EMIT,
    }
}

fn and_or_ports(arity: u8) -> &'static [PortInfo] {
    match arity {
        1 => AND1,
        2 => AND2,
        3 => AND3,
        4 => AND4,
        _ => &[], // validate will reject unsupported arity for now
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
    use crate::kind::TimerMode;

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

    #[test]
    fn all_arity_and_timer_tables() {
        assert_eq!(ports_of(&KnotKind::And { arity: 1 }).len(), 2);
        assert_eq!(ports_of(&KnotKind::And { arity: 3 }).len(), 4);
        assert_eq!(ports_of(&KnotKind::Or { arity: 4 }).len(), 5);
        assert!(ports_of(&KnotKind::And { arity: 5 }).is_empty());
        assert!(ports_of(&KnotKind::Or { arity: 9 }).is_empty());
        assert_eq!(
            ports_of(&KnotKind::timer(TimerMode::FedCountdown, 1))[0].name,
            "feed"
        );
        assert_eq!(
            ports_of(&KnotKind::timer(TimerMode::PulseHold, 1))[0].name,
            "start"
        );
        assert_eq!(
            ports_of(&KnotKind::Map {
                in_min: crate::ZERO,
                in_max: crate::ONE,
                out_min: crate::ZERO,
                out_max: crate::ONE,
            })[0]
            .name,
            "in"
        );
        assert_eq!(ports_of(&KnotKind::Abs)[1].name, "out");
        assert_eq!(ports_of(&KnotKind::Neg)[0].name, "in");
        assert_eq!(port_slot(&KnotKind::select(), "sel"), Some(PortSlot(0)));
        assert_eq!(port_slot(&KnotKind::select(), "b"), Some(PortSlot(2)));
        assert_eq!(ports_of(&KnotKind::constant(crate::ONE))[0].name, "out");
        assert_eq!(ports_of(&KnotKind::signal_in())[0].name, "out");
        assert_eq!(ports_of(&KnotKind::OnStart)[0].name, "out");
        assert_eq!(ports_of(&KnotKind::not())[0].name, "in");
        assert_eq!(ports_of(&KnotKind::rising_from_zero())[0].name, "in");
        // Touch PortDir / PortInfo fields + runtime const-fn paths (coverage).
        let p = &ports_of(&KnotKind::not())[0];
        assert_eq!(p.dir, PortDir::In);
        assert!(p.required);
        assert_eq!(p.slot, PortSlot(0));
        let _in = pin(0, "x", true);
        let _out = pout(1, "y");
        assert_eq!(_out.dir, PortDir::Out);
        assert_eq!(_in.name, "x");
    }
}
