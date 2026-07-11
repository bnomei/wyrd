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
    #[cfg_attr(feature = "serde", serde(rename = "f32"))]
    F32,
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
    /// Whether this domain supports numeric knot operations.
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
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
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
    Add,
    Sub,
    Mul,
    Div,
}

/// Simultaneous set/reset priority for [`KnotKind::Flag`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FlagPriority {
    ResetWins,
    SetWins,
}

/// Author / asset knot kind. Host path and emit names stay open strings until bind.
///
/// Closed enum: port tables, validate, and loom dispatch all key off these
/// variants. Adding a kind requires catalog ports plus runtime eval.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum KnotKind {
    Constant {
        domain: SignalDomain,
        value: Signal,
    },
    SignalIn {
        domain: SignalDomain,
    },
    OnStart,
    Not,
    And {
        arity: u8,
    },
    Or {
        arity: u8,
    },
    Compare {
        domain: SignalDomain,
        op: CompareOp,
        /// Domain-encoded fallback when the `rhs` port is unconnected.
        rhs_const: Option<Signal>,
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
        domain: SignalDomain,
        op: CalcOp,
    },
    Map {
        domain: SignalDomain,
        in_min: Signal,
        in_max: Signal,
        out_min: Signal,
        out_max: Signal,
    },
    Abs {
        domain: SignalDomain,
    },
    Neg {
        domain: SignalDomain,
    },
    /// Multiplex: falsey `sel` → `a`, truthy `sel` → `b`.
    Select,
    /// Quantize `in` into `steps` bins over in range, map to out range.
    Digitize {
        domain: SignalDomain,
        steps: u16,
        in_min: Signal,
        in_max: Signal,
        out_min: Signal,
        out_max: Signal,
    },
    /// Gate a continuous signal with optional hysteresis; edge pulse outs.
    Threshold {
        domain: SignalDomain,
        high: Signal,
        low: Signal,
        use_hysteresis: bool,
    },
    /// Seeded PRNG sample into `[min,max]` ports; optional rising `gate`.
    Random {
        domain: SignalDomain,
        require_gate: bool,
    },
    /// Square root using the declared numeric domain's representation.
    Sqrt {
        domain: SignalDomain,
    },
    Xor,
    FallingToZero,
    Change,
    Clamp {
        domain: SignalDomain,
        min: Signal,
        max: Signal,
    },
    /// Explicit conversion between two distinct signal domains.
    Convert {
        from: SignalDomain,
        to: SignalDomain,
    },
    SignalOut {
        path: std::string::String,
        domain: SignalDomain,
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

    pub fn signal_in(domain: SignalDomain) -> Self {
        KnotKind::SignalIn { domain }
    }

    pub fn constant(value: Signal, domain: SignalDomain) -> Self {
        KnotKind::Constant { domain, value }
    }

    pub fn constant_count(n: i32) -> Self {
        KnotKind::Constant {
            domain: SignalDomain::Count,
            value: crate::foundation::signal::from_count(n),
        }
    }

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

    pub fn constant_level(value: f32) -> Self {
        KnotKind::Constant {
            domain: SignalDomain::Level,
            value: crate::foundation::signal::from_level(value),
        }
    }

    pub fn signal_out(path: impl Into<std::string::String>, domain: SignalDomain) -> Self {
        KnotKind::SignalOut {
            path: path.into(),
            domain,
        }
    }

    pub fn emit_command(name: impl Into<std::string::String>) -> Self {
        KnotKind::EmitCommand { name: name.into() }
    }

    pub fn rising_from_zero() -> Self {
        KnotKind::RisingFromZero
    }

    pub fn compare(op: CompareOp, rhs_const: Option<Signal>, domain: SignalDomain) -> Self {
        KnotKind::Compare {
            domain,
            op,
            rhs_const,
        }
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

    pub fn calc(op: CalcOp, domain: SignalDomain) -> Self {
        KnotKind::Calc { domain, op }
    }

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

    pub fn abs(domain: SignalDomain) -> Self {
        KnotKind::Abs { domain }
    }

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

    pub fn random(require_gate: bool, domain: SignalDomain) -> Self {
        KnotKind::Random {
            domain,
            require_gate,
        }
    }

    pub fn sqrt(domain: SignalDomain) -> Self {
        KnotKind::Sqrt { domain }
    }

    pub fn xor() -> Self {
        KnotKind::Xor
    }

    pub fn falling_to_zero() -> Self {
        KnotKind::FallingToZero
    }

    pub fn change() -> Self {
        KnotKind::Change
    }

    pub fn clamp(min: Signal, max: Signal, domain: SignalDomain) -> Self {
        KnotKind::Clamp { domain, min, max }
    }

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
    use crate::foundation::signal::{from_count, ONE};

    #[test]
    fn helpers_and_arity() {
        assert!(matches!(KnotKind::or2(), KnotKind::Or { arity: 2 }));
        assert!(matches!(KnotKind::not(), KnotKind::Not));
        assert!(matches!(
            KnotKind::constant_count(7),
            KnotKind::Constant { value, .. } if value == from_count(7)
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
    }
}
