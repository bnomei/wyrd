//! Monomorphic Signal: feature-selected f32 or i32.

#[cfg(all(feature = "signal-f32", feature = "signal-i32"))]
compile_error!("enable exactly one of: feature \"signal-f32\", feature \"signal-i32\"");

#[cfg(not(any(feature = "signal-f32", feature = "signal-i32")))]
compile_error!("enable one of: feature \"signal-f32\", feature \"signal-i32\"");

#[cfg(feature = "signal-f32")]
pub type Signal = f32;

#[cfg(feature = "signal-i32")]
pub type Signal = i32;

#[cfg(feature = "signal-f32")]
pub const ZERO: Signal = 0.0;

#[cfg(feature = "signal-f32")]
pub const ONE: Signal = 1.0;

#[cfg(feature = "signal-i32")]
pub const FRAC_BITS: u32 = 16;

#[cfg(feature = "signal-i32")]
pub const ZERO: Signal = 0;

#[cfg(feature = "signal-i32")]
pub const ONE: Signal = 1 << FRAC_BITS;

#[inline]
pub fn is_truthy(s: Signal) -> bool {
    s != ZERO
}

/// Whole count into Signal bits (not Q-scaled on i32).
#[inline]
pub fn from_count(n: i32) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        n as f32
    }
    #[cfg(feature = "signal-i32")]
    {
        n
    }
}

/// Author level ~0..=1 into Signal.
#[inline]
pub fn from_level(x: f32) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        x
    }
    #[cfg(feature = "signal-i32")]
    {
        // no_std: avoid f32::round; nearest-int via truncate bias
        let y = x * (ONE as f32);
        if y >= 0.0 {
            (y + 0.5) as i32
        } else {
            (y - 0.5) as i32
        }
    }
}

/// Path-local mul (f32 `*`, i32 Q-mul). Prefer Calc knots in graphs.
#[inline]
pub fn mul(a: Signal, b: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        a * b
    }
    #[cfg(feature = "signal-i32")]
    {
        let p = (a as i64) * (b as i64);
        let s = p >> FRAC_BITS;
        s.clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }
}

#[inline]
pub fn div(a: Signal, b: Signal) -> Signal {
    if b == ZERO {
        return ZERO;
    }
    #[cfg(feature = "signal-f32")]
    {
        a / b
    }
    #[cfg(feature = "signal-i32")]
    {
        let n = (a as i64) << FRAC_BITS;
        (n / b as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }
}

#[inline]
pub fn sat_add(a: Signal, b: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        a + b
    }
    #[cfg(feature = "signal-i32")]
    {
        a.saturating_add(b)
    }
}

#[inline]
pub fn sat_sub(a: Signal, b: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        a - b
    }
    #[cfg(feature = "signal-i32")]
    {
        a.saturating_sub(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truthy_one() {
        assert!(is_truthy(ONE));
        assert!(!is_truthy(ZERO));
    }

    #[test]
    fn count_three() {
        let c = from_count(3);
        assert!(is_truthy(c));
        #[cfg(feature = "signal-f32")]
        assert_eq!(c, 3.0);
        #[cfg(feature = "signal-i32")]
        assert_eq!(c, 3);
    }
}
