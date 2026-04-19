#![cfg_attr(not(feature = "std"), no_std)]

//! Fixed-point angles modulo 2π, backed by unsigned integers.
//!
//! [`Angle<T>`] represents an angle as an unsigned integer where the full
//! range of `T` maps onto one full turn (2π radians / 360 degrees). Natural
//! integer overflow provides wraparound at 2π, so modular arithmetic is free.
//!
//! Type aliases [`Angle8`], [`Angle16`], [`Angle32`], [`Angle64`], and
//! [`Angle128`] cover the standard widths.
//!
//! # Precision
//!
//! Float conversions are generic over any `F: Float`. Expect precision loss
//! when the integer width exceeds the float mantissa (e.g. [`Angle128`] with
//! `f64` keeps ~53 of 128 bits). Non-finite inputs (`NaN`, infinities) are
//! coerced to zero rather than panicking.
//!
//! # Example
//!
//! ```
//! use turns::Angle8;
//! use core::f64::consts::PI;
//!
//! let pi = Angle8::from_radians(PI);
//! assert_eq!(pi + pi, Angle8::from_radians(0.0_f64));
//! ```

use core::ops::{Add, Div, Mul, Neg, Shl, Shr, Sub};
use num_traits::{
    Bounded, CheckedDiv, CheckedMul, Euclid, Float, FloatConst, NumCast, ToPrimitive, WrappingAdd,
    WrappingMul, WrappingNeg, WrappingSub, Zero,
};

/// Fixed-point angle modulo 2π, stored as an unsigned integer `T`.
///
/// The full range of `T` covers one turn: `T::MAX + 1` corresponds to 2π.
/// Intended to be instantiated with an unsigned primitive integer; the
/// type aliases [`Angle8`] through [`Angle128`] are the supported widths.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Angle<T>(pub T);

impl<T: NumCast + ToPrimitive + Bounded + Zero> Angle<T> {
    /// Construct an angle from radians.
    ///
    /// Input is reduced modulo 2π. Non-finite values (`NaN`, `±∞`) map to
    /// the zero angle.
    #[must_use]
    pub fn from_radians<F: Float + FloatConst + Euclid>(radians: F) -> Self {
        if !radians.is_finite() {
            return Self(T::zero());
        }
        let scale = <F as NumCast>::from(T::max_value()).unwrap() + F::one();
        let normalized = radians.rem_euclid(&F::TAU()) / F::TAU();
        Self(NumCast::from(normalized * scale).unwrap_or_else(T::zero))
    }

    /// Convert the angle to radians in `[0, 2π)`.
    #[must_use]
    pub fn to_radians<F: Float + FloatConst>(self) -> F {
        let scale = <F as NumCast>::from(T::max_value()).unwrap() + F::one();
        <F as NumCast>::from(self.0).unwrap() / scale * F::TAU()
    }

    /// Construct an angle from degrees.
    ///
    /// Input is reduced modulo 360. Non-finite values map to zero.
    #[must_use]
    pub fn from_degrees<F: Float + Euclid>(degrees: F) -> Self {
        if !degrees.is_finite() {
            return Self(T::zero());
        }
        let scale = <F as NumCast>::from(T::max_value()).unwrap() + F::one();
        let full = <F as NumCast>::from(360).unwrap();
        let normalized = degrees.rem_euclid(&full) / full;
        Self(NumCast::from(normalized * scale).unwrap_or_else(T::zero))
    }

    /// Convert the angle to degrees in `[0, 360)`.
    #[must_use]
    pub fn to_degrees<F: Float>(self) -> F {
        let scale = <F as NumCast>::from(T::max_value()).unwrap() + F::one();
        let full = <F as NumCast>::from(360).unwrap();
        <F as NumCast>::from(self.0).unwrap() / scale * full
    }

    /// Construct an angle from the result of `atan2(y, x)`.
    ///
    /// Non-finite coordinates map to the zero angle (via [`Self::from_radians`]).
    #[must_use]
    pub fn from_atan2<F: Float + FloatConst + Euclid>(y: F, x: F) -> Self {
        Self::from_radians(y.atan2(x))
    }

    /// Sine of the angle.
    #[must_use]
    pub fn sin<F: Float + FloatConst>(self) -> F {
        self.to_radians::<F>().sin()
    }

    /// Cosine of the angle.
    #[must_use]
    pub fn cos<F: Float + FloatConst>(self) -> F {
        self.to_radians::<F>().cos()
    }

    /// Tangent of the angle.
    #[must_use]
    pub fn tan<F: Float + FloatConst>(self) -> F {
        self.to_radians::<F>().tan()
    }

    /// Sine and cosine computed together as `(sin, cos)`.
    #[must_use]
    pub fn sin_cos<F: Float + FloatConst>(self) -> (F, F) {
        self.to_radians::<F>().sin_cos()
    }

