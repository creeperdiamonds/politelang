//! Whole numbers of any size.
//!
//! A whole number in PoliteLang has no limit. Small ones live in an `i64` because that is fast and
//! covers almost everything; the moment one grows past that it moves here and keeps growing. The
//! person writing the program never sees the join.
//!
//! Written by hand, because the language has no dependencies (spec 9.5). The magnitude is a list
//! of 32-bit pieces, smallest first, with no leading zeroes; an empty list is zero.

use std::cmp::Ordering;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Big {
    /// Never true when the magnitude is empty: there is only one zero.
    negative: bool,
    mag: Vec<u32>,
}

impl Big {
    pub fn zero() -> Big {
        Big::default()
    }

    pub fn from_i64(v: i64) -> Big {
        if v == 0 {
            return Big::zero();
        }
        let negative = v < 0;
        // i64::MIN has no positive counterpart, so go through u64.
        let m = v.unsigned_abs();
        Big {
            negative,
            mag: mag_from_u64(m),
        }
    }

    pub fn from_u64(v: u64) -> Big {
        Big {
            negative: false,
            mag: mag_from_u64(v),
        }
    }

    /// The value as an `i64`, when it fits.
    pub fn to_i64(&self) -> Option<i64> {
        if self.mag.len() > 2 {
            return None;
        }
        let mut m: u64 = 0;
        for (i, piece) in self.mag.iter().enumerate() {
            m |= (*piece as u64) << (32 * i);
        }
        if self.negative {
            if m > (i64::MAX as u64) + 1 {
                return None;
            }
            if m == (i64::MAX as u64) + 1 {
                return Some(i64::MIN);
            }
            Some(-(m as i64))
        } else {
            if m > i64::MAX as u64 {
                return None;
            }
            Some(m as i64)
        }
    }

    pub fn is_zero(&self) -> bool {
        self.mag.is_empty()
    }

    pub fn is_negative(&self) -> bool {
        self.negative
    }

    pub fn is_even(&self) -> bool {
        self.mag.first().map(|p| p % 2 == 0).unwrap_or(true)
    }

    pub fn negated(&self) -> Big {
        if self.is_zero() {
            return Big::zero();
        }
        Big {
            negative: !self.negative,
            mag: self.mag.clone(),
        }
    }

    pub fn abs(&self) -> Big {
        Big {
            negative: false,
            mag: self.mag.clone(),
        }
    }

    /// How many 32-bit pieces it takes, which is a fair measure of how big it is.
    pub fn pieces(&self) -> usize {
        self.mag.len()
    }

    pub fn from_decimal(text: &str) -> Option<Big> {
        let text = text.trim();
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let mut out = Big::zero();
        // Nine digits at a time is the most that always fits in a u32 chunk.
        for chunk in chunks_of(digits, 9) {
            let scale = 10u32.pow(chunk.len() as u32);
            out = out.multiplied_small(scale);
            out = out.added_small(chunk.parse::<u32>().ok()?);
        }
        if negative && !out.is_zero() {
            out.negative = true;
        }
        Some(out)
    }

    pub fn to_decimal(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let mut groups: Vec<u32> = Vec::new();
        let mut left = self.abs();
        while !left.is_zero() {
            let (q, r) = left.divided_small(1_000_000_000);
            groups.push(r);
            left = q;
        }
        let mut out = String::with_capacity(groups.len() * 9 + 1);
        if self.negative {
            out.push('-');
        }
        out.push_str(&groups[groups.len() - 1].to_string());
        for g in groups.iter().rev().skip(1) {
            out.push_str(&format!("{g:09}"));
        }
        out
    }

    pub fn to_f64(&self) -> f64 {
        let mut out = 0.0f64;
        for piece in self.mag.iter().rev() {
            out = out * 4_294_967_296.0 + *piece as f64;
        }
        if self.negative {
            -out
        } else {
            out
        }
    }

