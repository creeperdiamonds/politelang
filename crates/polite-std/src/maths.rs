//! Number theory, combinatorics, number bases, bits, and statistics.

use crate::big::Big;
use crate::numbers;
use crate::{Answer, Value};

// ---------------------------------------------------------------------------
// Primes and factors
// ---------------------------------------------------------------------------

/// Whether a number is prime, decided rather than guessed.
///
/// Miller and Rabin's test, with the set of witnesses that is known to be exactly right for every
/// number a 64-bit whole number can hold. No chance is involved and no answer is approximate.
pub fn is_prime(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    for small in [2i64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if n == small {
            return true;
        }
        if n % small == 0 {
            return false;
        }
    }
    let n = n as u128;
    let mut d = n - 1;
    let mut twos = 0u32;
    while d % 2 == 0 {
        d /= 2;
        twos += 1;
    }
    'witness: for a in [2u128, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        let mut x = mod_pow(a, d, n);
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 1..twos {
            x = x * x % n;
            if x == n - 1 {
                continue 'witness;
            }
        }
        return false;
    }
    true
}

fn mod_pow(mut base: u128, mut power: u128, modulus: u128) -> u128 {
    let mut out: u128 = 1;
    base %= modulus;
    while power > 0 {
        if power & 1 == 1 {
            out = out * base % modulus;
        }
        base = base * base % modulus;
        power >>= 1;
    }
    out
}

/// Every prime that multiplies together to make this number, smallest first and repeated as often
/// as it appears.
pub fn prime_factors(n: i64) -> Vec<Value> {
    let mut out = Vec::new();
    let mut left = n.saturating_abs();
    if left < 2 {
        return out;
    }
    let mut factor = 2i64;
    while factor.saturating_mul(factor) <= left {
        while left % factor == 0 {
            out.push(Value::Whole(factor));
            left /= factor;
        }
        factor += if factor == 2 { 1 } else { 2 };
    }
    if left > 1 {
        out.push(Value::Whole(left));
    }
    out
}

/// Every whole number that goes evenly into this one, in order.
pub fn divisors(n: i64) -> Vec<Value> {
    let n = n.saturating_abs();
    if n == 0 {
        return Vec::new();
    }
    let mut small = Vec::new();
    let mut large = Vec::new();
    let mut d = 1i64;
    while d.saturating_mul(d) <= n {
        if n % d == 0 {
            small.push(Value::Whole(d));
            if d != n / d {
                large.push(Value::Whole(n / d));
            }
        }
        d += 1;
    }
    large.reverse();
    small.extend(large);
    small
}

/// A power worked out within a modulus, which stays small however large the power is.
pub fn power_within(base: i64, power: i64, modulus: i64) -> Answer<Value> {
    if modulus <= 0 {
        return Err("a modulus has to be above zero".to_string());
    }
    if power < 0 {
        return Err("a power within a modulus cannot be below zero".to_string());
    }
    if modulus == 1 {
        return Ok(Value::Whole(0));
    }
    let b = base.rem_euclid(modulus) as u128;
    Ok(Value::Whole(
        mod_pow(b, power as u128, modulus as u128) as i64
    ))
}

/// The number which, times this one, leaves one within the modulus.
pub fn inverse_within(value: i64, modulus: i64) -> Answer<Value> {
    if modulus <= 0 {
        return Err("a modulus has to be above zero".to_string());
    }
    let (mut old_r, mut r) = (value.rem_euclid(modulus), modulus);
    let (mut old_s, mut s) = (1i64, 0i64);
    while r != 0 {
        let q = old_r / r;
        let next_r = old_r - q * r;
        old_r = r;
        r = next_r;
        let next_s = old_s - q * s;
        old_s = s;
        s = next_s;
    }
    if old_r != 1 {
        return Err(format!(
            "{value} and {modulus} share a factor, so there is no such number"
        ));
    }
    Ok(Value::Whole(old_s.rem_euclid(modulus)))
}

// ---------------------------------------------------------------------------
// Combinatorics
// ---------------------------------------------------------------------------

/// How many ways there are to choose some of a number of things, where the order does not matter.
///
/// Worked out with numbers of any size, because these grow very fast.
pub fn ways_to_choose(count: i64, total: i64) -> Answer<Value> {
    if count < 0 || total < 0 {
        return Err("you cannot choose from fewer than none".to_string());
    }
    if count > total {
        return Ok(Value::Whole(0));
    }
    let count = count.min(total - count);
    let mut out = Big::from_i64(1);
    for i in 0..count {
        out = out.mul(&Big::from_i64(total - i));
        let (q, _) = out
            .div_rem(&Big::from_i64(i + 1))
            .ok_or_else(|| "nothing can be shared into zero parts".to_string())?;
        out = q;
    }
    Ok(numbers::from_big(out))
}

/// How many ways there are to arrange some of a number of things, where the order matters.
pub fn ways_to_arrange(count: i64, total: i64) -> Answer<Value> {
    if count < 0 || total < 0 {
        return Err("you cannot arrange fewer than none".to_string());
    }
    if count > total {
        return Ok(Value::Whole(0));
    }
    let mut out = Big::from_i64(1);
    for i in 0..count {
        out = out.mul(&Big::from_i64(total - i));
    }
    Ok(numbers::from_big(out))
}

