//! Fractions and complex numbers.
//!
//! A fraction is exact: a third really is a third, and adding three of them gives one, which is
//! something no decimal number can honestly say. It is a whole number over a whole number, kept
//! in its smallest form with the sign on top and the bottom always above zero.
//!
//! A complex number is two decimals, one across and one up.

use crate::big::Big;
use std::cmp::Ordering;

// ---------------------------------------------------------------------------
// Fractions
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fraction {
    top: Big,
    /// Always above zero, so the sign lives on top and there is only one way to write a value.
    bottom: Big,
}

impl Fraction {
    /// `None` when the bottom is zero, because nothing can be shared into zero parts.
    pub fn new(top: Big, bottom: Big) -> Option<Fraction> {
        if bottom.is_zero() {
            return None;
        }
        let (top, bottom) = if bottom.is_negative() {
            (top.negated(), bottom.negated())
        } else {
            (top, bottom)
        };
        if top.is_zero() {
            return Some(Fraction {
                top: Big::zero(),
                bottom: Big::from_i64(1),
            });
        }
        let g = top.gcd(&bottom);
        let (top, _) = top.div_rem(&g)?;
        let (bottom, _) = bottom.div_rem(&g)?;
        Some(Fraction { top, bottom })
    }

    pub fn from_whole(v: Big) -> Fraction {
        Fraction {
            top: v,
            bottom: Big::from_i64(1),
        }
    }

    pub fn top(&self) -> &Big {
        &self.top
    }

    pub fn bottom(&self) -> &Big {
        &self.bottom
    }

    pub fn is_whole(&self) -> bool {
        self.bottom.to_i64() == Some(1)
    }

    pub fn is_zero(&self) -> bool {
        self.top.is_zero()
    }

    pub fn to_f64(&self) -> f64 {
        // Dividing the two directly loses everything once they outgrow a decimal, so take the
        // whole part exactly first and only then fall back to dividing.
        match (self.top.to_i64(), self.bottom.to_i64()) {
            (Some(t), Some(b)) => t as f64 / b as f64,
            _ => self.top.to_f64() / self.bottom.to_f64(),
        }
    }

    pub fn add(&self, other: &Fraction) -> Fraction {
        let top = self
            .top
            .mul(&other.bottom)
            .add(&other.top.mul(&self.bottom));
        let bottom = self.bottom.mul(&other.bottom);
        Fraction::new(top, bottom).unwrap_or_else(Fraction::zero)
    }

    pub fn sub(&self, other: &Fraction) -> Fraction {
        self.add(&other.negated())
    }

    pub fn mul(&self, other: &Fraction) -> Fraction {
        Fraction::new(self.top.mul(&other.top), self.bottom.mul(&other.bottom))
            .unwrap_or_else(Fraction::zero)
    }

    /// `None` when the other one is zero.
    pub fn div(&self, other: &Fraction) -> Option<Fraction> {
        if other.is_zero() {
            return None;
        }
        Fraction::new(self.top.mul(&other.bottom), self.bottom.mul(&other.top))
    }

    pub fn negated(&self) -> Fraction {
        Fraction {
            top: self.top.negated(),
            bottom: self.bottom.clone(),
        }
    }

    pub fn zero() -> Fraction {
        Fraction {
            top: Big::zero(),
            bottom: Big::from_i64(1),
        }
    }

    pub fn compare(&self, other: &Fraction) -> Ordering {
        self.top
            .mul(&other.bottom)
            .compare(&other.top.mul(&self.bottom))
    }

    /// How a fraction is written when shown: `3/4`, and a whole one as just itself.
    pub fn showable(&self) -> String {
        if self.is_whole() {
            self.top.to_decimal()
        } else {
            format!("{}/{}", self.top.to_decimal(), self.bottom.to_decimal())
        }
    }