    pub fn compare(&self, other: &Big) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => compare_mag(&self.mag, &other.mag),
            (true, true) => compare_mag(&other.mag, &self.mag),
        }
    }

    pub fn add(&self, other: &Big) -> Big {
        if self.negative == other.negative {
            Big {
                negative: self.negative,
                mag: add_mag(&self.mag, &other.mag),
            }
            .settled()
        } else {
            match compare_mag(&self.mag, &other.mag) {
                Ordering::Equal => Big::zero(),
                Ordering::Greater => Big {
                    negative: self.negative,
                    mag: sub_mag(&self.mag, &other.mag),
                }
                .settled(),
                Ordering::Less => Big {
                    negative: other.negative,
                    mag: sub_mag(&other.mag, &self.mag),
                }
                .settled(),
            }
        }
    }

    pub fn sub(&self, other: &Big) -> Big {
        self.add(&other.negated())
    }

    pub fn mul(&self, other: &Big) -> Big {
        if self.is_zero() || other.is_zero() {
            return Big::zero();
        }
        Big {
            negative: self.negative != other.negative,
            mag: mul_mag(&self.mag, &other.mag),
        }
        .settled()
    }

    /// Division that rounds towards zero, with the remainder taking the sign of the left side —
    /// the same rule the small whole numbers follow, so the join stays invisible.
    pub fn div_rem(&self, other: &Big) -> Option<(Big, Big)> {
        if other.is_zero() {
            return None;
        }
        let (q_mag, r_mag) = div_mag(&self.mag, &other.mag);
        let quotient = Big {
            negative: self.negative != other.negative,
            mag: q_mag,
        }
        .settled();
        let remainder = Big {
            negative: self.negative,
            mag: r_mag,
        }
        .settled();
        Some((quotient, remainder))
    }

    pub fn pow(&self, exponent: u32) -> Big {
        let mut result = Big::from_i64(1);
        let mut base = self.clone();
        let mut n = exponent;
        while n > 0 {
            if n & 1 == 1 {
                result = result.mul(&base);
            }
            n >>= 1;
            if n > 0 {
                base = base.mul(&base);
            }
        }
        result
    }

    /// The greatest common factor, by Stein's method: halving and subtracting, never dividing.
    pub fn gcd(&self, other: &Big) -> Big {
        let mut a = self.abs();
        let mut b = other.abs();
        if a.is_zero() {
            return b;
        }
        if b.is_zero() {
            return a;
        }
        let mut shift = 0u32;
        while a.is_even() && b.is_even() {
            a = a.halved();
            b = b.halved();
            shift += 1;
        }
        while a.is_even() {
            a = a.halved();
        }
        loop {
            while b.is_even() {
                b = b.halved();
            }
            if a.compare(&b) == Ordering::Greater {
                std::mem::swap(&mut a, &mut b);
            }
            b = b.sub(&a);
            if b.is_zero() {
                break;
            }
        }
        let mut out = a;
        for _ in 0..shift {
            out = out.doubled();
        }
        out
    }

    fn halved(&self) -> Big {
        let mut mag = self.mag.clone();
        let mut carry = 0u32;
        for piece in mag.iter_mut().rev() {
            let current = *piece;
            *piece = (current >> 1) | (carry << 31);
            carry = current & 1;
        }
        Big {
            negative: self.negative,
            mag,
        }
        .settled()
    }

    fn doubled(&self) -> Big {
        self.add(self)
    }

    fn added_small(&self, v: u32) -> Big {
        self.add(&Big::from_u64(v as u64))
    }

    fn multiplied_small(&self, v: u32) -> Big {
        if v == 0 || self.is_zero() {
            return Big::zero();
        }
        let mut mag = Vec::with_capacity(self.mag.len() + 1);
        let mut carry: u64 = 0;
        for piece in &self.mag {
            let wide = *piece as u64 * v as u64 + carry;
            mag.push(wide as u32);
            carry = wide >> 32;
        }
        while carry > 0 {
            mag.push(carry as u32);
            carry >>= 32;
        }
        Big {
            negative: self.negative,
            mag,
        }
        .settled()
    }

    /// Divide by something small. Used by `to_decimal`, where it is the whole cost.
    fn divided_small(&self, v: u32) -> (Big, u32) {
        let mut mag = vec![0u32; self.mag.len()];
        let mut rest: u64 = 0;
        for i in (0..self.mag.len()).rev() {
            let wide = (rest << 32) | self.mag[i] as u64;
            mag[i] = (wide / v as u64) as u32;
            rest = wide % v as u64;
        }
        (
            Big {
                negative: self.negative,
                mag,
            }
            .settled(),
            rest as u32,
        )
    }

    fn settled(mut self) -> Big {
        while self.mag.last() == Some(&0) {
            self.mag.pop();
        }
        if self.mag.is_empty() {
            self.negative = false;
        }
        self
    }
}

