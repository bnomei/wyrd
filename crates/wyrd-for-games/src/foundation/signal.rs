//! Monomorphic [`Signal`] wire type: feature-selected `f32` or i32 Q16.
//!
//! Exactly one of `signal-f32` / `signal-i32` must be enabled. Truthiness is
//! non-zero. Author helpers (`from_level`, `from_count`) convert into the active
//! path; path-local mul/div/sat ops live here but graphs should prefer `Calc`.

#[cfg(all(feature = "signal-f32", feature = "signal-i32"))]
compile_error!("enable exactly one of: feature \"signal-f32\", feature \"signal-i32\"");

#[cfg(not(any(feature = "signal-f32", feature = "signal-i32")))]
compile_error!("enable one of: feature \"signal-f32\", feature \"signal-i32\"");

/// Wire value type for the `signal-f32` compile-time path.
#[cfg(feature = "signal-f32")]
pub type Signal = f32;

/// Wire value type for the `signal-i32` Q16.16 compile-time path.
#[cfg(feature = "signal-i32")]
pub type Signal = i32;

/// Canonical false / off level for the active signal path.
#[cfg(feature = "signal-f32")]
pub const ZERO: Signal = 0.0;

/// Canonical true / on level for the active signal path.
#[cfg(feature = "signal-f32")]
pub const ONE: Signal = 1.0;

/// Fractional bits for the i32 Q16.16 path (`ONE == 1 << FRAC_BITS`).
#[cfg(feature = "signal-i32")]
pub const FRAC_BITS: u32 = 16;

/// Canonical false / off level for the Q16.16 signal path.
#[cfg(feature = "signal-i32")]
pub const ZERO: Signal = 0;

/// Canonical true / on level for the Q16.16 signal path.
#[cfg(feature = "signal-i32")]
pub const ONE: Signal = 1 << FRAC_BITS;

/// Truthy when not equal to [`ZERO`] (level and count domains share this rule).
#[inline]
pub fn is_truthy(s: Signal) -> bool {
    s != ZERO
}

/// Whole count into Signal bits (not Q-scaled on i32).
#[inline]
pub fn from_count(n: i32) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        // `i32::MAX` rounds up to 2^31 as f32, which is outside the Count
        // domain's inclusive i32 contract. Keep the largest representable
        // whole f32 at or below that bound instead.
        (n as f32).min(2_147_483_520.0)
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
        // no_std nearest-int without `f32::round`
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

/// Path-local div; `b == ZERO` yields [`ZERO`]. Prefer Calc knots in graphs.
///
/// Fast paths: identity on `ONE`, and Q-domain negate on `-ONE` (i32) so
/// `i32::MIN` still saturates rather than panicking on a raw shift/div path.
#[inline]
pub fn div(a: Signal, b: Signal) -> Signal {
    if b == ZERO {
        return ZERO;
    }
    if b == ONE {
        return a;
    }
    #[cfg(feature = "signal-f32")]
    {
        a / b
    }
    #[cfg(feature = "signal-i32")]
    {
        if b == -ONE {
            return a.saturating_neg();
        }
        let n = (a as i64) << FRAC_BITS;
        (n / b as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }
}

/// Saturating add on i32; plain add on f32.
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

/// Saturating sub on i32; plain sub on f32.
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
            // Whole-count bits are not Q-levels: 2*3 shifts to 0 under Q-mul.
            assert_eq!(p, 0);
            assert_eq!(mul(ONE, ONE), ONE);
        }

        #[cfg(feature = "signal-f32")]
        {
            assert_eq!(div(from_count(6), from_count(2)), from_count(3));
            assert_eq!(div(from_count(6), ZERO), ZERO);
            assert_eq!(
                div(from_count(7), ONE),
                from_count(7),
                "div by ONE identity"
            );
            assert_eq!(sat_add(from_count(1), from_count(2)), from_count(3));
            assert_eq!(sat_sub(from_count(5), from_count(2)), from_count(3));
        }
        #[cfg(feature = "signal-i32")]
        {
            assert_eq!(div(ONE * 2, ONE), ONE * 2);
            assert_eq!(div(ONE * 3, ONE), ONE * 3, "div by ONE identity");
            assert_eq!(div(ONE, -ONE), -ONE, "div by -ONE");
            assert_eq!(div(ONE, ZERO), ZERO);
            assert_eq!(sat_add(from_count(1), from_count(2)), from_count(3));
            assert_eq!(sat_sub(from_count(5), from_count(2)), from_count(3));
            assert_eq!(sat_add(i32::MAX, 1), i32::MAX);
            assert_eq!(sat_sub(i32::MIN, 1), i32::MIN);
            let big = mul(i32::MAX, i32::MAX);
            assert_eq!(big, i32::MAX);
            let d = div(i32::MAX, 1);
            let _ = d;
            let neg = from_level(-0.25);
            assert!(neg < 0);
        }
    }
}
