//! Bind-time dispatch tags for hot loom eval (no String clones per tick).
//!
//! Derived from [`KnotKind`] at bind, then patched with wiring facts (emit
//! enable, Random min/max ports, Calc Div-by-Constant). Map/Digitize precompute
//! spans so settle does not re-derive scales every tick.

use wyrd_core::{CalcOp, CompareOp, FlagPriority, KnotKind, Signal, SignalDomain, TimerMode};

/// Exact i32 affine-map execution plan built once at bind.
///
/// The input and output spans are reduced by their greatest common divisor before
/// settle. Both remaining factors fit in `u32`, so their per-tick product fits
/// in `u64` even for full-domain maps. That avoids `i128` helpers on 32-bit
/// constrained targets while retaining truncation-toward-zero for descending
/// output ranges.
#[cfg(feature = "signal-i32")]
#[derive(Clone, Copy, Debug)]
pub(crate) enum I32MapPlan {
    Constant {
        out_min: Signal,
    },
    Unit {
        in_min: Signal,
        out_min: Signal,
        den: u64,
        descending: bool,
    },
    Scale {
        in_min: Signal,
        out_min: Signal,
        den: u64,
        multiplier: u64,
        descending: bool,
    },
    Shift {
        in_min: Signal,
        out_min: Signal,
        den: u64,
        multiplier: u64,
        shift: u32,
        descending: bool,
    },
    Divide {
        in_min: Signal,
        out_min: Signal,
        den: u64,
        multiplier: u64,
        divisor: u64,
        descending: bool,
    },
}

#[cfg(feature = "signal-i32")]
impl I32MapPlan {
    fn from_ranges(in_min: Signal, in_max: Signal, out_min: Signal, out_max: Signal) -> Self {
        let den = ((in_max as i64) - (in_min as i64)) as u64;
        let span = (out_max as i64) - (out_min as i64);
        if den == 0 || span == 0 {
            return Self::Constant { out_min };
        }

        let descending = span < 0;
        let gcd = gcd_u64(den, span.unsigned_abs());
        let multiplier = span.unsigned_abs() / gcd;
        let divisor = den / gcd;

        if divisor == 1 {
            if multiplier == 1 {
                Self::Unit {
                    in_min,
                    out_min,
                    den,
                    descending,
                }
            } else {
                Self::Scale {
                    in_min,
                    out_min,
                    den,
                    multiplier,
                    descending,
                }
            }
        } else if divisor.is_power_of_two() {
            Self::Shift {
                in_min,
                out_min,
                den,
                multiplier,
                shift: divisor.trailing_zeros(),
                descending,
            }
        } else {
            Self::Divide {
                in_min,
                out_min,
                den,
                multiplier,
                divisor,
                descending,
            }
        }
    }

    #[inline]
    pub(crate) fn map(self, input: Signal) -> Signal {
        match self {
            Self::Constant { out_min } => out_min,
            Self::Unit {
                in_min,
                out_min,
                den,
                descending,
            } => finish_i32_map(out_min, map_offset(input, in_min, den), descending),
            Self::Scale {
                in_min,
                out_min,
                den,
                multiplier,
                descending,
            } => finish_i32_map(
                out_min,
                map_offset(input, in_min, den) * multiplier,
                descending,
            ),
            Self::Shift {
                in_min,
                out_min,
                den,
                multiplier,
                shift,
                descending,
            } => finish_i32_map(
                out_min,
                (map_offset(input, in_min, den) * multiplier) >> shift,
                descending,
            ),
            Self::Divide {
                in_min,
                out_min,
                den,
                multiplier,
                divisor,
                descending,
            } => finish_i32_map(
                out_min,
                map_offset(input, in_min, den) * multiplier / divisor,
                descending,
            ),
        }
    }
}

#[cfg(feature = "signal-i32")]
#[inline]
fn map_offset(input: Signal, in_min: Signal, den: u64) -> u64 {
    ((input as i64) - (in_min as i64)).clamp(0, den as i64) as u64
}

#[cfg(feature = "signal-i32")]
#[inline]
fn finish_i32_map(out_min: Signal, magnitude: u64, descending: bool) -> Signal {
    let mapped = if descending {
        (out_min as i64) - (magnitude as i64)
    } else {
        (out_min as i64) + (magnitude as i64)
    };
    debug_assert!((i32::MIN as i64..=i32::MAX as i64).contains(&mapped));
    mapped as i32
}

#[cfg(feature = "signal-i32")]
fn gcd_u64(mut lhs: u64, mut rhs: u64) -> u64 {
    while rhs != 0 {
        let remainder = lhs % rhs;
        lhs = rhs;
        rhs = remainder;
    }
    lhs
}

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
    /// Linear map with bind-time arithmetic plan.
    Map {
        domain: SignalDomain,
        #[cfg(feature = "signal-f32")]
        degenerate: bool,
        #[cfg(feature = "signal-f32")]
        in_min: Signal,
        #[cfg(feature = "signal-f32")]
        out_min: Signal,
        #[cfg(feature = "signal-f32")]
        /// `1/(in_max-in_min)` for the float path.
        inv_in_span: Signal,
        #[cfg(feature = "signal-f32")]
        /// `out_max-out_min` for the float path.
        out_span: Signal,
        #[cfg(feature = "signal-i32")]
        plan: I32MapPlan,
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
            KindTag::Map {
                domain,
                plan: I32MapPlan::from_ranges(in_min, in_max, out_min, out_max),
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
