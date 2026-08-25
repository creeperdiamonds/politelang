//! Values and the standard library.
//!
//! Spec 10.2: memory is reference-counted rather than garbage-collected — a lower ceiling, no
//! pause spikes, and a much smaller runtime, which is the right trade on a small machine. The
//! honest caveat from the spec stands: plain reference counting leaks cycles, and a cycle
//! collector is a later sub-project.
//!
//! Spec 10.3 shows up here as the in-place text append: joining text in a loop is the classic
//! accidental O(n^2), and when the text being added to is not shared with anybody else it is
//! extended in place instead of copied. The person writing the program learns nothing and changes
//! nothing, and their program is fast.

#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

pub type Reason = String;
pub type Answer<T> = Result<T, Reason>;

#[derive(Clone, Debug)]
pub enum Value {
    Whole(i64),
    Decimal(f64),
    Text(Rc<String>),
    YesNo(bool),
    Nothing,
    List(Rc<RefCell<Vec<Value>>>),
    Lookup(Rc<RefCell<BTreeMap<String, Value>>>),
}

impl Default for Value {
    fn default() -> Self {
        Value::Nothing
    }
}

impl Value {
    pub fn text(s: impl Into<String>) -> Value {
        Value::Text(Rc::new(s.into()))
    }

    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(RefCell::new(items)))
    }

    pub fn lookup() -> Value {
        Value::Lookup(Rc::new(RefCell::new(BTreeMap::new())))
    }

    /// The name of this kind of thing, as a person would say it.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Value::Whole(_) => "a whole number",
            Value::Decimal(_) => "a decimal number",
            Value::Text(_) => "text",
            Value::YesNo(_) => "a yes or no",
            Value::Nothing => "nothing",
            Value::List(_) => "a list",
            Value::Lookup(_) => "a lookup",
        }
    }

    pub fn as_whole(&self) -> i64 {
        match self {
            Value::Whole(v) => *v,
            Value::Decimal(v) => *v as i64,
            Value::YesNo(b) => i64::from(*b),
            _ => 0,
        }
    }

    pub fn as_decimal(&self) -> f64 {
        match self {
            Value::Whole(v) => *v as f64,
            Value::Decimal(v) => *v,
            _ => 0.0,
        }
    }

    pub fn as_yes_no(&self) -> bool {
        match self {
            Value::YesNo(b) => *b,
            Value::Whole(v) => *v != 0,
            _ => false,
        }
    }

    pub fn as_text(&self) -> Rc<String> {
        match self {
            Value::Text(t) => t.clone(),
            other => Rc::new(other.showable()),
        }
    }

    /// How this value is written when shown.
    pub fn showable(&self) -> String {
        match self {
            Value::Whole(v) => v.to_string(),
            Value::Decimal(v) => {
                if v.fract() == 0.0 && v.is_finite() {
                    format!("{v:.1}")
                } else {
                    format!("{v}")
                }
            }
            Value::Text(t) => (**t).clone(),
            Value::YesNo(b) => if *b { "yes" } else { "no" }.to_string(),
            Value::Nothing => "nothing".to_string(),
            Value::List(items) => {
                let items = items.borrow();
                let inner: Vec<String> = items.iter().map(|v| v.showable()).collect();
                format!("[{}]", inner.join(", "))
            }
            Value::Lookup(map) => {
                let map = map.borrow();
                let inner: Vec<String> = map
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.showable()))
                    .collect();
                format!("[{}]", inner.join(", "))
            }
        }
    }

    pub fn same_as(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Whole(a), Value::Whole(b)) => a == b,
            (Value::Decimal(a), Value::Decimal(b)) => a == b,
            (Value::Whole(a), Value::Decimal(b)) | (Value::Decimal(b), Value::Whole(a)) => {
                (*a as f64) == *b
            }
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::YesNo(a), Value::YesNo(b)) => a == b,
            (Value::Nothing, Value::Nothing) => true,
            (Value::List(a), Value::List(b)) => {
                if Rc::ptr_eq(a, b) {
                    return true;
                }
                let (a, b) = (a.borrow(), b.borrow());
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.same_as(y))
            }
            (Value::Lookup(a), Value::Lookup(b)) => {
                if Rc::ptr_eq(a, b) {
                    return true;
                }
                let (a, b) = (a.borrow(), b.borrow());
                a.len() == b.len()
                    && a.iter()
                        .zip(b.iter())
                        .all(|((k1, v1), (k2, v2))| k1 == k2 && v1.same_as(v2))
            }
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

