//! Closed KnotKind enum (D-dispatch).

use crate::signal::Signal;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NumericPath {
    #[cfg_attr(feature = "serde", serde(rename = "f32"))]
    F32,
    #[cfg_attr(feature = "serde", serde(rename = "i32q16"))]
    I32Q16,
}

impl NumericPath {
    pub fn compiled() -> Self {
        #[cfg(feature = "signal-f32")]
        {
            NumericPath::F32
        }
        #[cfg(feature = "signal-i32")]
        {
            NumericPath::I32Q16
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TimerMode {
    FedCountdown,
    PulseHold,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CalcOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FlagPriority {
    ResetWins,
    SetWins,
}

/// Author / asset form. HostPath and Emit names stay open strings until bind.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum KnotKind {
    Constant {
        value: Signal,
    },
    SignalIn,
    OnStart,
    Not,
    And {
        arity: u8,
    },
    Or {
        arity: u8,
    },
    Compare {
        op: CompareOp,
        /// Whole-unit rhs when `rhs` port unconnected.
        rhs_const: Option<i32>,
    },
    RisingFromZero,
    Flag {
        priority: FlagPriority,
        enable_toggle: bool,
    },
    Counter,
    Timer {
        mode: TimerMode,
        ticks: u16,
    },
    Delay {
        ticks: u16,
    },
    Calc {
        op: CalcOp,
    },
    Map {
        in_min: Signal,
        in_max: Signal,
        out_min: Signal,
        out_max: Signal,
    },
    Abs,
    Neg,
    /// Multiplex: falsey sel → a, truthy sel → b.
    Select,
    /// Quantize `in` into `steps` bins over in range, map to out range.
    Digitize {
        steps: u16,
        in_min: Signal,
        in_max: Signal,
        out_min: Signal,
        out_max: Signal,
    },
    /// Gate continuous signal with optional hysteresis; edge pulse outs.
    Threshold {
        high: Signal,
        low: Signal,
        use_hysteresis: bool,
    },
    /// Seeded PRNG sample into [min,max] ports; optional rising `gate`.
    Random {
        require_gate: bool,
    },
    SignalOut {
        path: std::string::String,
    },
    EmitCommand {
        name: std::string::String,
    },
}

impl KnotKind {
    pub fn and2() -> Self {
        KnotKind::And { arity: 2 }
    }

    pub fn or2() -> Self {
        KnotKind::Or { arity: 2 }
    }

    pub fn not() -> Self {
        KnotKind::Not
    }

    pub fn signal_in() -> Self {
        KnotKind::SignalIn
    }

    pub fn constant(value: Signal) -> Self {
        KnotKind::Constant { value }
    }

    pub fn constant_count(n: i32) -> Self {
        KnotKind::Constant {
            value: crate::signal::from_count(n),
        }
    }

    pub fn signal_out(path: impl Into<std::string::String>) -> Self {
        KnotKind::SignalOut { path: path.into() }
    }

    pub fn emit_command(name: impl Into<std::string::String>) -> Self {
        KnotKind::EmitCommand { name: name.into() }
    }

    pub fn rising_from_zero() -> Self {
        KnotKind::RisingFromZero
    }

    pub fn compare(op: CompareOp, rhs_const: Option<i32>) -> Self {
        KnotKind::Compare { op, rhs_const }
    }

    pub fn counter() -> Self {
        KnotKind::Counter
    }

    pub fn timer(mode: TimerMode, ticks: u16) -> Self {
        KnotKind::Timer { mode, ticks }
    }

    pub fn flag(priority: FlagPriority, enable_toggle: bool) -> Self {
        KnotKind::Flag {
            priority,
            enable_toggle,
        }
    }

    pub fn select() -> Self {
        KnotKind::Select
    }

    /// Digitize with `steps` bins over 0..ONE → 0..ONE. Steps of 0 become 1.
    pub fn digitize(steps: u16) -> Self {
        KnotKind::Digitize {
            steps: steps.max(1),
            in_min: crate::signal::ZERO,
            in_max: crate::signal::ONE,
            out_min: crate::signal::ZERO,
            out_max: crate::signal::ONE,
        }
    }

    /// Threshold at half-scale with mild hysteresis (low=0.4·ONE, high=0.5·ONE on f32).
    pub fn threshold_default() -> Self {
        #[cfg(feature = "signal-f32")]
        {
            KnotKind::Threshold {
                high: 0.5,
                low: 0.4,
                use_hysteresis: true,
            }
        }
        #[cfg(feature = "signal-i32")]
        {
            let one = crate::signal::ONE;
            KnotKind::Threshold {
                high: one / 2,
                low: one * 2 / 5, // 0.4
                use_hysteresis: true,
            }
        }
    }

    pub fn random(require_gate: bool) -> Self {
        KnotKind::Random { require_gate }
    }

    pub fn arity(&self) -> Option<u8> {
        match self {
            KnotKind::And { arity } => Some(*arity),
            KnotKind::Or { arity } => Some(*arity),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::{from_count, ONE};

    #[test]
    fn helpers_and_arity() {
        assert!(matches!(KnotKind::or2(), KnotKind::Or { arity: 2 }));
        assert!(matches!(KnotKind::not(), KnotKind::Not));
        assert!(matches!(
            KnotKind::constant_count(7),
            KnotKind::Constant { value } if value == from_count(7)
        ));
        assert!(matches!(
            KnotKind::emit_command("go"),
            KnotKind::EmitCommand { name } if name == "go"
        ));
        assert_eq!(KnotKind::and2().arity(), Some(2));
        assert_eq!(KnotKind::or2().arity(), Some(2));
        assert_eq!(KnotKind::not().arity(), None);
        assert_eq!(NumericPath::compiled(), NumericPath::compiled());
        let _ = ONE;
        let _ = KnotKind::signal_in();
        let _ = KnotKind::signal_out("p");
        let _ = KnotKind::rising_from_zero();
        let _ = KnotKind::compare(CompareOp::Eq, None);
        let _ = KnotKind::counter();
        let _ = KnotKind::timer(TimerMode::PulseHold, 1);
        let _ = KnotKind::flag(FlagPriority::SetWins, false);
        let _ = KnotKind::constant(ONE);
        let _ = KnotKind::select();
        let _ = KnotKind::digitize(4);
        let _ = KnotKind::threshold_default();
        let _ = KnotKind::random(false);
    }
}