fn mag_from_u64(v: u64) -> Vec<u32> {
    if v == 0 {
        return Vec::new();
    }
    let low = v as u32;
    let high = (v >> 32) as u32;
    if high == 0 {
        vec![low]
    } else {
        vec![low, high]
    }
}

fn chunks_of(digits: &str, size: usize) -> Vec<&str> {
    // The first chunk is the short one, so every later chunk is exactly `size` digits and can be
    // scaled by a fixed power of ten.
    let mut out = Vec::new();
    let first = digits.len() % size;
    if first > 0 {
        out.push(&digits[..first]);
    }
    let mut i = first;
    while i < digits.len() {
        out.push(&digits[i..i + size]);
        i += size;
    }
    out
}

fn compare_mag(a: &[u32], b: &[u32]) -> Ordering {
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    for i in (0..a.len()).rev() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
    }
    Ordering::Equal
}

fn add_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut out = Vec::with_capacity(long.len() + 1);
    let mut carry: u64 = 0;
    for i in 0..long.len() {
        let wide = long[i] as u64 + short.get(i).copied().unwrap_or(0) as u64 + carry;
        out.push(wide as u32);
        carry = wide >> 32;
    }
    if carry > 0 {
        out.push(carry as u32);
    }
    out
}

/// `a - b`, where `a` is known not to be smaller.
fn sub_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow: i64 = 0;
    for i in 0..a.len() {
        let mut wide = a[i] as i64 - b.get(i).copied().unwrap_or(0) as i64 - borrow;
        if wide < 0 {
            wide += 1 << 32;
            borrow = 1;
        } else {
            borrow = 0;
        }
        out.push(wide as u32);
    }
    out
}

fn mul_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = vec![0u32; a.len() + b.len()];
    for (i, x) in a.iter().enumerate() {
        if *x == 0 {
            continue;
        }
        let mut carry: u64 = 0;
        for (j, y) in b.iter().enumerate() {
            let at = i + j;
            let wide = out[at] as u64 + *x as u64 * *y as u64 + carry;
            out[at] = wide as u32;
            carry = wide >> 32;
        }
        let mut at = i + b.len();
        while carry > 0 {
            let wide = out[at] as u64 + carry;
            out[at] = wide as u32;
            carry = wide >> 32;
            at += 1;
        }
    }
    out
}

/// Long division, one bit at a time.
///
/// Slower than the cleverer methods, and small enough to be plainly correct, which is the better
/// trade for something that has to be right every time.
fn div_mag(a: &[u32], b: &[u32]) -> (Vec<u32>, Vec<u32>) {
    if compare_mag(a, b) == Ordering::Less {
        return (Vec::new(), a.to_vec());
    }
    let bits = a.len() * 32;
    let mut quotient = vec![0u32; a.len()];
    let mut remainder: Vec<u32> = Vec::with_capacity(b.len() + 1);

    for i in (0..bits).rev() {
        // Shift the remainder up by one bit and bring the next bit of `a` down into it.
        shift_up_one(&mut remainder);
        let bit = (a[i / 32] >> (i % 32)) & 1;
        if bit == 1 {
            if remainder.is_empty() {
                remainder.push(1);
            } else {
                remainder[0] |= 1;
            }
        }
        if compare_mag(&remainder, b) != Ordering::Less {
            remainder = sub_mag(&remainder, b);
            while remainder.last() == Some(&0) {
                remainder.pop();
            }
            quotient[i / 32] |= 1 << (i % 32);
        }
    }

    while quotient.last() == Some(&0) {
        quotient.pop();
    }
    (quotient, remainder)
}