    /// The closest fraction to a decimal, found by continued fractions.
    ///
    /// Stops once it is within a hair of the value, so 0.75 comes back as three quarters rather
    /// than as some enormous pair that happens to match the bits.
    pub fn nearest_to(value: f64) -> Option<Fraction> {
        if !value.is_finite() {
            return None;
        }
        let negative = value < 0.0;
        let mut x = value.abs();

        let (mut prev_top, mut top) = (0i64, 1i64);
        let (mut prev_bottom, mut bottom) = (1i64, 0i64);

        for _ in 0..40 {
            let whole = x.floor();
            if whole > 1e18 {
                break;
            }
            let a = whole as i64;
            let next_top = match a.checked_mul(top).and_then(|v| v.checked_add(prev_top)) {
                Some(v) => v,
                None => break,
            };
            let next_bottom = match a.checked_mul(bottom).and_then(|v| v.checked_add(prev_bottom)) {
                Some(v) => v,
                None => break,
            };
            prev_top = top;
            top = next_top;
            prev_bottom = bottom;
            bottom = next_bottom;

            if bottom != 0 {
                let approximation = top as f64 / bottom as f64;
                if (approximation - value.abs()).abs() <= 1e-12 * value.abs().max(1.0) {
                    break;
                }
            }
            let fraction_part = x - whole;
            if fraction_part.abs() < 1e-15 {
                break;
            }
            x = 1.0 / fraction_part;
        }

        if bottom == 0 {
            return None;
        }
        let top = if negative { -top } else { top };
        Fraction::new(Big::from_i64(top), Big::from_i64(bottom))
    }
}

// ---------------------------------------------------------------------------
// Complex numbers
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Complex {
    pub across: f64,
    pub up: f64,
}

impl Complex {
    pub fn new(across: f64, up: f64) -> Complex {
        Complex { across, up }
    }

    pub fn real(v: f64) -> Complex {
        Complex { across: v, up: 0.0 }
    }

    pub fn imaginary(v: f64) -> Complex {
        Complex { across: 0.0, up: v }
    }

    pub fn is_real(&self) -> bool {
        self.up == 0.0
    }

    pub fn add(&self, other: &Complex) -> Complex {
        Complex::new(self.across + other.across, self.up + other.up)
    }

    pub fn sub(&self, other: &Complex) -> Complex {
        Complex::new(self.across - other.across, self.up - other.up)
    }

    pub fn mul(&self, other: &Complex) -> Complex {
        Complex::new(
            self.across * other.across - self.up * other.up,
            self.across * other.up + self.up * other.across,
        )
    }

    /// `None` when the other one is zero.
    pub fn div(&self, other: &Complex) -> Option<Complex> {
        let denominator = other.across * other.across + other.up * other.up;
        if denominator == 0.0 {
            return None;
        }
        Some(Complex::new(
            (self.across * other.across + self.up * other.up) / denominator,
            (self.up * other.across - self.across * other.up) / denominator,
        ))
    }

    pub fn negated(&self) -> Complex {
        Complex::new(-self.across, -self.up)
    }

    pub fn conjugate(&self) -> Complex {
        Complex::new(self.across, -self.up)
    }

    /// How far it is from zero.
    pub fn size(&self) -> f64 {
        self.across.hypot(self.up)
    }

    /// Which way it points, in radians.
    pub fn direction(&self) -> f64 {
        self.up.atan2(self.across)
    }

    /// Every number has a square root once you allow the imaginary ones.
    pub fn square_root(&self) -> Complex {
        if self.up == 0.0 && self.across >= 0.0 {
            return Complex::real(self.across.sqrt());
        }
        let size = self.size();
        let across = ((size + self.across) / 2.0).max(0.0).sqrt();
        let up = ((size - self.across) / 2.0).max(0.0).sqrt();
        Complex::new(across, if self.up < 0.0 { -up } else { up })
    }

    pub fn pow(&self, other: &Complex) -> Complex {
        if self.across == 0.0 && self.up == 0.0 {
            return if other.across == 0.0 && other.up == 0.0 {
                Complex::real(1.0)
            } else {
                Complex::real(0.0)
            };
        }
        let log_size = self.size().ln();
        let direction = self.direction();
        let new_log = other.across * log_size - other.up * direction;
        let new_direction = other.up * log_size + other.across * direction;
        let size = new_log.exp();
        Complex::new(size * new_direction.cos(), size * new_direction.sin())
    }

    pub fn showable(&self) -> String {
        let across = show_part(self.across);
        if self.up == 0.0 {
            return across;
        }
        let sign = if self.up < 0.0 { "-" } else { "+" };
        let up = show_part(self.up.abs());
        if self.across == 0.0 && self.up > 0.0 {
            return format!("{up}i");
        }
        format!("{across}{sign}{up}i")
    }
}