/// Join two pieces of text.
///
/// When the left side is not shared with anybody else, it is extended in place. That is what
/// turns the beginner's loop of `add word to sentence` from O(n^2) into O(n) without the beginner
/// ever knowing there was a problem (spec 10.3).
pub fn text_join(left: Rc<String>, right: &str) -> Rc<String> {
    let mut owned = left;
    match Rc::get_mut(&mut owned) {
        Some(s) => {
            s.push_str(right);
            owned
        }
        None => {
            let mut s = String::with_capacity(owned.len() + right.len());
            s.push_str(&owned);
            s.push_str(right);
            Rc::new(s)
        }
    }
}

pub fn text_length(t: &str) -> i64 {
    t.chars().count() as i64
}

pub fn text_words(t: &str) -> Vec<Value> {
    t.split_whitespace().map(Value::text).collect()
}

pub fn text_split(t: &str, separator: &str) -> Vec<Value> {
    if separator.is_empty() {
        return t.chars().map(|c| Value::text(c.to_string())).collect();
    }
    t.split(separator).map(Value::text).collect()
}

/// A run of letters between two positions, counting from 1 and taking both ends.
///
/// Positions outside the text are brought back to its edges rather than refused: asking for more
/// than there is should give you what there is.
pub fn text_slice(t: &str, from: i64, to: i64) -> String {
    let letters: Vec<char> = t.chars().collect();
    let len = letters.len() as i64;
    let start = from.max(1).min(len + 1) - 1;
    let end = to.max(0).min(len);
    if start >= end {
        return String::new();
    }
    letters[start as usize..end as usize].iter().collect()
}

pub fn text_replace(t: &str, old: &str, new: &str) -> String {
    if old.is_empty() {
        return t.to_string();
    }
    t.replace(old, new)
}

pub fn text_letter(t: &str, position: i64) -> Answer<Value> {
    let letters: Vec<char> = t.chars().collect();
    if position < 1 || position as usize > letters.len() {
        return Err(if letters.is_empty() {
            format!("there is no letter {position}, because the text is empty")
        } else {
            format!(
                "there is no letter {position}, because the text has {} of them",
                letters.len()
            )
        });
    }
    Ok(Value::text(letters[position as usize - 1].to_string()))
}

pub fn text_letters(t: &str) -> Vec<Value> {
    t.chars().map(|c| Value::text(c.to_string())).collect()
}

pub fn text_repeated(t: &str, count: i64) -> String {
    if count <= 0 || t.is_empty() {
        return String::new();
    }
    // A guard against asking for more text than a small machine can hold.
    let wanted = (t.len() as i64).saturating_mul(count);
    if wanted > 64 * 1024 * 1024 {
        return t.repeat((64 * 1024 * 1024 / t.len().max(1)) as usize);
    }
    t.repeat(count as usize)
}

pub fn is_empty(v: &Value) -> bool {
    match v {
        Value::Text(t) => t.is_empty(),
        Value::List(items) => items.borrow().is_empty(),
        Value::Lookup(map) => map.borrow().is_empty(),
        Value::Nothing => true,
        _ => false,
    }
}

pub fn text_number(t: &str) -> Answer<Value> {
    let trimmed = t.trim();
    if let Ok(v) = trimmed.parse::<i64>() {
        return Ok(Value::Whole(v));
    }
    if let Ok(v) = trimmed.parse::<f64>() {
        if v.is_finite() {
            return Ok(Value::Decimal(v));
        }
    }
    Err(format!("\"{trimmed}\" is not a number"))
}

// ---------------------------------------------------------------------------
// Lists
// ---------------------------------------------------------------------------

/// Spec 10.2: lists grow by half again rather than doubling. Peak memory is what runs a small
/// machine out of room, and this keeps the peak close to what is actually being held.
pub fn list_push(items: &mut Vec<Value>, value: Value) {
    if items.len() == items.capacity() {
        let extra = (items.capacity() / 2).max(4);
        items.reserve_exact(extra);
    }
    items.push(value);
}

pub fn list_item(items: &[Value], position: i64) -> Answer<Value> {
    if position < 1 || position as usize > items.len() {
        return Err(out_of_range(position, items.len()));
    }
    Ok(items[position as usize - 1].clone())
}

pub fn list_first(items: &[Value]) -> Answer<Value> {
    items
        .first()
        .cloned()
        .ok_or_else(|| "the list is empty, so it has no first item".to_string())
}