    /// Multiply the angle by a floating-point factor, wrapping mod 2π.
    ///
    /// A non-finite `factor` returns the zero angle.
    #[must_use]
    pub fn scale<F: Float + Euclid>(self, factor: F) -> Self {
        if !factor.is_finite() {
            return Self(T::zero());
        }
        let full = <F as NumCast>::from(T::max_value()).unwrap() + F::one();
        let wrapped = (<F as NumCast>::from(self.0).unwrap() * factor).rem_euclid(&full);
        Self(NumCast::from(wrapped).unwrap_or_else(T::zero))
    }
}

impl<T: ToPrimitive> Angle<T> {
    /// Ratio of two angles as a float: `self / other`.
    ///
    /// Division is on the underlying integer values; dividing by the zero
    /// angle produces `±∞` or `NaN` per IEEE-754.
    #[must_use]
    pub fn ratio<F: Float>(self, other: Self) -> F {
        <F as NumCast>::from(self.0).unwrap() / <F as NumCast>::from(other.0).unwrap()
    }
}

impl<T: ToPrimitive + Shr<usize, Output = T>> Angle<T> {
    /// Re-encode the angle in a different integer width.
    ///
    /// Widening copies the source value into the top bits of the target;
    /// narrowing keeps the top bits of the source. The angle itself is
    /// preserved up to the target's resolution.
    #[must_use]
    pub fn cast<U: NumCast + Shl<usize, Output = U>>(self) -> Angle<U> {
        let src_bits = core::mem::size_of::<T>() * 8;
        let dst_bits = core::mem::size_of::<U>() * 8;
        if src_bits >= dst_bits {
            let shifted = self.0 >> (src_bits - dst_bits);
            Angle(<U as NumCast>::from(shifted).unwrap())
        } else {
            let widened: U = <U as NumCast>::from(self.0).unwrap();
            Angle(widened << (dst_bits - src_bits))
        }
    }
}

impl<T: WrappingAdd> Add for Angle<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Angle(self.0.wrapping_add(&rhs.0))
    }
}

impl<T: WrappingSub> Sub for Angle<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Angle(self.0.wrapping_sub(&rhs.0))
    }
}

impl<T: WrappingNeg> Neg for Angle<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Angle(self.0.wrapping_neg())
    }
}

impl<T: WrappingMul> Mul<T> for Angle<T> {
    type Output = Self;
    fn mul(self, rhs: T) -> Self {
        Angle(self.0.wrapping_mul(&rhs))
    }
}

impl<T: Div> Div<T> for Angle<T> {
    type Output = Angle<T::Output>;
    fn div(self, rhs: T) -> Self::Output {
        Angle(self.0 / rhs)
    }
}

impl<T: Shl<usize, Output = T> + Zero> Shl<usize> for Angle<T> {
    type Output = Self;
    fn shl(self, rhs: usize) -> Self {
        if rhs >= core::mem::size_of::<T>() * 8 {
            Angle(T::zero())
        } else {
            Angle(self.0 << rhs)
        }
    }
}

impl<T: Shr<usize, Output = T> + Zero> Shr<usize> for Angle<T> {
    type Output = Self;
    fn shr(self, rhs: usize) -> Self {
        if rhs >= core::mem::size_of::<T>() * 8 {
            Angle(T::zero())
        } else {
            Angle(self.0 >> rhs)
        }
    }
}

impl<T: CheckedMul> Angle<T> {
    /// Multiply by an integer scalar, returning `None` on raw integer
    /// overflow.
    ///
    /// Unlike the wrapping [`Mul`] impl, this surfaces overflow instead of
    /// wrapping — useful when you need to know whether the operation
    /// crossed 2π.
    #[must_use]
    pub fn checked_mul(self, rhs: T) -> Option<Self> {
        self.0.checked_mul(&rhs).map(Angle)
    }
}

impl<T: CheckedDiv> Angle<T> {
    /// Divide by an integer scalar, returning `None` on division by zero.
    #[must_use]
    pub fn checked_div(self, rhs: T) -> Option<Self> {
        self.0.checked_div(&rhs).map(Angle)
    }
}

macro_rules! impl_consts {
    ($t:ty) => {
        impl Angle<$t> {
            /// The zero angle.
            pub const ZERO: Self = Angle(0);
            /// One full turn (2π). Wraps to zero in this representation.
            pub const TAU: Self = Angle(0);
            /// π (half turn).
            pub const PI: Self = Angle(<$t>::MAX / 2 + 1);
            /// π/2 (quarter turn).
            pub const FRAC_PI_2: Self = Angle(<$t>::MAX / 4 + 1);
            /// π/3 (truncated to the nearest representable value).
            pub const FRAC_PI_3: Self = Angle(<$t>::MAX / 6);
            /// π/4 (eighth turn).
            pub const FRAC_PI_4: Self = Angle(<$t>::MAX / 8 + 1);
            /// π/6 (truncated to the nearest representable value).
            pub const FRAC_PI_6: Self = Angle(<$t>::MAX / 12);
            /// π/8.
            pub const FRAC_PI_8: Self = Angle(<$t>::MAX / 16 + 1);
        }
    };
}

