//! Bind-time dispatch tags for hot loom eval (no String clones per tick).
//!
//! Derived from [`KnotKind`] at bind, then patched with wiring facts (emit
//! enable, Random min/max ports, Calc Div-by-Constant). Map/Digitize precompute
//! spans so settle does not re-derive scales every tick.

use wyrd_core::{CalcOp, CompareOp, FlagPriority, KnotKind, Signal, SignalDomain, TimerMode};

/// Copyable dispatch tag — built once at bind.
#[derive(Clone, Copy, Debug)]
pub(crate) enum KindTag {
    Sense,
    Not,
    And {
        arity: u8,
    },
    Or {
        arity: u8,
    },
    RisingFromZero,
    Compare {
        op: CompareOp,
        /// Domain-encoded authored fallback (`None` → wire rhs).
        rhs_const: Option<Signal>,
    },
    Flag {
        priority: FlagPriority,
        enable_toggle: bool,
    },
    Counter,
    TimerPulseHold {
        ticks: u16,
    },
    TimerFedCountdown {
        ticks: u16,
    },
    Delay {
        ticks: u16,
    },
    /// Split Calc ops into dedicated tags so hot chains monomorphize the op.
    CalcAdd {
        domain: SignalDomain,
    },
    CalcSub {
        domain: SignalDomain,
    },
    CalcMulLevel,
    CalcMulCount,
    CalcDivLevel,
    CalcDivCount,
    /// `b` is a Constant resolved at bind (common Div-by-ONE pattern).
    CalcDivLevelConst {
        divisor: Signal,
    },
    CalcDivCountConst {
        divisor: Signal,
    },
    Abs {
        domain: SignalDomain,
    },
    Neg {
        domain: SignalDomain,
    },
    /// Linear map with bind-time inv/scale.
    Map {
        domain: SignalDomain,
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
        domain: SignalDomain,
        /// True when steps≤1 or zero in-span → always `out_min`.
        degenerate: bool,
        in_min: Signal,
        out_min: Signal,
        /// f32: `steps/(in_max-in_min)` for bin index; i32: unused (use `den`).
        bin_scale: Signal,
        /// f32: `(out_max-out_min)/(steps-1)`; i32: unused (use `out_span`/`last`).
        out_scale: Signal,
        /// f32: `last as f32` for clamp; i32: unused.
        last_f: Signal,
        steps: u16,
        last: u16,
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
    RandomLevel {
        require_gate: bool,
        /// Bind-time: min/max ports wired (unconnected → ZERO / ONE).
        min_wired: bool,
        max_wired: bool,
    },
    RandomCount {
        require_gate: bool,
        min_wired: bool,
        max_wired: bool,
    },
    SqrtLevel,
    SqrtCount,
    ConvertBoolToLevel,
    ConvertBoolToCount,
    ConvertLevelToBool,
    ConvertLevelToCount,
    ConvertCountToBool,
    ConvertCountToLevel,
    ConvertIdentity,
    Xor,
    FallingToZero,
    Change,
    Clamp {
        min: Signal,
        max: Signal,
    },
    SignalOut,
    /// `enable_wired` set at bind from inbound CSR (not from KnotKind alone).
    EmitCommand {
        enable_wired: bool,
    },
}

impl KindTag {
    pub(crate) fn from_kind(k: &KnotKind) -> Self {
        match k {
            KnotKind::Constant { .. } | KnotKind::SignalIn { .. } | KnotKind::OnStart => {
                KindTag::Sense
            }
            KnotKind::Not => KindTag::Not,
            KnotKind::And { arity } => KindTag::And { arity: *arity },
            KnotKind::Or { arity } => KindTag::Or { arity: *arity },
            KnotKind::RisingFromZero => KindTag::RisingFromZero,
            KnotKind::Compare { op, rhs_const, .. } => KindTag::Compare {
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
            KnotKind::Calc { domain, op } => match (op, domain) {
                (CalcOp::Add, domain) => KindTag::CalcAdd { domain: *domain },
                (CalcOp::Sub, domain) => KindTag::CalcSub { domain: *domain },
                (CalcOp::Mul, SignalDomain::Count) => KindTag::CalcMulCount,
                (CalcOp::Mul, _) => KindTag::CalcMulLevel,
                (CalcOp::Div, SignalDomain::Count) => KindTag::CalcDivCount,
                (CalcOp::Div, _) => KindTag::CalcDivLevel,
            },
            KnotKind::Abs { domain } => KindTag::Abs { domain: *domain },
            KnotKind::Neg { domain } => KindTag::Neg { domain: *domain },
            KnotKind::Map {
                domain,
                in_min,
                in_max,
                out_min,
                out_max,
                ..
            } => KindTag::map_precomputed(*domain, *in_min, *in_max, *out_min, *out_max),
            KnotKind::Select => KindTag::Select,
            KnotKind::Digitize {
                domain,
                steps,
                in_min,
                in_max,
                out_min,
                out_max,
                ..
            } => {
                KindTag::digitize_precomputed(*domain, *steps, *in_min, *in_max, *out_min, *out_max)
            }
            KnotKind::Threshold {
                high,
                low,
                use_hysteresis,
                ..
            } => KindTag::Threshold {
                high: *high,
                low: *low,
                use_hysteresis: *use_hysteresis,
            },
            KnotKind::Random {
                domain,
                require_gate,
            } => match domain {
                SignalDomain::Count => KindTag::RandomCount {
                    require_gate: *require_gate,
                    min_wired: false,
                    max_wired: false,
                },
                _ => KindTag::RandomLevel {
                    require_gate: *require_gate,
                    min_wired: false,
                    max_wired: false,
                },
            },
            KnotKind::Sqrt { domain } => match domain {
                SignalDomain::Count => KindTag::SqrtCount,
                _ => KindTag::SqrtLevel,
            },
            KnotKind::Xor => KindTag::Xor,
            KnotKind::FallingToZero => KindTag::FallingToZero,
            KnotKind::Change => KindTag::Change,
            KnotKind::Clamp { min, max, .. } => KindTag::Clamp {
                min: *min,
                max: *max,
            },
            KnotKind::Convert { from, to } => match (from, to) {
                (SignalDomain::Bool, SignalDomain::Level) => KindTag::ConvertBoolToLevel,
                (SignalDomain::Bool, SignalDomain::Count) => KindTag::ConvertBoolToCount,
                (SignalDomain::Level, SignalDomain::Bool) => KindTag::ConvertLevelToBool,
                (SignalDomain::Level, SignalDomain::Count) => KindTag::ConvertLevelToCount,
                (SignalDomain::Count, SignalDomain::Bool) => KindTag::ConvertCountToBool,
                (SignalDomain::Count, SignalDomain::Level) => KindTag::ConvertCountToLevel,
                _ => KindTag::ConvertIdentity,
            },
            KnotKind::SignalOut { .. } => KindTag::SignalOut,
            KnotKind::EmitCommand { .. } => KindTag::EmitCommand {
                enable_wired: false,
            },
        }
    }

    pub(crate) fn with_random_wiring(self, min_wired: bool, max_wired: bool) -> Self {
        match self {
            KindTag::RandomLevel { require_gate, .. } => KindTag::RandomLevel {
                require_gate,
                min_wired,
                max_wired,
            },
            KindTag::RandomCount { require_gate, .. } => KindTag::RandomCount {
                require_gate,
                min_wired,
                max_wired,
            },
            other => other,
        }
    }

    pub(crate) fn calc_div_const(domain: SignalDomain, divisor: Signal) -> Self {
        match domain {
            SignalDomain::Count => KindTag::CalcDivCountConst { divisor },
            _ => KindTag::CalcDivLevelConst { divisor },
        }
    }

    #[inline]
    pub(crate) fn is_sense(self) -> bool {
        matches!(self, KindTag::Sense)
    }

    pub(crate) fn map_precomputed(
        domain: SignalDomain,
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
                domain,
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
                domain,
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
        domain: SignalDomain,
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
            let bin_scale = if degenerate {
                0.0
            } else {
                (steps as f32) / span_in
            };
            let out_scale = if degenerate || last == 0 {
                0.0
            } else {
                (out_max - out_min) / (last as f32)
            };
            KindTag::Digitize {
                domain,
                degenerate,
                in_min,
                out_min,
                bin_scale,
                out_scale,
                last_f: last as f32,
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
                domain,
                degenerate,
                in_min,
                out_min,
                bin_scale: 0,
                out_scale: 0,
                last_f: 0,
                steps,
                last,
                den,
                out_span,
            }
        }
    }
}
