//! Closed [`KnotKind`] catalog and related op enums (D-dispatch).
//!
//! Author and asset form: host path and emit names stay open strings until
//! bind interns them. Runtime dispatch uses bind-time tags derived from these
//! variants rather than matching `KnotKind` every settle.

use crate::foundation::signal::Signal;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Which numeric wire path this weave was authored for.
///
/// Must match the crate feature selected at compile time (`signal-f32` or
/// `signal-i32`); validate rejects a mismatch.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NumericPath {
    /// `f32` wire representation (`signal-f32` builds).
    #[cfg_attr(feature = "serde", serde(rename = "f32"))]
    F32,
    /// Fixed-point i32 Q16 wire representation (`signal-i32` builds).
    #[cfg_attr(feature = "serde", serde(rename = "i32q16"))]
    I32Q16,
}

/// Semantic domain carried by a monomorphic [`Signal`] wire.
///
/// Domains are graph-time contracts. They do not change the runtime wire
/// representation selected by `signal-f32` or `signal-i32`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum SignalDomain {
    /// Exact false/true values (`ZERO`/`ONE`).
    Bool,
    /// Continuous numeric values.
    Level,
    /// Whole-number values.
    Count,
}

impl SignalDomain {
    /// Whether [`KnotKind`] numeric ops (Calc, Map, Threshold, …) may use this domain.
    pub const fn is_numeric(self) -> bool {
        matches!(self, SignalDomain::Level | SignalDomain::Count)
    }
}

impl NumericPath {
    /// Path encoded by the active cargo feature for this build.
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

/// Comparison operator for [`KnotKind::Compare`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CompareOp {
    /// `lhs` equals `rhs`.
    Eq,
    /// `lhs` does not equal `rhs`.
    Ne,
    /// `lhs` is strictly less than `rhs` (numeric domains only).
    Lt,
    /// `lhs` is less than or equal to `rhs` (numeric domains only).
    Lte,
    /// `lhs` is strictly greater than `rhs` (numeric domains only).
    Gt,
    /// `lhs` is greater than or equal to `rhs` (numeric domains only).
    Gte,
}

impl CompareOp {
    /// Whether this comparison is defined for `domain`.
    ///
    /// Boolean signals support equality only; numeric domains also support
    /// ordering comparisons.
    pub const fn supports_domain(self, domain: SignalDomain) -> bool {
        !matches!(domain, SignalDomain::Bool) || matches!(self, CompareOp::Eq | CompareOp::Ne)
    }
}

/// Timer behavior for [`KnotKind::Timer`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TimerMode {
    /// Countdown reloaded while the `feed` port stays truthy.
    FedCountdown,
    /// Hold `active` for `ticks` after a rising edge on `start`.
    PulseHold,
}

/// Binary arithmetic for [`KnotKind::Calc`] (prefer over path-local `signal_ops`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CalcOp {
    /// Saturating add of `a` and `b`.
    Add,
    /// Saturating subtract `b` from `a`.
    Sub,
    /// Multiply `a` and `b` (Level saturates; Count truncates toward zero).
    Mul,
    /// Divide `a` by `b` (Level float div; Count truncates toward zero).
    Div,
}

/// Simultaneous set/reset priority for [`KnotKind::Flag`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FlagPriority {
    /// Simultaneous set and reset clears the latch.
    ResetWins,
    /// Simultaneous set and reset holds the latch set.
    SetWins,
}