impl_consts!(u8);
impl_consts!(u16);
impl_consts!(u32);
impl_consts!(u64);
impl_consts!(u128);

/// 8-bit angle: one full turn per 256 steps.
pub type Angle8 = Angle<u8>;
/// 16-bit angle: one full turn per 65,536 steps.
pub type Angle16 = Angle<u16>;
/// 32-bit angle: one full turn per 2³² steps.
pub type Angle32 = Angle<u32>;
/// 64-bit angle: one full turn per 2⁶⁴ steps.
pub type Angle64 = Angle<u64>;
/// 128-bit angle: one full turn per 2¹²⁸ steps.
pub type Angle128 = Angle<u128>;

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::{PI, TAU};

    #[test]
    fn pi_is_half_circle_u8() {
        assert_eq!(Angle8::from_radians(PI), Angle(0b1000_0000));
    }

    #[test]
    fn zero_and_tau_wrap_to_same() {
        assert_eq!(Angle16::from_radians(0.0_f64), Angle16::from_radians(TAU),);
    }

    #[test]
    fn roundtrip_u32_f64() {
        let a = Angle32::from_radians(1.2345_f64);
        let r: f64 = a.to_radians();
        assert!((r - 1.2345).abs() < 1e-8);
    }

    #[test]
    fn roundtrip_u16_f32() {
        let a = Angle16::from_radians(1.0_f32);
        let r: f32 = a.to_radians();
        assert!((r - 1.0).abs() < 1e-3);
    }

    #[test]
    fn widen_angle8_to_angle16_preserves_pi() {
        assert_eq!(Angle::<u8>(0x80).cast::<u16>(), Angle(0x8000_u16));
    }

    #[test]
    fn narrow_angle16_to_angle8_keeps_top_bits() {
        assert_eq!(Angle::<u16>(0x80FF).cast::<u8>(), Angle(0x80_u8));
    }

    #[test]
    fn identity_same_width() {
        assert_eq!(
            Angle::<u32>(0xDEAD_BEEF).cast::<u32>(),
            Angle(0xDEAD_BEEF_u32),
        );
    }

    #[test]
    fn degrees_180_is_half_circle_u8() {
        assert_eq!(Angle8::from_degrees(180.0_f64), Angle(0x80));
    }

    #[test]
    fn degrees_wrap_at_360() {
        assert_eq!(
            Angle16::from_degrees(0.0_f64),
            Angle16::from_degrees(360.0_f64),
        );
    }

    #[test]
    fn roundtrip_degrees_u32() {
        let a = Angle32::from_degrees(123.456_f64);
        let d: f64 = a.to_degrees();
        assert!((d - 123.456).abs() < 1e-6);
    }

    #[test]
    fn add_wraps_past_tau() {
        let pi: Angle8 = Angle(0x80);
        assert_eq!(pi + pi, Angle(0));
    }

    #[test]
    fn sub_wraps_below_zero() {
        let zero: Angle8 = Angle(0);
        let pi: Angle8 = Angle(0x80);
        assert_eq!(zero - pi, pi);
    }

    #[test]
    fn mul_three_pi_wraps_to_pi() {
        let pi: Angle8 = Angle(0x80);
        assert_eq!(pi * 3_u8, Angle(0x80));
    }

    #[test]
    fn div_halves_pi() {
        let pi: Angle8 = Angle(0x80);
        assert_eq!(pi / 2_u8, Angle(0x40));
    }

    #[test]
    fn checked_mul_detects_overflow() {
        let a: Angle8 = Angle(200);
        assert!(a.checked_mul(2).is_none());
        assert_eq!(a.checked_mul(1), Some(a));
    }

    #[test]
    fn checked_div_by_zero_is_none() {
        let a: Angle8 = Angle(0x80);
        assert!(a.checked_div(0).is_none());
        assert_eq!(a.checked_div(2), Some(Angle(0x40)));
    }

    #[test]
    fn scale_half_pi_gives_quarter_pi() {
        let pi: Angle8 = Angle(0x80);
        assert_eq!(pi.scale(0.5_f64), Angle(0x40));
    }

    #[test]
    fn scale_two_wraps_to_zero() {
        let pi: Angle8 = Angle(0x80);
        assert_eq!(pi.scale(2.0_f64), Angle(0));
    }

    #[test]
    fn constants_match_from_radians() {
        assert_eq!(Angle8::PI, Angle8::from_radians(PI));
        assert_eq!(Angle8::FRAC_PI_2, Angle8::from_radians(PI / 2.0));
        assert_eq!(Angle8::FRAC_PI_4, Angle8::from_radians(PI / 4.0));
        assert_eq!(Angle8::FRAC_PI_8, Angle8::from_radians(PI / 8.0));
        assert_eq!(Angle8::FRAC_PI_3, Angle8::from_radians(PI / 3.0));
        assert_eq!(Angle8::FRAC_PI_6, Angle8::from_radians(PI / 6.0));
        assert_eq!(Angle8::TAU, Angle8::ZERO);
        assert_eq!(Angle8::ZERO, Angle(0));
        assert_eq!(Angle8::PI, Angle(0x80));
        assert_eq!(Angle8::FRAC_PI_2, Angle(0x40));
    }

    #[test]
    fn neg_pi_is_pi() {
        let pi: Angle8 = Angle(0x80);
        assert_eq!(-pi, pi);
    }

    #[test]
    fn neg_frac_pi_2_is_three_quarter_turn() {
        let quarter: Angle8 = Angle(0x40);
        assert_eq!(-quarter, Angle(0xC0));
    }

    #[test]
    fn from_atan2_north_is_frac_pi_2() {
        let a = Angle64::from_atan2(1.0_f64, 0.0_f64);
        assert_eq!(a, Angle64::FRAC_PI_2);
    }

    #[test]
    fn tan_frac_pi_4_is_one() {
        let t: f64 = Angle64::FRAC_PI_4.tan();
        assert!((t - 1.0).abs() < 1e-9);
    }

    #[test]
    fn sin_cos_matches_individual() {
        let a: Angle32 = Angle::<u32>(0x1234_5678);
        let (s, c): (f64, f64) = a.sin_cos();
        assert!((s - a.sin::<f64>()).abs() < 1e-12);
        assert!((c - a.cos::<f64>()).abs() < 1e-12);
    }

    #[test]
    fn sin_pi_is_near_zero() {
        let pi: Angle8 = Angle(0x80);
        let s: f64 = pi.sin();
        assert!(s.abs() < 1e-9);
    }

    #[test]
    fn cos_zero_is_one() {
        let zero: Angle16 = Angle(0);
        let c: f64 = zero.cos();
        assert!((c - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cos_pi_is_minus_one() {
        let pi: Angle8 = Angle(0x80);
        let c: f64 = pi.cos();
        assert!((c + 1.0).abs() < 1e-9);
    }

    #[test]
    fn from_radians_nan_is_zero() {
        assert_eq!(Angle8::from_radians(f64::NAN), Angle(0));
        assert_eq!(Angle8::from_radians(f64::INFINITY), Angle(0));
        assert_eq!(Angle8::from_radians(f64::NEG_INFINITY), Angle(0));
    }

    #[test]
    fn from_degrees_non_finite_is_zero() {
        assert_eq!(Angle16::from_degrees(f32::NAN), Angle(0));
        assert_eq!(Angle16::from_degrees(f32::INFINITY), Angle(0));
    }

    #[test]
    fn scale_non_finite_is_zero() {
        let pi: Angle8 = Angle(0x80);
        assert_eq!(pi.scale(f64::NAN), Angle(0));
        assert_eq!(pi.scale(f64::INFINITY), Angle(0));
    }

    #[test]
    fn scale_negative_wraps() {
        let half_pi: Angle8 = Angle(0x40);
        assert_eq!(half_pi.scale(-1.0_f64), Angle(0xC0));
    }

    #[test]
    fn ratio_pi_over_half_pi_is_two() {
        let pi: Angle8 = Angle(0x80);
        let half_pi: Angle8 = Angle(0x40);
        let r: f64 = pi.ratio(half_pi);
        assert!((r - 2.0).abs() < 1e-9);
    }

    #[test]
    fn shl_doubles_angle() {
        let eighth: Angle8 = Angle(0x20);
        assert_eq!(eighth << 1, Angle(0x40));
    }

    #[test]
    fn shr_halves_pi() {
        let pi: Angle8 = Angle(0x80);
        assert_eq!(pi >> 1, Angle(0x40));
    }

    #[test]
    fn shl_past_width_saturates_to_zero() {
        let a: Angle8 = Angle(0xFF);
        assert_eq!(a << 8, Angle(0));
        assert_eq!(a << 100, Angle(0));
    }

    #[test]
    fn shr_past_width_saturates_to_zero() {
        let a: Angle8 = Angle(0xFF);
        assert_eq!(a >> 8, Angle(0));
        assert_eq!(a >> 100, Angle(0));
    }

    #[test]
    fn widen_then_narrow_is_lossless() {
        let a: Angle8 = Angle(0b1010_1010);
        assert_eq!(a.cast::<u128>().cast::<u8>(), a);
    }
}