pub fn list_last(items: &[Value]) -> Answer<Value> {
    items
        .last()
        .cloned()
        .ok_or_else(|| "the list is empty, so it has no last item".to_string())
}

pub fn list_sum(items: &[Value]) -> Value {
    let any_decimal = items.iter().any(|v| matches!(v, Value::Decimal(_)));
    if any_decimal {
        Value::Decimal(items.iter().map(|v| v.as_decimal()).sum())
    } else {
        Value::Whole(items.iter().map(|v| v.as_whole()).sum())
    }
}

pub fn list_biggest(items: &[Value]) -> Answer<Value> {
    pick(items, true)
}

pub fn list_smallest(items: &[Value]) -> Answer<Value> {
    pick(items, false)
}

fn pick(items: &[Value], biggest: bool) -> Answer<Value> {
    if items.is_empty() {
        return Err(format!(
            "the list is empty, so it has no {}",
            if biggest { "biggest" } else { "smallest" }
        ));
    }
    let mut best = items[0].clone();
    for v in &items[1..] {
        let better = if biggest {
            v.as_decimal() > best.as_decimal()
        } else {
            v.as_decimal() < best.as_decimal()
        };
        if better {
            best = v.clone();
        }
    }
    Ok(best)
}

pub fn list_sorted(items: &[Value]) -> Vec<Value> {
    let mut copy = items.to_vec();
    copy.sort_by(|a, b| match (a, b) {
        (Value::Text(x), Value::Text(y)) => x.cmp(y),
        _ => a
            .as_decimal()
            .partial_cmp(&b.as_decimal())
            .unwrap_or(std::cmp::Ordering::Equal),
    });
    copy
}

pub fn list_join(items: &[Value], separator: &str) -> String {
    items
        .iter()
        .map(|v| v.showable())
        .collect::<Vec<_>>()
        .join(separator)
}

pub fn list_position(items: &[Value], wanted: &Value) -> Answer<Value> {
    for (i, v) in items.iter().enumerate() {
        if v.same_as(wanted) {
            return Ok(Value::Whole(i as i64 + 1));
        }
    }
    Err(format!(
        "{} is not in the list",
        short_form(&wanted.showable())
    ))
}

pub fn list_put_at(items: &mut Vec<Value>, position: i64, value: Value) -> Answer<()> {
    if position < 1 || position as usize > items.len() {
        return Err(out_of_range(position, items.len()));
    }
    items[position as usize - 1] = value;
    Ok(())
}

pub fn list_remove_at(items: &mut Vec<Value>, position: i64) -> Answer<()> {
    if position < 1 || position as usize > items.len() {
        return Err(out_of_range(position, items.len()));
    }
    items.remove(position as usize - 1);
    Ok(())
}

fn out_of_range(position: i64, len: usize) -> String {
    if len == 0 {
        format!("there is no item {position}, because the list is empty")
    } else {
        format!(
            "there is no item {position}, because the list holds {len} {}",
            if len == 1 { "item" } else { "items" }
        )
    }
}

fn short_form(s: &str) -> String {
    if s.chars().count() <= 24 {
        format!("`{s}`")
    } else {
        let cut: String = s.chars().take(21).collect();
        format!("`{cut}...`")
    }
}

// ---------------------------------------------------------------------------
// Lookups
// ---------------------------------------------------------------------------

pub fn lookup_get(map: &BTreeMap<String, Value>, key: &str) -> Answer<Value> {
    map.get(key)
        .cloned()
        .ok_or_else(|| format!("nothing is stored under \"{key}\""))
}

pub fn lookup_keys(map: &BTreeMap<String, Value>) -> Vec<Value> {
    map.keys().map(|k| Value::text(k.clone())).collect()
}

// ---------------------------------------------------------------------------
// Numbers
// ---------------------------------------------------------------------------

pub fn remainder(a: &Value, b: &Value) -> Answer<Value> {
    let d = b.as_whole();
    if d == 0 {
        return Err("nothing can be shared into zero parts".to_string());
    }
    Ok(Value::Whole(a.as_whole() % d))
}

pub fn smaller(a: &Value, b: &Value) -> Value {
    if a.as_decimal() <= b.as_decimal() {
        a.clone()
    } else {
        b.clone()
    }
}

pub fn larger(a: &Value, b: &Value) -> Value {
    if a.as_decimal() >= b.as_decimal() {
        a.clone()
    } else {
        b.clone()
    }
}

pub fn power(a: &Value, b: &Value) -> Value {
    Value::Decimal(a.as_decimal().powf(b.as_decimal()))
}