/// Author / asset knot kind. Host path and emit names stay open strings until bind.
///
/// Closed enum: port tables, validate, and loom dispatch all key off these
/// variants. Adding a kind requires catalog ports plus runtime eval.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum KnotKind {
    /// Fixed authored value seeded on `out` before topo eval.
    Constant {
        /// Domain contract for `out`.
        domain: SignalDomain,
        /// Literal emitted each settle.
        value: Signal,
    },
    /// Host sense source: loom copies the bound value onto `out` each settle.
    SignalIn {
        /// Expected domain of the host-bound signal.
        domain: SignalDomain,
    },
    /// One-shot truthy pulse on the first settle after bind.
    OnStart,
    /// Boolean invert: truthy `in` → falsey `out`, and vice versa.
    Not,
    /// Conjunction: `out` truthy only when every `in_*` port is truthy.
    And {
        /// Number of boolean inputs (`in_0` … `in_{arity-1}`).
        arity: u8,
    },
    /// Disjunction: `out` truthy when any `in_*` port is truthy.
    Or {
        /// Number of boolean inputs (`in_0` … `in_{arity-1}`).
        arity: u8,
    },
    /// Relational compare of `lhs` against `rhs` (or `rhs_const`) into boolean `out`.
    Compare {
        /// Domain shared by `lhs` and `rhs`.
        domain: SignalDomain,
        /// Comparison applied each settle.
        op: CompareOp,
        /// Domain-encoded fallback when the `rhs` port is unconnected.
        rhs_const: Option<Signal>,
    },
    /// One-tick pulse when `in` rises from falsey to truthy.
    RisingFromZero,
    /// Set/reset/toggle latch with configurable simultaneous priority.
    Flag {
        /// Tie-break when `set` and `reset` are both truthy in one settle.
        priority: FlagPriority,
        /// Rising edge on `toggle` flips the latch when true.
        enable_toggle: bool,
    },
    /// Saturating counter: rising `inc`/`dec`, level `reset` clears to zero.
    Counter,
    /// Boolean `active` from countdown or pulse-hold rune state.
    Timer {
        /// Countdown reload vs pulse-hold behavior.
        mode: TimerMode,
        /// Duration in loom settle ticks.
        ticks: u16,
    },
    /// Ring-buffer delay: `out` lags `in` by `ticks` settle passes.
    Delay {
        /// Delay depth in loom settle ticks.
        ticks: u16,
    },
    /// Binary arithmetic on `a` and `b` into `out`.
    Calc {
        /// Numeric domain for operands and result.
        domain: SignalDomain,
        /// Operation applied each settle.
        op: CalcOp,
    },
    /// Linear rescale of `in` across authored input and output ranges.
    Map {
        /// Numeric domain for `in` and `out`.
        domain: SignalDomain,
        /// Input range low endpoint (bind-time constant).
        in_min: Signal,
        /// Input range high endpoint (bind-time constant).
        in_max: Signal,
        /// Output range low endpoint (bind-time constant).
        out_min: Signal,
        /// Output range high endpoint (bind-time constant).
        out_max: Signal,
    },
    /// Absolute value of `in` in the declared domain.
    Abs {
        /// Numeric domain for `in` and `out`.
        domain: SignalDomain,
    },
    /// Negation of `in` in the declared domain.
    Neg {
        /// Numeric domain for `in` and `out`.
        domain: SignalDomain,
    },
    /// Multiplex: falsey `sel` → `a`, truthy `sel` → `b`.
    Select,
    /// Quantize `in` into `steps` bins over the in range, map to the out range.
    Digitize {
        /// Numeric domain for `in` and `out`.
        domain: SignalDomain,
        /// Bin count across the input span.
        steps: u16,
        /// Input range low endpoint (bind-time constant).
        in_min: Signal,
        /// Input range high endpoint (bind-time constant).
        in_max: Signal,
        /// Output range low endpoint (bind-time constant).
        out_min: Signal,
        /// Output range high endpoint (bind-time constant).
        out_max: Signal,
    },
    /// Gate a continuous signal with optional hysteresis; edge pulse outs.
    Threshold {
        /// Numeric domain for `in` and threshold constants.
        domain: SignalDomain,
        /// Upper crossing level (or sole threshold when hysteresis is off).
        high: Signal,
        /// Lower release level when hysteresis is on.
        low: Signal,
        /// Latch `out` between `low` and `high` instead of a single cutoff.
        use_hysteresis: bool,
    },
    /// Seeded PRNG sample into `[min, max]` ports; optional rising `gate`.
    Random {
        /// Numeric domain for sample and range ports.
        domain: SignalDomain,
        /// Resample only on a rising edge of `gate` when true.
        require_gate: bool,
    },
    /// Square root of `in` using the declared numeric domain's representation.
    Sqrt {
        /// Numeric domain for `in` and `out`.
        domain: SignalDomain,
    },
    /// Exclusive-or of two boolean inputs into `out`.
    Xor,
    /// One-tick pulse when `in` falls from truthy to falsey.
    FallingToZero,
    /// One-tick pulse when `in` truthiness changes in either direction.
    Change,
    /// Saturate `in` between authored `min` and `max`.
    Clamp {
        /// Numeric domain for `in`, bounds, and `out`.
        domain: SignalDomain,
        /// Lower clamp bound (bind-time constant).
        min: Signal,
        /// Upper clamp bound (bind-time constant).
        max: Signal,
    },
    /// Explicit conversion between two distinct signal domains.
    Convert {
        /// Source domain on `in`.
        from: SignalDomain,
        /// Target domain on `out`.
        to: SignalDomain,
    },
    /// Write `in` to a host-bound signal path each settle.
    SignalOut {
        /// Open host path string until bind interns it.
        path: std::string::String,
        /// Expected domain of the host-bound signal.
        domain: SignalDomain,
    },
    /// Queue a named host command when `trigger` is truthy.
    EmitCommand {
        /// Open command name string until bind interns it.
        name: std::string::String,
    },
}

