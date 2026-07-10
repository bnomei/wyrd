//! Bind-time dispatch tags for hot loom eval (no String clones per tick).

use wyrd_core::{CalcOp, CompareOp, FlagPriority, KnotKind, Signal, TimerMode};

/// Copyable dispatch tag — built once at bind.
#[derive(Clone, Copy, Debug)]
pub(crate) enum KindTag {
    Sense,
    Not,
    And { arity: u8 },
    Or { arity: u8 },
    RisingFromZero,
    Compare {
        op: CompareOp,
        rhs_const: Option<i32>,
    },
    Flag {
        priority: FlagPriority,
        enable_toggle: bool,
    },
    Counter,
    TimerPulseHold { ticks: u16 },
    TimerFedCountdown { ticks: u16 },
    Delay { ticks: u16 },
    Calc { op: CalcOp },
    Abs,
    Neg,
    Map {
        in_min: Signal,
        in_max: Signal,
        out_min: Signal,
        out_max: Signal,
    },
    Select,
    Digitize {
        steps: u16,
        in_min: Signal,
        in_max: Signal,
        out_min: Signal,
        out_max: Signal,
    },
    Threshold {
        high: Signal,
        low: Signal,
        use_hysteresis: bool,
    },
    Random { require_gate: bool },
    Sqrt,
    Xor,
    FallingToZero,
    Change,
    Clamp { min: Signal, max: Signal },
    SignalOut,
    EmitCommand,
}

impl KindTag {
    pub(crate) fn from_kind(k: &KnotKind) -> Self {
        match k {
            KnotKind::Constant { .. } | KnotKind::SignalIn | KnotKind::OnStart => KindTag::Sense,
            KnotKind::Not => KindTag::Not,
            KnotKind::And { arity } => KindTag::And { arity: *arity },
            KnotKind::Or { arity } => KindTag::Or { arity: *arity },
            KnotKind::RisingFromZero => KindTag::RisingFromZero,
            KnotKind::Compare { op, rhs_const } => KindTag::Compare {
                op: *op,
                rhs_const: *rhs_const,
            },
            KnotKind::Flag {
                priority,
                enable_toggle,
            } => KindTag::Flag {
                priority: *priority,
                enable_toggle: *enable_toggle,
            },
            KnotKind::Counter => KindTag::Counter,
            KnotKind::Timer { mode, ticks } => match mode {
                TimerMode::PulseHold => KindTag::TimerPulseHold { ticks: *ticks },
                TimerMode::FedCountdown => KindTag::TimerFedCountdown { ticks: *ticks },
            },
            KnotKind::Delay { ticks } => KindTag::Delay { ticks: *ticks },
            KnotKind::Calc { op } => KindTag::Calc { op: *op },
            KnotKind::Abs => KindTag::Abs,
            KnotKind::Neg => KindTag::Neg,
            KnotKind::Map {
                in_min,
                in_max,
                out_min,
                out_max,
            } => KindTag::Map {
                in_min: *in_min,
                in_max: *in_max,
                out_min: *out_min,
                out_max: *out_max,
            },
            KnotKind::Select => KindTag::Select,
            KnotKind::Digitize {
                steps,
                in_min,
                in_max,
                out_min,
                out_max,
            } => KindTag::Digitize {
                steps: *steps,
                in_min: *in_min,
                in_max: *in_max,
                out_min: *out_min,
                out_max: *out_max,
            },
            KnotKind::Threshold {
                high,
                low,
                use_hysteresis,
            } => KindTag::Threshold {
                high: *high,
                low: *low,
                use_hysteresis: *use_hysteresis,
            },
            KnotKind::Random { require_gate } => KindTag::Random {
                require_gate: *require_gate,
            },
            KnotKind::Sqrt => KindTag::Sqrt,
            KnotKind::Xor => KindTag::Xor,
            KnotKind::FallingToZero => KindTag::FallingToZero,
            KnotKind::Change => KindTag::Change,
            KnotKind::Clamp { min, max } => KindTag::Clamp {
                min: *min,
                max: *max,
            },
            KnotKind::SignalOut { .. } => KindTag::SignalOut,
            KnotKind::EmitCommand { .. } => KindTag::EmitCommand,
        }
    }

    #[inline]
    pub(crate) fn is_sense(self) -> bool {
        matches!(self, KindTag::Sense)
    }
}
