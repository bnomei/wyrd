//! Closed static port tables per [`KnotKind`] (D-port-schema).
//!
//! Author strings (`"in"`, `"out"`, …) resolve to dense [`PortSlot`] values
//! used by validate, builder endpoint checks, and bind. Unsupported And/Or
//! arities return an empty table so validate can reject them.

use crate::ids::PortSlot;
use crate::kind::{KnotKind, SignalDomain, TimerMode};

/// Direction of a catalog port relative to its knot.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PortDir {
    In,
    Out,
}

/// One entry in a kind's fixed port table.
#[derive(Copy, Clone, Debug)]
pub struct PortInfo {
    pub slot: PortSlot,
    pub dir: PortDir,
    pub name: &'static str,
    pub required: bool,
}

/// Graph-time domain constraint for one structural port.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PortDomain {
    /// The port always carries this domain.
    Fixed(SignalDomain),
    /// Ports with the same variable id on a knot must resolve identically.
    Variable(u8),
    /// The port accepts any already-resolved domain without constraining it.
    Any,
}

const fn pin(slot: u8, name: &'static str, required: bool) -> PortInfo {
    PortInfo {
        slot: PortSlot::new(slot),
        dir: PortDir::In,
        name,
        required,
    }
}

const fn pout(slot: u8, name: &'static str) -> PortInfo {
    PortInfo {
        slot: PortSlot::new(slot),
        dir: PortDir::Out,
        name,
        required: false,
    }
}

const OUT_ONLY: &[PortInfo] = &[pout(0, "out")];
const IN_OUT: &[PortInfo] = &[pin(0, "in", true), pout(1, "out")];
const COMPARE: &[PortInfo] = &[
    pin(0, "lhs", true),
    // Catalog marks required; validate relaxes when `rhs_const` is set.
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
const RANDOM: &[PortInfo] = &[
    pin(0, "min", false),
    pin(1, "max", false),
    pin(2, "gate", false),
    pout(3, "out"),
];
const MAP_LIKE: &[PortInfo] = &[pin(0, "in", true), pout(1, "out")];
const CONVERT: &[PortInfo] = &[pin(0, "in", true), pout(1, "out")];
const SIGNAL_OUT: &[PortInfo] = &[pin(0, "in", true)];
const EMIT: &[PortInfo] = &[
    pin(0, "trigger", true),
    pin(1, "enable", false),
    pin(2, "payload", false),
];

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

/// Static port table for a kind. Empty for unsupported And/Or arity.
pub fn ports_of(kind: &KnotKind) -> &'static [PortInfo] {
    match kind {
        KnotKind::Constant { .. } => OUT_ONLY,
        KnotKind::SignalIn { .. } => OUT_ONLY,
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
        KnotKind::Abs { .. } => MAP_LIKE,
        KnotKind::Neg { .. } => MAP_LIKE,
        KnotKind::Select => SELECT,
        KnotKind::Digitize { .. } => MAP_LIKE,
        KnotKind::Threshold { .. } => THRESHOLD,
        KnotKind::Random { .. } => RANDOM,
        KnotKind::Sqrt { .. } => MAP_LIKE,
        KnotKind::Xor => CALC,
        KnotKind::FallingToZero => IN_OUT,
        KnotKind::Change => IN_OUT,
        KnotKind::Clamp { .. } => MAP_LIKE,
        KnotKind::Convert { .. } => CONVERT,
        KnotKind::SignalOut { .. } => SIGNAL_OUT,
        KnotKind::EmitCommand { .. } => EMIT,
    }
}