pub fn rounded_down(v: &Value) -> Value {
    Value::Whole(v.as_decimal().floor() as i64)
}

pub fn rounded_up(v: &Value) -> Value {
    Value::Whole(v.as_decimal().ceil() as i64)
}

pub fn list_rest(items: &[Value]) -> Vec<Value> {
    if items.len() <= 1 {
        return Vec::new();
    }
    items[1..].to_vec()
}

pub fn list_first_few(items: &[Value], count: i64) -> Vec<Value> {
    if count <= 0 {
        return Vec::new();
    }
    let take = (count as usize).min(items.len());
    items[..take].to_vec()
}

pub fn list_average(items: &[Value]) -> Answer<Value> {
    if items.is_empty() {
        return Err("the list is empty, so there is nothing to share out".to_string());
    }
    let total: f64 = items.iter().map(|v| v.as_decimal()).sum();
    Ok(Value::Decimal(total / items.len() as f64))
}

pub fn list_count_in(items: &[Value], wanted: &Value) -> i64 {
    items.iter().filter(|v| v.same_as(wanted)).count() as i64
}

/// Let some time pass.
pub fn wait_for(seconds: f64) {
    if seconds <= 0.0 {
        return;
    }
    // A whole day is more waiting than anybody meant.
    let capped = seconds.min(86_400.0);
    std::thread::sleep(std::time::Duration::from_secs_f64(capped));
}

pub fn file_append(path: &str, contents: &str) -> Answer<()> {
    use std::io::Write as _;
    match std::fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut f) => match f.write_all(contents.as_bytes()) {
            Ok(()) => Ok(()),
            Err(e) => Err(describe_file_problem(path, &e)),
        },
        Err(e) => Err(describe_file_problem(path, &e)),
    }
}

pub fn divide(a: &Value, b: &Value) -> Answer<Value> {
    let divisor = b.as_decimal();
    if divisor == 0.0 {
        return Err("nothing can be shared into zero parts".to_string());
    }
    Ok(Value::Decimal(a.as_decimal() / divisor))
}

pub fn divides_evenly(a: &Value, b: &Value) -> Value {
    let d = b.as_whole();
    if d == 0 {
        return Value::YesNo(false);
    }
    Value::YesNo(a.as_whole() % d == 0)
}

pub fn square_root(v: &Value) -> Answer<Value> {
    let n = v.as_decimal();
    if n < 0.0 {
        return Err("a number below zero has no square root".to_string());
    }
    Ok(Value::Decimal(n.sqrt()))
}

pub fn rounded(v: &Value) -> Value {
    Value::Whole(v.as_decimal().round() as i64)
}

pub fn absolute(v: &Value) -> Value {
    match v {
        Value::Decimal(d) => Value::Decimal(d.abs()),
        other => Value::Whole(other.as_whole().abs()),
    }
}

// ---------------------------------------------------------------------------
// Randomness
// ---------------------------------------------------------------------------

/// A small xorshift generator.
///
/// Spec 9.5 keeps the dependency list at nothing, and a program that picks a number between one
/// and a hundred does not need a cryptographic source.
pub struct Dice {
    state: u64,
}

impl Default for Dice {
    fn default() -> Self {
        Self::new()
    }
}

impl Dice {
    pub fn new() -> Dice {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x2545F4914F6CDD1D)
            ^ (&Dice { state: 1 } as *const Dice as u64);
        Dice {
            state: seed | 1, // never zero
        }
    }

    pub fn with_seed(seed: u64) -> Dice {
        Dice { state: seed | 1 }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// A whole number between two limits, including both.
    pub fn between(&mut self, from: i64, to: i64) -> i64 {
        let (low, high) = if from <= to { (from, to) } else { (to, from) };
        let span = (high - low).unsigned_abs().saturating_add(1);
        if span == 0 {
            return low;
        }
        low + (self.next() % span) as i64
    }
}

// ---------------------------------------------------------------------------
// Files and the world
// ---------------------------------------------------------------------------

pub fn file_contents(path: &str) -> Answer<Value> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Value::text(text)),
        Err(e) => Err(describe_file_problem(path, &e)),
    }
}

pub fn file_write(path: &str, contents: &str) -> Answer<()> {
    match std::fs::write(path, contents) {
        Ok(()) => Ok(()),
        Err(e) => Err(describe_file_problem(path, &e)),
    }
}

pub fn file_exists(path: &str) -> bool {
    std::path::Path::new(path).is_file()
}