// ---------------------------------------------------------------------------
// Number bases
// ---------------------------------------------------------------------------

const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

pub fn in_base(value: i64, base: i64) -> Answer<String> {
    if !(2..=36).contains(&base) {
        return Err(format!(
            "{base} is no use as a base; two to thirty six is as far as the digits go"
        ));
    }
    let negative = value < 0;
    let mut left = value.unsigned_abs();
    if left == 0 {
        return Ok("0".to_string());
    }
    let mut out = Vec::new();
    while left > 0 {
        out.push(DIGITS[(left % base as u64) as usize]);
        left /= base as u64;
    }
    if negative {
        out.push(b'-');
    }
    out.reverse();
    Ok(String::from_utf8_lossy(&out).to_string())
}

pub fn value_of_in_base(text: &str, base: i64) -> Answer<Value> {
    if !(2..=36).contains(&base) {
        return Err(format!(
            "{base} is no use as a base; two to thirty six is as far as the digits go"
        ));
    }
    let text = text.trim();
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    if digits.is_empty() {
        return Err("there are no digits here to read".to_string());
    }
    let mut out: i64 = 0;
    for c in digits.chars() {
        let d = match DIGITS.iter().position(|x| *x == c.to_ascii_lowercase() as u8) {
            Some(d) if (d as i64) < base => d as i64,
            _ => {
                return Err(format!(
                    "`{c}` is not a digit in base {base}"
                ))
            }
        };
        out = match out.checked_mul(base).and_then(|v| v.checked_add(d)) {
            Some(v) => v,
            None => return Err(format!("\"{text}\" is larger than I can read this way")),
        };
    }
    Ok(Value::Whole(if negative { -out } else { out }))
}

// ---------------------------------------------------------------------------
// Bits
// ---------------------------------------------------------------------------

pub fn bit_and(a: i64, b: i64) -> Value {
    Value::Whole(a & b)
}

pub fn bit_or(a: i64, b: i64) -> Value {
    Value::Whole(a | b)
}

pub fn bit_exclusive_or(a: i64, b: i64) -> Value {
    Value::Whole(a ^ b)
}

pub fn bit_not(a: i64) -> Value {
    Value::Whole(!a)
}

pub fn shifted_left(a: i64, by: i64) -> Value {
    if !(0..64).contains(&by) {
        return Value::Whole(0);
    }
    Value::Whole(a.wrapping_shl(by as u32))
}

