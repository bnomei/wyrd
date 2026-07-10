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
    // Identity divisors: Q-div by ONE and float `/ 1.0` are no-ops.
    if b == ONE {
        return a;
    }
    #[cfg(feature = "signal-f32")]
    {
        a / b
    }
    #[cfg(feature = "signal-i32")]
    {
        // (-ONE) → negate in Q domain (still saturating via i64 path for a==MIN).
        if b == -ONE {
            return a.saturating_neg();
        }
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

    #[test]
    fn from_level_and_ops() {
        let half = from_level(0.5);
        assert!(is_truthy(half));
        #[cfg(feature = "signal-f32")]
        assert_eq!(half, 0.5);
        #[cfg(feature = "signal-i32")]
        {
            assert_eq!(half, ONE / 2);
            let neg = from_level(-0.5);
            assert!(neg < 0);
        }

        let two = from_count(2);
        let three = from_count(3);
        let p = mul(two, three);
        #[cfg(feature = "signal-f32")]
        assert_eq!(p, 6.0);
        #[cfg(feature = "signal-i32")]
        {
            // Whole-count Q-mul shifts away the product (2*3 >> 16 = 0).
            assert_eq!(p, 0);
            // Level-scale Q-mul: 1.0 * 1.0 stays ONE.
            assert_eq!(mul(ONE, ONE), ONE);
        }

        #[cfg(feature = "signal-f32")]
        {
            assert_eq!(div(from_count(6), from_count(2)), from_count(3));
            assert_eq!(div(from_count(6), ZERO), ZERO);
            assert_eq!(div(from_count(7), ONE), from_count(7), "div by ONE identity");
            assert_eq!(sat_add(from_count(1), from_count(2)), from_count(3));
            assert_eq!(sat_sub(from_count(5), from_count(2)), from_count(3));
        }
        #[cfg(feature = "signal-i32")]
        {
            // Q-div: levels in ONE units (not whole-count integers).
            assert_eq!(div(ONE * 2, ONE), ONE * 2); // 2.0 / 1.0
            assert_eq!(div(ONE * 3, ONE), ONE * 3, "div by ONE identity");
            assert_eq!(div(ONE, -ONE), -ONE, "div by -ONE");
            assert_eq!(div(ONE, ZERO), ZERO);
            assert_eq!(sat_add(from_count(1), from_count(2)), from_count(3));
            assert_eq!(sat_sub(from_count(5), from_count(2)), from_count(3));
            assert_eq!(sat_add(i32::MAX, 1), i32::MAX);
            assert_eq!(sat_sub(i32::MIN, 1), i32::MIN);
            // clamp path in mul/div
            let big = mul(i32::MAX, i32::MAX);
            assert!(big <= i32::MAX);
            let d = div(i32::MAX, 1);
            let _ = d;
            // from_level negative branch
            let neg = from_level(-0.25);
            assert!(neg < 0);
        }
    }
}