/// Domain constraint for a structural port, or `None` when `slot` is invalid.
pub fn port_domain(kind: &KnotKind, slot: PortSlot) -> Option<PortDomain> {
    if !ports_of(kind).iter().any(|port| port.slot == slot) {
        return None;
    }

    let slot = slot.get();
    Some(match kind {
        KnotKind::Constant { domain, .. } | KnotKind::SignalIn { domain } => {
            PortDomain::Fixed(*domain)
        }
        KnotKind::OnStart
        | KnotKind::Not
        | KnotKind::And { .. }
        | KnotKind::Or { .. }
        | KnotKind::Flag { .. }
        | KnotKind::Timer { .. }
        | KnotKind::Xor => PortDomain::Fixed(SignalDomain::Bool),
        KnotKind::Compare { domain, .. } => {
            if slot == 2 {
                PortDomain::Fixed(SignalDomain::Bool)
            } else {
                PortDomain::Fixed(*domain)
            }
        }
        KnotKind::RisingFromZero | KnotKind::FallingToZero | KnotKind::Change => {
            if slot == 0 {
                PortDomain::Any
            } else {
                PortDomain::Fixed(SignalDomain::Bool)
            }
        }
        KnotKind::Counter => {
            if slot == 3 {
                PortDomain::Fixed(SignalDomain::Count)
            } else {
                PortDomain::Fixed(SignalDomain::Bool)
            }
        }
        KnotKind::Delay { .. } => PortDomain::Variable(0),
        KnotKind::Calc { domain, .. }
        | KnotKind::Map { domain, .. }
        | KnotKind::Abs { domain }
        | KnotKind::Neg { domain }
        | KnotKind::Digitize { domain, .. }
        | KnotKind::Sqrt { domain }
        | KnotKind::Clamp { domain, .. } => PortDomain::Fixed(*domain),
        KnotKind::Select => {
            if slot == 0 {
                PortDomain::Fixed(SignalDomain::Bool)
            } else {
                PortDomain::Variable(0)
            }
        }
        KnotKind::Threshold { domain, .. } => {
            if slot == 0 {
                PortDomain::Fixed(*domain)
            } else {
                PortDomain::Fixed(SignalDomain::Bool)
            }
        }
        KnotKind::Random { domain, .. } => {
            if slot == 2 {
                PortDomain::Fixed(SignalDomain::Bool)
            } else {
                PortDomain::Fixed(*domain)
            }
        }
        KnotKind::Convert { from, to } => {
            if slot == 0 {
                PortDomain::Fixed(*from)
            } else {
                PortDomain::Fixed(*to)
            }
        }
        KnotKind::SignalOut { domain, .. } => PortDomain::Fixed(*domain),
        KnotKind::EmitCommand { .. } => {
            if slot == 2 {
                PortDomain::Any
            } else {
                PortDomain::Fixed(SignalDomain::Bool)
            }
        }
    })
}

fn and_or_ports(arity: u8) -> &'static [PortInfo] {
    match arity {
        1 => AND1,
        2 => AND2,
        3 => AND3,
        4 => AND4,
        _ => &[],
    }
}

