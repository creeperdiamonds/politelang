//! The tower of numbers.
//!
//! Whole numbers sit inside fractions, fractions sit inside decimals, and decimals sit inside
//! complex numbers. When two kinds meet, both are lifted to whichever sits further out, and the
//! answer comes back in that kind. That is the same rule mathematics has always used, and it means
//! nobody has to think about it.
//!
//! Whole numbers have no limit. A small one lives in an `i64` because that is fast; the moment it
//! grows past that it moves to [`Big`] and keeps going. Nothing in the language shows the join.

use crate::big::Big;
use crate::exact::{Complex, Fraction};
use crate::{Answer, Value};
use std::cmp::Ordering;
use std::rc::Rc;

/// How far out in the tower a value sits.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Rank {
    Whole,
    Fraction,
    Decimal,
    Complex,
}

pub fn rank(v: &Value) -> Rank {
    match v {
        Value::Whole(_) | Value::Big(_) | Value::YesNo(_) | Value::Nothing => Rank::Whole,
        Value::Fraction(_) => Rank::Fraction,
        Value::Decimal(_) => Rank::Decimal,
        Value::Complex(_) => Rank::Complex,
        _ => Rank::Decimal,
    }
}

/// A whole number, however large.
pub fn as_big(v: &Value) -> Big {
    match v {
        Value::Whole(n) => Big::from_i64(*n),
        Value::Big(b) => (**b).clone(),
        Value::Fraction(f) => f
            .top()
            .div_rem(f.bottom())
            .map(|(q, _)| q)
            .unwrap_or_else(Big::zero),
        Value::Decimal(d) => Big::from_decimal(&format!("{:.0}", d.trunc())).unwrap_or_else(Big::zero),
        other => Big::from_i64(other.as_whole()),
    }
}

pub fn as_fraction(v: &Value) -> Fraction {
    match v {
        Value::Fraction(f) => (**f).clone(),
        Value::Decimal(d) => Fraction::nearest_to(*d).unwrap_or_else(Fraction::zero),
        other => Fraction::from_whole(as_big(other)),
    }
}

pub fn as_complex(v: &Value) -> Complex {
    match v {
        Value::Complex(c) => *c,
        other => Complex::real(other.as_decimal()),
    }
}

/// Give back a whole number in the smallest form that holds it, so the fast path stays fast.
pub fn from_big(b: Big) -> Value {
    match b.to_i64() {
        Some(v) => Value::Whole(v),
        None => Value::Big(Rc::new(b)),
    }
}

/// A fraction that turned out to be whole comes back as a whole number.
pub fn from_fraction(f: Fraction) -> Value {
    if f.is_whole() {
        from_big(f.top().clone())
    } else {
        Value::Fraction(Rc::new(f))
    }
}

/// A complex number with nothing above the line comes back as a decimal.
pub fn from_complex(c: Complex) -> Value {
    if c.is_real() {
        Value::Decimal(c.across)
    } else {
        Value::Complex(c)
    }
}

pub fn add(a: &Value, b: &Value) -> Value {
    // The overwhelmingly common case, kept first and kept cheap.
    if let (Value::Whole(x), Value::Whole(y)) = (a, b) {
        if let Some(v) = x.checked_add(*y) {
            return Value::Whole(v);
        }
    }
    match rank(a).max(rank(b)) {
        Rank::Whole => from_big(as_big(a).add(&as_big(b))),
        Rank::Fraction => from_fraction(as_fraction(a).add(&as_fraction(b))),
        Rank::Decimal => Value::Decimal(a.as_decimal() + b.as_decimal()),
        Rank::Complex => from_complex(as_complex(a).add(&as_complex(b))),
    }
}

pub fn sub(a: &Value, b: &Value) -> Value {
    if let (Value::Whole(x), Value::Whole(y)) = (a, b) {
        if let Some(v) = x.checked_sub(*y) {
            return Value::Whole(v);
        }
    }
    match rank(a).max(rank(b)) {
        Rank::Whole => from_big(as_big(a).sub(&as_big(b))),
        Rank::Fraction => from_fraction(as_fraction(a).sub(&as_fraction(b))),
        Rank::Decimal => Value::Decimal(a.as_decimal() - b.as_decimal()),
        Rank::Complex => from_complex(as_complex(a).sub(&as_complex(b))),
    }
}

pub fn mul(a: &Value, b: &Value) -> Value {
    if let (Value::Whole(x), Value::Whole(y)) = (a, b) {
        if let Some(v) = x.checked_mul(*y) {
            return Value::Whole(v);
        }
    }
    match rank(a).max(rank(b)) {
        Rank::Whole => from_big(as_big(a).mul(&as_big(b))),
        Rank::Fraction => from_fraction(as_fraction(a).mul(&as_fraction(b))),
        Rank::Decimal => Value::Decimal(a.as_decimal() * b.as_decimal()),
        Rank::Complex => from_complex(as_complex(a).mul(&as_complex(b))),
    }
}

pub fn negate(v: &Value) -> Value {
    match v {
        Value::Whole(n) => match n.checked_neg() {
            Some(x) => Value::Whole(x),
            None => from_big(Big::from_i64(*n).negated()),
        },
        Value::Big(b) => from_big(b.negated()),
        Value::Fraction(f) => from_fraction(f.negated()),
        Value::Decimal(d) => Value::Decimal(-d),
        Value::Complex(c) => from_complex(c.negated()),
        other => Value::Whole(-other.as_whole()),
    }
}