fn shift_up_one(mag: &mut Vec<u32>) {
    let mut carry = 0u32;
    for piece in mag.iter_mut() {
        let current = *piece;
        *piece = (current << 1) | carry;
        carry = current >> 31;
    }
    if carry > 0 {
        mag.push(carry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big(s: &str) -> Big {
        Big::from_decimal(s).unwrap()
    }

    #[test]
    fn small_numbers_go_in_and_come_back_out() {
        for v in [0i64, 1, -1, 42, -42, i64::MAX, i64::MIN] {
            let b = Big::from_i64(v);
            assert_eq!(b.to_i64(), Some(v), "for {v}");
            assert_eq!(b.to_decimal(), v.to_string(), "for {v}");
        }
    }

    #[test]
    fn text_goes_in_and_comes_back_out() {
        for s in [
            "0",
            "7",
            "-7",
            "1000000000",
            "123456789012345678901234567890",
            "-99999999999999999999999999999999999999",
        ] {
            assert_eq!(big(s).to_decimal(), s, "for {s}");
        }
        assert!(Big::from_decimal("").is_none());
        assert!(Big::from_decimal("twelve").is_none());
        assert!(Big::from_decimal("1.5").is_none());
    }

    #[test]
    fn adding_and_taking_away_agree_with_small_numbers() {
        for a in [-97i64, -1, 0, 1, 5, 12345] {
            for b in [-31i64, -1, 0, 1, 7, 99999] {
                assert_eq!(
                    Big::from_i64(a).add(&Big::from_i64(b)).to_i64(),
                    Some(a + b),
                    "{a} + {b}"
                );
                assert_eq!(
                    Big::from_i64(a).sub(&Big::from_i64(b)).to_i64(),
                    Some(a - b),
                    "{a} - {b}"
                );
                assert_eq!(
                    Big::from_i64(a).mul(&Big::from_i64(b)).to_i64(),
                    Some(a * b),
                    "{a} * {b}"
                );
            }
        }
    }

    #[test]
    fn dividing_agrees_with_small_numbers_including_the_signs() {
        for a in [-97i64, -7, 0, 7, 97, 1000] {
            for b in [-9i64, -3, 3, 9] {
                let (q, r) = Big::from_i64(a).div_rem(&Big::from_i64(b)).unwrap();
                assert_eq!(q.to_i64(), Some(a / b), "{a} / {b}");
                assert_eq!(r.to_i64(), Some(a % b), "{a} rem {b}");
            }
        }
        assert!(Big::from_i64(1).div_rem(&Big::zero()).is_none());
    }

    #[test]
    fn it_keeps_going_past_where_a_small_number_stops() {
        // 2^64, which no i64 can hold.
        let v = Big::from_i64(2).pow(64);
        assert_eq!(v.to_decimal(), "18446744073709551616");
        assert_eq!(v.to_i64(), None);

        // And a hundred factorial, which is the usual thing to ask of a number with no limit.
        let mut f = Big::from_i64(1);
        for n in 1..=100i64 {
            f = f.mul(&Big::from_i64(n));
        }
        assert_eq!(
            f.to_decimal(),
            "93326215443944152681699238856266700490715968264381621468592963895217599993229915\
             608941463976156518286253697920827223758251185210916864000000000000000000000000"
        );
    }

    #[test]
    fn big_division_undoes_big_multiplication() {
        let a = big("123456789012345678901234567890");
        let b = big("987654321098765432109876");
        let product = a.mul(&b);
        let (q, r) = product.div_rem(&b).unwrap();
        assert_eq!(q.compare(&a), Ordering::Equal);
        assert!(r.is_zero());
    }

    #[test]
    fn common_factors_are_found_without_dividing() {
        assert_eq!(
            Big::from_i64(12).gcd(&Big::from_i64(18)).to_i64(),
            Some(6)
        );
        assert_eq!(Big::from_i64(0).gcd(&Big::from_i64(5)).to_i64(), Some(5));
        assert_eq!(
            Big::from_i64(-12).gcd(&Big::from_i64(18)).to_i64(),
            Some(6)
        );
        let a = big("123456789012345678901234567890");
        let b = big("246913578024691357802469135780"); // twice a
        assert_eq!(a.gcd(&b).compare(&a), Ordering::Equal);
    }

    #[test]
    fn comparing_works_across_the_signs() {
        assert_eq!(Big::from_i64(-5).compare(&Big::from_i64(3)), Ordering::Less);
        assert_eq!(Big::from_i64(3).compare(&Big::from_i64(-5)), Ordering::Greater);
        assert_eq!(Big::from_i64(-5).compare(&Big::from_i64(-5)), Ordering::Equal);
        assert_eq!(
            big("100000000000000000000").compare(&big("99999999999999999999")),
            Ordering::Greater
        );
    }

    #[test]
    fn there_is_only_one_zero() {
        let z = Big::from_i64(0);
        assert!(!z.is_negative());
        assert_eq!(z.negated(), z);
        assert_eq!(Big::from_i64(5).sub(&Big::from_i64(5)), z);
    }

    #[test]
    fn very_large_numbers_still_reach_a_decimal() {
        let v = Big::from_i64(10).pow(50);
        assert!((v.to_f64() - 1e50).abs() / 1e50 < 1e-12);
    }
}