impl KnotKind {
    /// Two-input And knot (`arity` 2).
    pub fn and2() -> Self {
        KnotKind::And { arity: 2 }
    }

    /// Two-input Or knot (`arity` 2).
    pub fn or2() -> Self {
        KnotKind::Or { arity: 2 }
    }

    /// Boolean Not knot.
    pub fn not() -> Self {
        KnotKind::Not
    }

    /// SignalIn sense source in `domain`.
    pub fn signal_in(domain: SignalDomain) -> Self {
        KnotKind::SignalIn { domain }
    }

    /// Constant source with explicit `value` and `domain`.
    pub fn constant(value: Signal, domain: SignalDomain) -> Self {
        KnotKind::Constant { domain, value }
    }

    /// Count-domain constant from whole number `n`.
    pub fn constant_count(n: i32) -> Self {
        KnotKind::Constant {
            domain: SignalDomain::Count,
            value: crate::foundation::signal::from_count(n),
        }
    }

    /// Bool-domain constant (`ONE` when true, `ZERO` when false).
    pub fn constant_bool(value: bool) -> Self {
        KnotKind::Constant {
            domain: SignalDomain::Bool,
            value: if value {
                crate::foundation::signal::ONE
            } else {
                crate::foundation::signal::ZERO
            },
        }
    }

    /// Level-domain constant from an author float (~0..=1).
    pub fn constant_level(value: f32) -> Self {
        KnotKind::Constant {
            domain: SignalDomain::Level,
            value: crate::foundation::signal::from_level(value),
        }
    }

    /// SignalOut sink bound to host `path` in `domain`.
    pub fn signal_out(path: impl Into<std::string::String>, domain: SignalDomain) -> Self {
        KnotKind::SignalOut {
            path: path.into(),
            domain,
        }
    }

    /// EmitCommand knot for host command `name`.
    pub fn emit_command(name: impl Into<std::string::String>) -> Self {
        KnotKind::EmitCommand { name: name.into() }
    }

    /// Rising-edge detector: pulse when `in` crosses from falsey to truthy.
    pub fn rising_from_zero() -> Self {
        KnotKind::RisingFromZero
    }

    /// Compare knot with `op`, optional baked-in `rhs_const`, in `domain`.
    pub fn compare(op: CompareOp, rhs_const: Option<Signal>, domain: SignalDomain) -> Self {
        KnotKind::Compare {
            domain,
            op,
            rhs_const,
        }
    }

    /// Saturating counter rune with default `inc`/`dec`/`reset` ports.
    pub fn counter() -> Self {
        KnotKind::Counter
    }

    /// Timer rune with `mode` behavior lasting `ticks` settle passes.
    pub fn timer(mode: TimerMode, ticks: u16) -> Self {
        KnotKind::Timer { mode, ticks }
    }

