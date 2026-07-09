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

    pub fn arity(&self) -> Option<u8> {
        match self {
            KnotKind::And { arity } | KnotKind::Or { arity } => Some(*arity),
            _ => None,
        }
    }
}
