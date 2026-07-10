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
        /// Bind-time `from_count` of KnotKind count const (None → wire rhs).
        rhs_const: Option<Signal>,
    },
    Flag {
        priority: FlagPriority,
        enable_toggle: bool,
    },
    Counter,
    TimerPulseHold { ticks: u16 },
    TimerFedCountdown { ticks: u16 },
    Delay { ticks: u16 },
    /// Split Calc ops into dedicated tags so hot chains monomorphize the op.
    CalcAdd,
    CalcSub,
    CalcMul,
    CalcDiv,
    Abs,
    Neg,
    /// Linear map with bind-time inv/scale.
    Map {
        degenerate: bool,
        in_min: Signal,
        out_min: Signal,
        /// f32: `1/(in_max-in_min)`; i32: unused.
        inv_in_span: Signal,
        /// f32: `out_max-out_min`; i32: unused (use `out_span_i64`).
        out_span: Signal,
        #[cfg(feature = "signal-i32")]
        den: i64,
        #[cfg(feature = "signal-i32")]
        out_span_i64: i64,
    },
    Select,
    /// Precomputed at bind so loom does not re-derive spans/scales each tick.
    Digitize {
        /// True when steps≤1 or zero in-span → always `out_min`.
        degenerate: bool,
        in_min: Signal,
        out_min: Signal,
        /// f32: `1/(in_max-in_min)`; i32: unused (use `den`).
        inv_in_span: Signal,
        /// f32: `(out_max-out_min)/(steps-1)`; i32: unused (use `out_span`/`last`).
        out_scale: Signal,
        steps: u16,
        last: u16,
        /// i32 path: `in_max - in_min` as i64 stored in two halves? Keep i64 den via Signal pair:
        /// den is i32-range for valid Q spans; large spans use i64 in digitize via recompute.
        /// Store den as i64-compatible: use `den_i64` only on i32 feature.
        #[cfg(feature = "signal-i32")]
        den: i64,
        #[cfg(feature = "signal-i32")]
        out_span: i64,
    },
    Threshold {
        high: Signal,
        low: Signal,
        use_hysteresis: bool,
    },
    Random {
        require_gate: bool,
        /// Bind-time: min/max ports wired (unconnected → ZERO / ONE).
        min_wired: bool,
        max_wired: bool,
    },
    Sqrt,
    Xor,
    FallingToZero,
    Change,
    Clamp { min: Signal, max: Signal },
    SignalOut,
    /// `enable_wired` set at bind from inbound CSR (not from KnotKind alone).
    EmitCommand { enable_wired: bool },
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
                rhs_const: rhs_const.map(wyrd_core::from_count),
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
            KnotKind::Calc { op } => match op {
                CalcOp::Add => KindTag::CalcAdd,
                CalcOp::Sub => KindTag::CalcSub,
                CalcOp::Mul => KindTag::CalcMul,
                CalcOp::Div => KindTag::CalcDiv,
            },
            KnotKind::Abs => KindTag::Abs,
            KnotKind::Neg => KindTag::Neg,
            KnotKind::Map {
                in_min,
                in_max,
                out_min,
                out_max,
            } => KindTag::map_precomputed(*in_min, *in_max, *out_min, *out_max),
            KnotKind::Select => KindTag::Select,
            KnotKind::Digitize {
                steps,
                in_min,
                in_max,
                out_min,
                out_max,
            } => KindTag::digitize_precomputed(*steps, *in_min, *in_max, *out_min, *out_max),
            KnotKind::Threshold {
                high,
                low,
                use_hysteresis,
            } => KindTag::Threshold {
                high: *high,
                low: *low,
                use_hysteresis: *use_hysteresis,
            },
            // min_wired/max_wired patched at bind from inbound edges.
            KnotKind::Random { require_gate } => KindTag::Random {
                require_gate: *require_gate,
                min_wired: false,
                max_wired: false,
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
            // enable_wired patched at bind from inbound edges.
            KnotKind::EmitCommand { .. } => KindTag::EmitCommand {
                enable_wired: false,
            },
        }
    }

    #[inline]
    pub(crate) fn is_sense(self) -> bool {
        matches!(self, KindTag::Sense)
    }

    pub(crate) fn map_precomputed(
        in_min: Signal,
        in_max: Signal,
        out_min: Signal,
        out_max: Signal,
    ) -> Self {
        #[cfg(feature = "signal-f32")]
        {
            let span_in = in_max - in_min;
            let degenerate = span_in.abs() < f32::EPSILON;
            KindTag::Map {
                degenerate,
                in_min,
                out_min,
                inv_in_span: if degenerate { 0.0 } else { 1.0 / span_in },
                out_span: out_max - out_min,
            }
        }
        #[cfg(feature = "signal-i32")]
        {
            let den = (in_max as i64) - (in_min as i64);
            KindTag::Map {
                degenerate: den == 0,
                in_min,
                out_min,
                inv_in_span: 0,
                out_span: 0,
                den,
                out_span_i64: (out_max as i64) - (out_min as i64),
            }
        }
    }

    pub(crate) fn digitize_precomputed(
        steps: u16,
        in_min: Signal,
        in_max: Signal,
        out_min: Signal,
        out_max: Signal,
    ) -> Self {
        let steps = steps.max(1);
        let last = steps.saturating_sub(1);
        #[cfg(feature = "signal-f32")]
        {
            let span_in = in_max - in_min;
            let degenerate = steps <= 1 || span_in.abs() < f32::EPSILON;
            let inv_in_span = if degenerate { 0.0 } else { 1.0 / span_in };
            let out_scale = if degenerate || last == 0 {
                0.0
            } else {
                (out_max - out_min) / (last as f32)
            };
            KindTag::Digitize {
                degenerate,
                in_min,
                out_min,
                inv_in_span,
                out_scale,
                steps,
                last,
            }
        }
        #[cfg(feature = "signal-i32")]
        {
            let den = (in_max as i64) - (in_min as i64);
            let out_span = (out_max as i64) - (out_min as i64);
            let degenerate = steps <= 1 || den == 0;
            KindTag::Digitize {
                degenerate,
                in_min,
                out_min,
                inv_in_span: 0, // unused on i32
                out_scale: 0,
                steps,
                last,
                den,
                out_span,
            }
        }
    }
}