/// Dividing, which gives a decimal so that the answer is the one people expect.
///
/// For an exact answer, `3 over 4` makes a fraction and keeps it exact for ever.
pub fn divide(a: &Value, b: &Value) -> Answer<Value> {
    if rank(a) == Rank::Complex || rank(b) == Rank::Complex {
        return match as_complex(a).div(&as_complex(b)) {
            Some(c) => Ok(from_complex(c)),
            None => Err("nothing can be shared into zero parts".to_string()),
        };
    }
    if rank(a) == Rank::Fraction || rank(b) == Rank::Fraction {
        return match as_fraction(a).div(&as_fraction(b)) {
            Some(f) => Ok(from_fraction(f)),
            None => Err("nothing can be shared into zero parts".to_string()),
        };
    }
    let divisor = b.as_decimal();
    if divisor == 0.0 {
        return Err("nothing can be shared into zero parts".to_string());
    }
    Ok(Value::Decimal(a.as_decimal() / divisor))
}

/// Ordering, where there is one.
///
/// Complex numbers have no order — there is no sense in which one point on a plane comes before
/// another — so this gives back nothing for them, and the checker says so before the program runs.
pub fn compare(a: &Value, b: &Value) -> Option<Ordering> {
    if let (Value::Whole(x), Value::Whole(y)) = (a, b) {
        return Some(x.cmp(y));
    }
    match rank(a).max(rank(b)) {
        Rank::Whole => Some(as_big(a).compare(&as_big(b))),
        Rank::Fraction => Some(as_fraction(a).compare(&as_fraction(b))),
        Rank::Decimal => a.as_decimal().partial_cmp(&b.as_decimal()),
        Rank::Complex => None,
    }
}

pub fn same(a: &Value, b: &Value) -> bool {
    if rank(a).max(rank(b)) == Rank::Complex {
        let (x, y) = (as_complex(a), as_complex(b));
        return x.across == y.across && x.up == y.up;
    }
    compare(a, b) == Some(Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn whole(v: i64) -> Value {
        Value::Whole(v)
    }

    #[test]
    fn whole_numbers_have_no_limit() {
        // Counting up past where a small whole number stops does not stop.
        let mut v = whole(i64::MAX);
        v = add(&v, &whole(1));
        assert_eq!(v.showable(), "9223372036854775808");
        // And coming back down lands exactly where it started.
        let back = sub(&v, &whole(1));
        assert_eq!(back.showable(), i64::MAX.to_string());
        assert!(matches!(back, Value::Whole(_)), "should shrink back down");
    }

    #[test]
    fn a_hundred_factorial_is_exact() {
        let mut f = whole(1);
        for n in 1..=100i64 {
            f = mul(&f, &whole(n));
        }
        assert!(f.showable().ends_with("000000000000000000000000"));
        assert_eq!(f.showable().len(), 158);
    }

    #[test]
    fn the_tower_lifts_both_sides_to_whichever_sits_further_out() {
        let third = from_fraction(Fraction::nearest_to(1.0 / 3.0).unwrap());
        // whole meets fraction, and the answer is a fraction
        assert_eq!(add(&whole(1), &third).showable(), "4/3");
        // fraction meets decimal, and the answer is a decimal
        assert!(matches!(add(&third, &Value::Decimal(0.5)), Value::Decimal(_)));
        // anything meets complex, and the answer is complex
        let i = Value::Complex(Complex::imaginary(1.0));
        assert_eq!(add(&whole(3), &i).showable(), "3+1i");
    }

    #[test]
    fn three_thirds_make_exactly_one() {
        let third = from_fraction(Fraction::nearest_to(1.0 / 3.0).unwrap());
        let sum = add(&add(&third, &third), &third);
        assert_eq!(sum.showable(), "1");
        assert!(matches!(sum, Value::Whole(1)));
    }

    #[test]
    fn complex_numbers_have_no_order_but_do_have_sameness() {
        let a = Value::Complex(Complex::new(1.0, 2.0));
        let b = Value::Complex(Complex::new(1.0, 2.0));
        let c = Value::Complex(Complex::new(2.0, 1.0));
        assert_eq!(compare(&a, &c), None);
        assert!(same(&a, &b));
        assert!(!same(&a, &c));
    }

    #[test]
    fn dividing_by_zero_is_refused_at_every_level() {
        assert!(divide(&whole(1), &whole(0)).is_err());
        let third = from_fraction(Fraction::nearest_to(1.0 / 3.0).unwrap());
        assert!(divide(&third, &whole(0)).is_err());
        assert!(divide(&Value::Complex(Complex::new(1.0, 1.0)), &whole(0)).is_err());
    }

    #[test]
    fn big_and_small_whole_numbers_compare_as_one_kind() {
        let big = add(&whole(i64::MAX), &whole(1));
        assert_eq!(compare(&big, &whole(1)), Some(Ordering::Greater));
        assert_eq!(compare(&whole(1), &big), Some(Ordering::Less));
        assert_eq!(compare(&whole(5), &whole(5)), Some(Ordering::Equal));
    }
}