fn show_part(v: f64) -> String {
    if v.fract() == 0.0 && v.is_finite() && v.abs() < 1e15 {
        format!("{v:.0}")
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(top: i64, bottom: i64) -> Fraction {
        Fraction::new(Big::from_i64(top), Big::from_i64(bottom)).unwrap()
    }

    #[test]
    fn a_fraction_keeps_itself_in_its_smallest_form() {
        assert_eq!(f(6, 8).showable(), "3/4");
        assert_eq!(f(-6, 8).showable(), "-3/4");
        // The sign always ends up on top, so there is only one way to write a value.
        assert_eq!(f(6, -8).showable(), "-3/4");
        assert_eq!(f(8, 4).showable(), "2");
        assert_eq!(f(0, 5).showable(), "0");
    }

    #[test]
    fn a_third_really_is_a_third() {
        let third = f(1, 3);
        let whole = third.add(&third).add(&third);
        assert_eq!(whole.showable(), "1");
        // Which decimals cannot say.
        assert!((0.1f64 + 0.2 - 0.3).abs() > 0.0);
        let tenth = f(1, 10);
        let fifth = f(1, 5);
        assert_eq!(tenth.add(&fifth).showable(), "3/10");
    }

    #[test]
    fn fractions_add_take_away_multiply_and_divide() {
        assert_eq!(f(1, 2).add(&f(1, 3)).showable(), "5/6");
        assert_eq!(f(1, 2).sub(&f(1, 3)).showable(), "1/6");
        assert_eq!(f(2, 3).mul(&f(3, 4)).showable(), "1/2");
        assert_eq!(f(1, 2).div(&f(1, 4)).unwrap().showable(), "2");
        assert!(f(1, 2).div(&f(0, 1)).is_none());
        assert!(Fraction::new(Big::from_i64(1), Big::zero()).is_none());
    }

    #[test]
    fn fractions_compare_without_ever_becoming_decimals() {
        assert_eq!(f(1, 3).compare(&f(1, 2)), Ordering::Less);
        assert_eq!(f(2, 4).compare(&f(1, 2)), Ordering::Equal);
        assert_eq!(f(3, 4).compare(&f(2, 3)), Ordering::Greater);
    }

    #[test]
    fn a_decimal_finds_the_fraction_a_person_would_have_written() {
        assert_eq!(Fraction::nearest_to(0.75).unwrap().showable(), "3/4");
        assert_eq!(Fraction::nearest_to(0.5).unwrap().showable(), "1/2");
        assert_eq!(Fraction::nearest_to(2.0).unwrap().showable(), "2");
        assert_eq!(Fraction::nearest_to(-0.25).unwrap().showable(), "-1/4");
        // A third read back as a decimal and turned round again is still a third.
        assert_eq!(Fraction::nearest_to(1.0 / 3.0).unwrap().showable(), "1/3");
    }

    #[test]
    fn complex_numbers_add_and_multiply_the_way_they_should() {
        let a = Complex::new(3.0, 4.0);
        let b = Complex::new(1.0, -2.0);
        assert_eq!(a.add(&b).showable(), "4+2i");
        assert_eq!(a.sub(&b).showable(), "2+6i");
        // (3+4i)(1-2i) = 3 - 6i + 4i + 8 = 11 - 2i
        assert_eq!(a.mul(&b).showable(), "11-2i");
        assert_eq!(a.size(), 5.0);
        assert_eq!(a.conjugate().showable(), "3-4i");
    }

    #[test]
    fn every_number_has_a_square_root_once_the_imaginary_ones_are_allowed() {
        let minus_one = Complex::real(-1.0);
        let root = minus_one.square_root();
        assert!((root.across).abs() < 1e-12);
        assert!((root.up - 1.0).abs() < 1e-12);
        // And squaring it again comes back.
        let back = root.mul(&root);
        assert!((back.across + 1.0).abs() < 1e-12);
    }

    #[test]
    fn dividing_a_complex_number_by_zero_is_refused() {
        assert!(Complex::new(1.0, 1.0).div(&Complex::real(0.0)).is_none());
        let q = Complex::new(1.0, 1.0).div(&Complex::new(1.0, 1.0)).unwrap();
        assert!((q.across - 1.0).abs() < 1e-12 && q.up.abs() < 1e-12);
    }

    #[test]
    fn complex_numbers_are_written_the_way_they_are_read() {
        assert_eq!(Complex::real(3.0).showable(), "3");
        assert_eq!(Complex::imaginary(1.0).showable(), "1i");
        assert_eq!(Complex::new(0.0, -1.0).showable(), "0-1i");
        assert_eq!(Complex::new(1.5, 2.5).showable(), "1.5+2.5i");
    }
}