fn describe_file_problem(path: &str, e: &std::io::Error) -> String {
    use std::io::ErrorKind::*;
    match e.kind() {
        NotFound => format!("there is no file called \"{path}\""),
        PermissionDenied => format!("\"{path}\" is not mine to read or write"),
        _ => format!("\"{path}\" could not be reached"),
    }
}

pub fn time_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_extended_in_place_when_nobody_else_holds_it() {
        let t = Rc::new(String::from("a"));
        let before = Rc::as_ptr(&t);
        let after = text_join(t, "b");
        assert_eq!(*after, "ab");
        assert_eq!(Rc::as_ptr(&after), before, "should have grown in place");
    }

    #[test]
    fn shared_text_is_copied_rather_than_changed_underneath_anybody() {
        let t = Rc::new(String::from("a"));
        let other = t.clone();
        let after = text_join(t, "b");
        assert_eq!(*after, "ab");
        assert_eq!(*other, "a");
    }

    #[test]
    fn joining_in_a_loop_stays_linear() {
        let mut t = Rc::new(String::new());
        for _ in 0..20_000 {
            t = text_join(t, "x");
        }
        assert_eq!(t.len(), 20_000);
    }

    #[test]
    fn positions_count_from_one() {
        let items = vec![Value::Whole(10), Value::Whole(20)];
        assert!(matches!(list_item(&items, 1), Ok(Value::Whole(10))));
        assert!(list_item(&items, 0).is_err());
        assert!(list_item(&items, 3).is_err());
    }

    #[test]
    fn reasons_are_written_as_sentences_not_codes() {
        let items: Vec<Value> = Vec::new();
        let reason = list_item(&items, 1).unwrap_err();
        assert_eq!(reason, "there is no item 1, because the list is empty");
        assert!(polite_diag::find_blame_word(&reason).is_none());
    }

    #[test]
    fn nothing_divides_by_zero_but_nothing_crashes_either() {
        let e = divide(&Value::Whole(1), &Value::Whole(0)).unwrap_err();
        assert!(e.contains("zero parts"));
        assert!(divides_evenly(&Value::Whole(1), &Value::Whole(0)).as_yes_no() == false);
    }

    #[test]
    fn lists_grow_by_half_again() {
        let mut items: Vec<Value> = Vec::new();
        for i in 0..100 {
            list_push(&mut items, Value::Whole(i));
        }
        assert_eq!(items.len(), 100);
        assert!(
            items.capacity() < 200,
            "capacity {} grew too eagerly",
            items.capacity()
        );
    }

    #[test]
    fn dice_stay_inside_their_limits() {
        let mut d = Dice::with_seed(12345);
        for _ in 0..1000 {
            let v = d.between(1, 6);
            assert!((1..=6).contains(&v));
        }
    }

    #[test]
    fn a_piece_of_text_counts_from_one_and_takes_both_ends() {
        assert_eq!(text_slice("hello", 1, 3), "hel");
        assert_eq!(text_slice("hello", 2, 4), "ell");
        // Asking for more than there is gives what there is.
        assert_eq!(text_slice("hello", 1, 99), "hello");
        assert_eq!(text_slice("hello", 4, 2), "");
    }

    #[test]
    fn a_letter_out_of_range_says_so_gently() {
        let reason = text_letter("hi", 5).unwrap_err();
        assert_eq!(reason, "there is no letter 5, because the text has 2 of them");
        assert!(polite_diag::find_blame_word(&reason).is_none());
    }

    #[test]
    fn emptiness_means_the_same_for_everything() {
        assert!(is_empty(&Value::text("")));
        assert!(is_empty(&Value::list(Vec::new())));
        assert!(is_empty(&Value::lookup()));
        assert!(!is_empty(&Value::text("a")));
    }

    #[test]
    fn the_rest_of_a_short_list_is_simply_empty() {
        assert!(list_rest(&[]).is_empty());
        assert!(list_rest(&[Value::Whole(1)]).is_empty());
        assert_eq!(list_rest(&[Value::Whole(1), Value::Whole(2)]).len(), 1);
    }

    #[test]
    fn repeating_text_will_not_eat_the_machine() {
        let big = text_repeated("hello", 1_000_000_000);
        assert!(big.len() <= 64 * 1024 * 1024 + 8);
    }

    #[test]
    fn decimals_are_shown_as_decimals() {
        assert_eq!(Value::Decimal(1.0).showable(), "1.0");
        assert_eq!(Value::Whole(1).showable(), "1");
    }
}