    /// Flag latch with simultaneous `priority` and optional `toggle` edge.
    pub fn flag(priority: FlagPriority, enable_toggle: bool) -> Self {
        KnotKind::Flag {
            priority,
            enable_toggle,
        }
    }

    /// Multiplex knot: falsey `sel` passes `a`, truthy `sel` passes `b`.
    pub fn select() -> Self {
        KnotKind::Select
    }

    /// Calc knot applying `op` in `domain`.
    pub fn calc(op: CalcOp, domain: SignalDomain) -> Self {
        KnotKind::Calc { domain, op }
    }

    /// Map knot with explicit input and output range endpoints.
    pub fn map(
        in_min: Signal,
        in_max: Signal,
        out_min: Signal,
        out_max: Signal,
        domain: SignalDomain,
    ) -> Self {
        KnotKind::Map {
            domain,
            in_min,
            in_max,
            out_min,
            out_max,
        }
    }

    /// Abs knot in `domain`.
    pub fn abs(domain: SignalDomain) -> Self {
        KnotKind::Abs { domain }
    }

    /// Neg knot in `domain`.
    pub fn neg(domain: SignalDomain) -> Self {
        KnotKind::Neg { domain }
    }

    /// Digitize with `steps` bins over 0..ONE → 0..ONE. Steps of 0 become 1.
    pub fn digitize(steps: u16, domain: SignalDomain) -> Self {
        KnotKind::Digitize {
            domain,
            steps: steps.max(1),
            in_min: crate::foundation::signal::ZERO,
            in_max: crate::foundation::signal::ONE,
            out_min: crate::foundation::signal::ZERO,
            out_max: crate::foundation::signal::ONE,
        }
    }

    /// Level thresholds use half-scale hysteresis; Count thresholds use 0/1.
    pub fn threshold_default(domain: SignalDomain) -> Self {
        if domain == SignalDomain::Count {
            return KnotKind::Threshold {
                domain,
                high: crate::foundation::signal::from_count(1),
                low: crate::foundation::signal::from_count(0),
                use_hysteresis: true,
            };
        }
        #[cfg(feature = "signal-f32")]
        {
            KnotKind::Threshold {
                domain,
                high: 0.5,
                low: 0.4,
                use_hysteresis: true,
            }
        }
        #[cfg(feature = "signal-i32")]
        {
            let one = crate::foundation::signal::ONE;
            KnotKind::Threshold {
                domain,
                high: one / 2,
                low: one * 2 / 5, // 0.4
                use_hysteresis: true,
            }
        }
    }

    /// Random sampler; resamples on rising `gate` when `require_gate` is true.
    pub fn random(require_gate: bool, domain: SignalDomain) -> Self {
        KnotKind::Random {
            domain,
            require_gate,
        }
    }

    /// Sqrt knot in `domain`.
    pub fn sqrt(domain: SignalDomain) -> Self {
        KnotKind::Sqrt { domain }
    }

    /// Boolean xor of `a` and `b`.
    pub fn xor() -> Self {
        KnotKind::Xor
    }

    /// Falling-edge detector: pulse when `in` crosses from truthy to falsey.
    pub fn falling_to_zero() -> Self {
        KnotKind::FallingToZero
    }

    /// Any-truthiness-change edge pulse on `in`.
    pub fn change() -> Self {
        KnotKind::Change
    }

    /// Clamp knot saturating `in` between `min` and `max` in `domain`.
    pub fn clamp(min: Signal, max: Signal, domain: SignalDomain) -> Self {
        KnotKind::Clamp { domain, min, max }
    }

    /// Cross-domain converter from `from` to `to` (must differ).
    pub fn convert(from: SignalDomain, to: SignalDomain) -> Self {
        KnotKind::Convert { from, to }
    }

    /// Whether all authored domain choices are legal for this knot kind.
    pub fn has_valid_domains(&self) -> bool {
        match self {
            KnotKind::Compare { domain, op, .. } => op.supports_domain(*domain),
            KnotKind::Calc { domain, .. }
            | KnotKind::Map { domain, .. }
            | KnotKind::Abs { domain }
            | KnotKind::Neg { domain }
            | KnotKind::Digitize { domain, .. }
            | KnotKind::Threshold { domain, .. }
            | KnotKind::Random { domain, .. }
            | KnotKind::Sqrt { domain }
            | KnotKind::Clamp { domain, .. } => domain.is_numeric(),
            KnotKind::Convert { from, to } => from != to,
            _ => true,
        }
    }