pub fn shifted_right(a: i64, by: i64) -> Value {
    if !(0..64).contains(&by) {
        return Value::Whole(if a < 0 { -1 } else { 0 });
    }
    Value::Whole(a.wrapping_shr(by as u32))
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// The one that turns up most often. Where several tie, the smallest of them.
pub fn mode(items: &[Value]) -> Answer<Value> {
    if items.is_empty() {
        return Err("the list is empty, so nothing turns up at all".to_string());
    }
    let mut best: Option<(&Value, usize)> = None;
    for candidate in items {
        let count = items.iter().filter(|v| v.same_as(candidate)).count();
        let better = match best {
            None => true,
            Some((current, best_count)) => {
                count > best_count
                    || (count == best_count
                        && numbers::compare(candidate, current)
                            == Some(std::cmp::Ordering::Less))
            }
        };
        if better {
            best = Some((candidate, count));
        }
    }
    Ok(best.map(|(v, _)| v.clone()).unwrap_or(Value::Nothing))
}

pub fn variance(items: &[Value]) -> Answer<Value> {
    if items.is_empty() {
        return Err("the list is empty, so it has no variance".to_string());
    }
    let numbers: Vec<f64> = items.iter().map(|v| v.as_decimal()).collect();
    let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;
    let v = numbers.iter().map(|n| (n - mean) * (n - mean)).sum::<f64>() / numbers.len() as f64;
    Ok(Value::Decimal(v))
}

/// How closely two lists move together, from minus one to one.
pub fn correlation(first: &[Value], second: &[Value]) -> Answer<Value> {
    if first.len() != second.len() {
        return Err(format!(
            "one list holds {} and the other holds {}, so they cannot be paired up",
            first.len(),
            second.len()
        ));
    }
    if first.len() < 2 {
        return Err("two lists of fewer than two things do not move together at all".to_string());
    }
    let xs: Vec<f64> = first.iter().map(|v| v.as_decimal()).collect();
    let ys: Vec<f64> = second.iter().map(|v| v.as_decimal()).collect();
    let n = xs.len() as f64;
    let mean_x = xs.iter().sum::<f64>() / n;
    let mean_y = ys.iter().sum::<f64>() / n;
    let mut top = 0.0;
    let mut spread_x = 0.0;
    let mut spread_y = 0.0;
    for i in 0..xs.len() {
        let dx = xs[i] - mean_x;
        let dy = ys[i] - mean_y;
        top += dx * dy;
        spread_x += dx * dx;
        spread_y += dy * dy;
    }
    let bottom = (spread_x * spread_y).sqrt();
    if bottom == 0.0 {
        return Err("one of the lists never changes, so there is nothing to compare".to_string());
    }
    Ok(Value::Decimal(top / bottom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primes_are_told_apart_from_the_rest() {
        let found: Vec<i64> = (1..40).filter(|n| is_prime(*n)).collect();
        assert_eq!(found, vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37]);
        assert!(!is_prime(1));
        assert!(!is_prime(0));
        assert!(!is_prime(-7));
        // Large ones too, without guessing.
        assert!(is_prime(2_147_483_647));
        assert!(!is_prime(2_147_483_649));
        assert!(is_prime(1_000_000_007));
    }

    #[test]
    fn a_number_comes_apart_into_its_primes() {
        let f: Vec<i64> = prime_factors(360).iter().map(|v| v.as_whole()).collect();
        assert_eq!(f, vec![2, 2, 2, 3, 3, 5]);
        let f: Vec<i64> = prime_factors(97).iter().map(|v| v.as_whole()).collect();
        assert_eq!(f, vec![97]);
        assert!(prime_factors(1).is_empty());
    }

    #[test]
    fn divisors_come_out_in_order() {
        let d: Vec<i64> = divisors(28).iter().map(|v| v.as_whole()).collect();
        assert_eq!(d, vec![1, 2, 4, 7, 14, 28]);
        let d: Vec<i64> = divisors(36).iter().map(|v| v.as_whole()).collect();
        assert_eq!(d, vec![1, 2, 3, 4, 6, 9, 12, 18, 36]);
    }

    #[test]
    fn powers_within_a_modulus_stay_small() {
        assert_eq!(power_within(2, 10, 1000).unwrap().as_whole(), 24);
        assert_eq!(power_within(3, 0, 7).unwrap().as_whole(), 1);
        // Which no ordinary power could reach.
        assert_eq!(power_within(7, 1_000_000, 1_000_000_007).unwrap().as_whole() >= 0, true);
        assert!(power_within(2, 3, 0).is_err());
    }

    #[test]
    fn an_inverse_within_a_modulus_undoes_multiplying() {
        let v = inverse_within(3, 11).unwrap().as_whole();
        assert_eq!(3 * v % 11, 1);
        assert!(inverse_within(4, 8).is_err());
    }

    #[test]
    fn choosing_and_arranging_grow_the_way_they_should() {
        assert_eq!(ways_to_choose(2, 5).unwrap().as_whole(), 10);
        assert_eq!(ways_to_choose(0, 5).unwrap().as_whole(), 1);
        assert_eq!(ways_to_choose(6, 5).unwrap().as_whole(), 0);
        assert_eq!(ways_to_arrange(2, 5).unwrap().as_whole(), 20);
        // And past where a small whole number stops.
        assert_eq!(
            ways_to_choose(50, 100).unwrap().showable(),
            "100891344545564193334812497256"
        );
    }

    #[test]
    fn numbers_go_into_other_bases_and_come_back() {
        assert_eq!(in_base(255, 2).unwrap(), "11111111");
        assert_eq!(in_base(255, 16).unwrap(), "ff");
        assert_eq!(in_base(-10, 2).unwrap(), "-1010");
        assert_eq!(in_base(0, 7).unwrap(), "0");
        assert!(in_base(1, 1).is_err());
        assert_eq!(value_of_in_base("ff", 16).unwrap().as_whole(), 255);
        assert_eq!(value_of_in_base("-1010", 2).unwrap().as_whole(), -10);
        assert!(value_of_in_base("2", 2).is_err());
    }

    #[test]
    fn bits_do_what_bits_do() {
        assert_eq!(bit_and(12, 10).as_whole(), 8);
        assert_eq!(bit_or(12, 10).as_whole(), 14);
        assert_eq!(bit_exclusive_or(12, 10).as_whole(), 6);
        assert_eq!(bit_not(0).as_whole(), -1);
        assert_eq!(shifted_left(1, 10).as_whole(), 1024);
        assert_eq!(shifted_right(1024, 10).as_whole(), 1);
        // Silly amounts do not wrap round to a surprise.
        assert_eq!(shifted_left(1, 999).as_whole(), 0);
    }

    #[test]
    fn statistics_agree_with_the_textbook() {
        let items: Vec<Value> = [2, 4, 4, 4, 5, 5, 7, 9].iter().map(|n| Value::Whole(*n)).collect();
        assert_eq!(mode(&items).unwrap().as_whole(), 4);
        assert!((variance(&items).unwrap().as_decimal() - 4.0).abs() < 1e-12);

        let xs: Vec<Value> = (1..=5).map(Value::Whole).collect();
        let ys: Vec<Value> = (1..=5).map(|n| Value::Whole(n * 2)).collect();
        assert!((correlation(&xs, &ys).unwrap().as_decimal() - 1.0).abs() < 1e-12);

        let down: Vec<Value> = (1..=5).map(|n| Value::Whole(-n)).collect();
        assert!((correlation(&xs, &down).unwrap().as_decimal() + 1.0).abs() < 1e-12);

        assert!(correlation(&xs, &[]).is_err());
        assert!(mode(&[]).is_err());
    }
}