/// Resolve a catalog port name to its dense slot for the given kind.
pub fn port_slot(kind: &KnotKind, name: &str) -> Option<PortSlot> {
    ports_of(kind)
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kind::{CompareOp, TimerMode};

    #[test]
    fn and2_ports() {
        let k = KnotKind::and2();
        assert_eq!(port_slot(&k, "in_0"), Some(PortSlot::new(0)));
        assert_eq!(port_slot(&k, "in_1"), Some(PortSlot::new(1)));
        assert_eq!(port_slot(&k, "out"), Some(PortSlot::new(2)));
        assert_eq!(port_slot(&k, "a"), None);
    }

    #[test]
    fn emit_trigger_not_in() {
        let k = KnotKind::emit_command("x");
        assert_eq!(port_slot(&k, "trigger"), Some(PortSlot::new(0)));
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
                domain: SignalDomain::Level,
                in_min: crate::ZERO,
                in_max: crate::ONE,
                out_min: crate::ZERO,
                out_max: crate::ONE,
            })[0]
                .name,
            "in"
        );
        assert_eq!(ports_of(&KnotKind::abs(SignalDomain::Level))[1].name, "out");
        assert_eq!(ports_of(&KnotKind::neg(SignalDomain::Count))[0].name, "in");
        assert_eq!(
            port_slot(&KnotKind::select(), "sel"),
            Some(PortSlot::new(0))
        );
        assert_eq!(port_slot(&KnotKind::select(), "b"), Some(PortSlot::new(2)));
        assert_eq!(
            port_slot(&KnotKind::random(false, SignalDomain::Level), "gate"),
            Some(PortSlot::new(2))
        );
        assert_eq!(
            ports_of(&KnotKind::threshold_default(SignalDomain::Level))[2].name,
            "crossed_up"
        );
        assert_eq!(
            ports_of(&KnotKind::digitize(4, SignalDomain::Level))[0].name,
            "in"
        );
        assert_eq!(ports_of(&KnotKind::sqrt(SignalDomain::Count))[0].name, "in");
        assert_eq!(ports_of(&KnotKind::xor())[2].name, "out");
        assert_eq!(ports_of(&KnotKind::falling_to_zero())[1].name, "out");
        assert_eq!(ports_of(&KnotKind::change())[0].name, "in");
        assert_eq!(
            ports_of(&KnotKind::clamp(
                crate::ZERO,
                crate::ONE,
                SignalDomain::Level
            ))[0]
                .name,
            "in"
        );
        assert_eq!(
            ports_of(&KnotKind::constant(crate::ONE, SignalDomain::Bool))[0].name,
            "out"
        );
        assert_eq!(
            ports_of(&KnotKind::signal_in(SignalDomain::Bool))[0].name,
            "out"
        );
        assert_eq!(ports_of(&KnotKind::OnStart)[0].name, "out");
        assert_eq!(ports_of(&KnotKind::not())[0].name, "in");
        assert_eq!(ports_of(&KnotKind::rising_from_zero())[0].name, "in");
        let p = &ports_of(&KnotKind::not())[0];
        assert_eq!(p.dir, PortDir::In);
        assert!(p.required);
        assert_eq!(p.slot, PortSlot::new(0));
        let _in = pin(0, "x", true);
        let _out = pout(1, "y");
        assert_eq!(_out.dir, PortDir::Out);
        assert_eq!(_in.name, "x");
    }

    #[test]
    fn domain_constraints_cover_fixed_variable_and_any_ports() {
        let bool_in = KnotKind::signal_in(SignalDomain::Bool);
        assert_eq!(
            port_domain(&bool_in, PortSlot::new(0)),
            Some(PortDomain::Fixed(SignalDomain::Bool))
        );
        assert_eq!(port_domain(&bool_in, PortSlot::new(1)), None);

        let compare = KnotKind::compare(CompareOp::Eq, None, SignalDomain::Count);
        assert_eq!(
            port_domain(&compare, PortSlot::new(0)),
            Some(PortDomain::Fixed(SignalDomain::Count))
        );
        assert_eq!(
            port_domain(&compare, PortSlot::new(2)),
            Some(PortDomain::Fixed(SignalDomain::Bool))
        );

        let delay = KnotKind::Delay { ticks: 1 };
        assert_eq!(
            port_domain(&delay, PortSlot::new(0)),
            Some(PortDomain::Variable(0))
        );
        assert_eq!(
            port_domain(&delay, PortSlot::new(1)),
            Some(PortDomain::Variable(0))
        );

        let select = KnotKind::select();
        assert_eq!(
            port_domain(&select, PortSlot::new(0)),
            Some(PortDomain::Fixed(SignalDomain::Bool))
        );
        assert_eq!(
            port_domain(&select, PortSlot::new(3)),
            Some(PortDomain::Variable(0))
        );

        let change = KnotKind::change();
        assert_eq!(
            port_domain(&change, PortSlot::new(0)),
            Some(PortDomain::Any)
        );
        assert_eq!(
            port_domain(&change, PortSlot::new(1)),
            Some(PortDomain::Fixed(SignalDomain::Bool))
        );

        let emit = KnotKind::emit_command("x");
        assert_eq!(port_domain(&emit, PortSlot::new(2)), Some(PortDomain::Any));

        let convert = KnotKind::convert(SignalDomain::Count, SignalDomain::Level);
        assert_eq!(
            port_domain(&convert, PortSlot::new(0)),
            Some(PortDomain::Fixed(SignalDomain::Count))
        );
        assert_eq!(
            port_domain(&convert, PortSlot::new(1)),
            Some(PortDomain::Fixed(SignalDomain::Level))
        );
    }
}