    /// And/Or input arity when applicable.
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
    use crate::foundation::signal::{from_count, from_level, ONE, ZERO};

    #[test]
    fn helpers_and_arity() {
        assert!(matches!(KnotKind::or2(), KnotKind::Or { arity: 2 }));
        assert!(matches!(KnotKind::not(), KnotKind::Not));
        assert!(matches!(
            KnotKind::constant_count(7),
            KnotKind::Constant { value, .. } if value == from_count(7)
        ));
        assert!(matches!(
            KnotKind::constant_bool(false),
            KnotKind::Constant {
                domain: SignalDomain::Bool,
                value: ZERO,
            }
        ));
        assert!(matches!(
            KnotKind::constant_level(0.25),
            KnotKind::Constant {
                domain: SignalDomain::Level,
                value,
            } if value == from_level(0.25)
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
        let _ = KnotKind::signal_in(SignalDomain::Bool);
        let _ = KnotKind::signal_out("p", SignalDomain::Bool);
        let _ = KnotKind::rising_from_zero();
        let _ = KnotKind::compare(CompareOp::Eq, None, SignalDomain::Bool);
        let _ = KnotKind::counter();
        let _ = KnotKind::timer(TimerMode::PulseHold, 1);
        let _ = KnotKind::flag(FlagPriority::SetWins, false);
        let _ = KnotKind::constant(ONE, SignalDomain::Bool);
        let _ = KnotKind::select();
        let _ = KnotKind::calc(CalcOp::Add, SignalDomain::Count);
        let _ = KnotKind::map(crate::ZERO, ONE, crate::ZERO, ONE, SignalDomain::Level);
        let _ = KnotKind::abs(SignalDomain::Level);
        let _ = KnotKind::neg(SignalDomain::Count);
        let _ = KnotKind::digitize(4, SignalDomain::Level);
        let _ = KnotKind::threshold_default(SignalDomain::Level);
        let _ = KnotKind::random(false, SignalDomain::Count);
        let _ = KnotKind::sqrt(SignalDomain::Count);
        let _ = KnotKind::xor();
        let _ = KnotKind::falling_to_zero();
        let _ = KnotKind::change();
        let _ = KnotKind::clamp(crate::ZERO, ONE, SignalDomain::Level);
        let _ = KnotKind::convert(SignalDomain::Count, SignalDomain::Level);
    }

    #[test]
    fn domain_legality_is_catalog_owned() {
        assert!(KnotKind::compare(CompareOp::Eq, None, SignalDomain::Bool).has_valid_domains());
        assert!(!KnotKind::compare(CompareOp::Lt, None, SignalDomain::Bool).has_valid_domains());
        assert!(KnotKind::calc(CalcOp::Mul, SignalDomain::Count).has_valid_domains());
        assert!(!KnotKind::calc(CalcOp::Mul, SignalDomain::Bool).has_valid_domains());
        assert!(KnotKind::convert(SignalDomain::Bool, SignalDomain::Level).has_valid_domains());
        assert!(!KnotKind::convert(SignalDomain::Bool, SignalDomain::Bool).has_valid_domains());
        assert!(KnotKind::signal_in(SignalDomain::Bool).has_valid_domains());

        let numeric_kinds = [
            KnotKind::map(ZERO, ONE, ZERO, ONE, SignalDomain::Level),
            KnotKind::abs(SignalDomain::Level),
            KnotKind::neg(SignalDomain::Count),
            KnotKind::digitize(2, SignalDomain::Level),
            KnotKind::threshold_default(SignalDomain::Level),
            KnotKind::random(false, SignalDomain::Count),
            KnotKind::sqrt(SignalDomain::Level),
            KnotKind::clamp(ZERO, ONE, SignalDomain::Count),
        ];
        assert!(numeric_kinds.iter().all(KnotKind::has_valid_domains));
    }
}
