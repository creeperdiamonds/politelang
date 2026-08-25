//! The reference runner — backend number one.
//!
//! Spec 10.2: because every type was worked out before the program ran, this never asks what kind
//! of thing a value is in order to decide what to do. `add.whole` adds whole numbers; the
//! decision was made once, at lowering. A dynamic language makes that decision every time the
//! line runs, and in a loop of a million turns that is a million decisions not made here.

#![forbid(unsafe_code)]

use polite_ir::{Builtin, CmpKind, Compare, FuncId, Function, Instr, Program};
use polite_std::{self as std_lib, Dice, Value};
use std::collections::VecDeque;
use std::io::{BufRead, Write};

/// Everything the runner needs from outside itself, so that tests can watch it without a
/// terminal and a person can use it with one.
pub trait World {
    fn show(&mut self, line: &str);
    /// `None` means there is nothing more to read.
    fn ask(&mut self, prompt: &str) -> Option<String>;
}

/// The real world: standard output and standard input.
pub struct Terminal;

impl World for Terminal {
    fn show(&mut self, line: &str) {
        let out = std::io::stdout();
        let mut out = out.lock();
        let _ = writeln!(out, "{line}");
    }

    fn ask(&mut self, prompt: &str) -> Option<String> {
        let out = std::io::stdout();
        {
            let mut out = out.lock();
            let _ = write!(out, "{prompt}");
            let _ = out.flush();
        }
        let mut line = String::new();
        match std::io::stdin().lock().read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => Some(line.trim_end_matches(['\n', '\r']).to_string()),
            Err(_) => None,
        }
    }
}

/// A world for tests: replies are handed over in order, and everything shown is kept.
#[derive(Default)]
pub struct Scripted {
    pub shown: Vec<String>,
    pub replies: VecDeque<String>,
}

impl Scripted {
    pub fn with_replies<I: IntoIterator<Item = S>, S: Into<String>>(replies: I) -> Scripted {
        Scripted {
            shown: Vec::new(),
            replies: replies.into_iter().map(Into::into).collect(),
        }
    }

    pub fn output(&self) -> String {
        let mut s = self.shown.join("\n");
        if !s.is_empty() {
            s.push('\n');
        }
        s
    }
}

impl World for Scripted {
    fn show(&mut self, line: &str) {
        self.shown.push(line.to_string());
    }
    fn ask(&mut self, _prompt: &str) -> Option<String> {
        self.replies.pop_front()
    }
}

/// How far a program may go before the runner assumes something has gone round in circles.
/// `None` means as far as it likes, which is what a real run wants.
#[derive(Copy, Clone, Default)]
pub struct Limits {
    pub steps: Option<u64>,
    pub depth: u32,
}

impl Limits {
    pub fn none() -> Limits {
        Limits {
            steps: None,
            depth: 512,
        }
    }
    pub fn steps(n: u64) -> Limits {
        Limits {
            steps: Some(n),
            depth: 512,
        }
    }
}

/// Running the program either finishes, or stops politely with something to say.
pub fn run(program: &Program, world: &mut dyn World) -> Result<(), String> {
    run_with(program, world, Limits::none(), None)
}

pub fn run_with(
    program: &Program,
    world: &mut dyn World,
    limits: Limits,
    seed: Option<u64>,
) -> Result<(), String> {
    let mut r = Runner {
        program,
        world,
        dice: match seed {
            Some(s) => Dice::with_seed(s),
            None => Dice::new(),
        },
        limits: Limits {
            steps: limits.steps,
            depth: if limits.depth == 0 { 512 } else { limits.depth },
        },
        steps: 0,
        depth: 0,
    };
    match r.call(program.main, Vec::new())? {
        Outcome::Returned(_) => Ok(()),
        Outcome::Failed(reason) => Err(format!(
            "The program stopped because {reason}. Nothing said what to do if it did not work out."
        )),
    }
}

enum Outcome {
    Returned(Value),
    /// A failure that left the action altogether (spec 7.4).
    Failed(String),
}

struct Runner<'a> {
    program: &'a Program,
    world: &'a mut dyn World,
    dice: Dice,
    limits: Limits,
    steps: u64,
    depth: u32,
}

impl Runner<'_> {
    fn call(&mut self, func: FuncId, args: Vec<Value>) -> Result<Outcome, String> {
        self.depth += 1;
        if self.depth > self.limits.depth {
            self.depth -= 1;
            return Err(
                "This is asking for itself over and over, deeper than I can follow. \
                 Somewhere it needs a check that stops it."
                    .to_string(),
            );
        }
        let f = &self.program.funcs[func as usize];
        let mut slots: Vec<Value> = vec![Value::Nothing; f.slot_count as usize];
        for (slot, value) in f.param_slots.iter().zip(args) {
            if (*slot as usize) < slots.len() {
                slots[*slot as usize] = value;
            }
        }
        let outcome = self.run_body(f, &mut slots);
        self.depth -= 1;
        outcome
    }

    fn run_body(&mut self, f: &Function, slots: &mut [Value]) -> Result<Outcome, String> {
        let mut block = f.entry as usize;
        let mut pc = 0usize;

        loop {
            if let Some(limit) = self.limits.steps {
                self.steps += 1;
                if self.steps > limit {
                    return Err("This has been going for a very long time. I stopped it so we \
                                could take a look together."
                        .to_string());
                }
            }

            let instr = match f.blocks[block].instrs.get(pc) {
                Some(i) => i,
                // A block that simply runs out is the end of the action.
                None => return Ok(Outcome::Returned(Value::Nothing)),
            };
            pc += 1;

            macro_rules! set {
                ($dst:expr, $value:expr) => {
                    slots[*$dst as usize] = $value
                };
            }

            match instr {
                Instr::ConstWhole { dst, value } => set!(dst, Value::Whole(*value)),
                Instr::ConstDecimal { dst, value } => set!(dst, Value::Decimal(*value)),
                Instr::ConstText { dst, text } => {
                    set!(dst, Value::text(self.program.texts[*text as usize].clone()))
                }
                Instr::ConstYesNo { dst, value } => set!(dst, Value::YesNo(*value)),
                Instr::ConstNothing { dst } => set!(dst, Value::Nothing),
                Instr::Move { dst, src } => {
                    let v = slots[*src as usize].clone();
                    set!(dst, v);
                }

                Instr::AddWhole { dst, a, b } => {
                    let (x, y) = (slots[*a as usize].as_whole(), slots[*b as usize].as_whole());
                    match x.checked_add(y) {
                        Some(v) => set!(dst, Value::Whole(v)),
                        None => return Err(too_big()),
                    }
                }
                Instr::SubWhole { dst, a, b } => {
                    let (x, y) = (slots[*a as usize].as_whole(), slots[*b as usize].as_whole());
                    match x.checked_sub(y) {
                        Some(v) => set!(dst, Value::Whole(v)),
                        None => return Err(too_big()),
                    }
                }
                Instr::MulWhole { dst, a, b } => {
                    let (x, y) = (slots[*a as usize].as_whole(), slots[*b as usize].as_whole());
                    match x.checked_mul(y) {
                        Some(v) => set!(dst, Value::Whole(v)),
                        None => return Err(too_big()),
                    }
                }
                Instr::AddDecimal { dst, a, b } => {
                    let v = slots[*a as usize].as_decimal() + slots[*b as usize].as_decimal();
                    set!(dst, Value::Decimal(v));
                }
                Instr::SubDecimal { dst, a, b } => {
                    let v = slots[*a as usize].as_decimal() - slots[*b as usize].as_decimal();
                    set!(dst, Value::Decimal(v));
                }
                Instr::MulDecimal { dst, a, b } => {
                    let v = slots[*a as usize].as_decimal() * slots[*b as usize].as_decimal();
                    set!(dst, Value::Decimal(v));
                }
                Instr::WholeToDecimal { dst, src } => {
                    let v = slots[*src as usize].as_decimal();
                    set!(dst, Value::Decimal(v));
                }
                Instr::NegateWhole { dst, src } => {
                    let v = -slots[*src as usize].as_whole();
                    set!(dst, Value::Whole(v));
                }
                Instr::NegateDecimal { dst, src } => {
                    let v = -slots[*src as usize].as_decimal();
                    set!(dst, Value::Decimal(v));
                }

                Instr::ConcatText { dst, a, b } => {
                    let right = slots[*b as usize].as_text();
                    let left = match &slots[*a as usize] {
                        Value::Text(t) => t.clone(),
                        other => other.as_text(),
                    };
                    // Hand the left side over before joining, so that when this slot held the
                    // only reference the text can grow in place instead of being copied.
                    if *dst == *a {
                        slots[*dst as usize] = Value::Nothing;
                    }
                    let joined = std_lib::text_join(left, &right);
                    set!(dst, Value::Text(joined));
                }

                Instr::Cmp {
                    dst,
                    op,
                    kind,
                    a,
                    b,
                } => {
                    let result = compare(&slots[*a as usize], &slots[*b as usize], *op, *kind);
                    set!(dst, Value::YesNo(result));
                }

                Instr::Not { dst, src } => {
                    let v = !slots[*src as usize].as_yes_no();
                    set!(dst, Value::YesNo(v));
                }

                Instr::Jump { to } => {
                    block = *to as usize;
                    pc = 0;
                }

                Instr::Branch {
                    cond,
                    then_block,
                    else_block,
                } => {
                    block = if slots[*cond as usize].as_yes_no() {
                        *then_block as usize
                    } else {
                        *else_block as usize
                    };
                    pc = 0;
                }

                Instr::Return { src } => {
                    let v = match src {
                        Some(s) => slots[*s as usize].clone(),
                        None => Value::Nothing,
                    };
                    return Ok(Outcome::Returned(v));
                }

                Instr::StopEverything => {
                    return Err("The program said to stop everything, so it did.".to_string())
                }

                Instr::StopBecauseSure { reason, what } => {
                    let why = slots[*reason as usize].showable();
                    let claim = &self.program.texts[*what as usize];
                    return Err(format!(
                        "{}, but {why}. Stopping here rather than guessing.",
                        capitalise(claim)
                    ));
                }

                Instr::Call { dst, which, args } => {
                    let values: Vec<Value> = args.iter().map(|s| slots[*s as usize].clone()).collect();
                    match self.builtin(*which, &values) {
                        Ok(v) => {
                            if let Some(d) = dst {
                                slots[*d as usize] = v.unwrap_or(Value::Nothing);
                            }
                        }
                        Err(reason) => return Err(reason),
                    }
                }

                Instr::TryCall {
                    dst,
                    which,
                    args,
                    reason,
                    fail,
                } => {
                    let values: Vec<Value> = args.iter().map(|s| slots[*s as usize].clone()).collect();
                    match self.builtin(*which, &values) {
                        Ok(v) => {
                            if let Some(d) = dst {
                                slots[*d as usize] = v.unwrap_or(Value::Nothing);
                            }
                        }
                        Err(why) => {
                            slots[*reason as usize] = Value::text(why.clone());
                            match fail {
                                Some(b) => {
                                    block = *b as usize;
                                    pc = 0;
                                }
                                None => return Ok(Outcome::Failed(why)),
                            }
                        }
                    }
                }

                Instr::CallAction {
                    dst,
                    func,
                    args,
                    reason,
                    fail,
                } => {
                    let values: Vec<Value> = args.iter().map(|s| slots[*s as usize].clone()).collect();
                    match self.call(*func, values)? {
                        Outcome::Returned(v) => {
                            if let Some(d) = dst {
                                slots[*d as usize] = v;
                            }
                        }
                        Outcome::Failed(why) => {
                            slots[*reason as usize] = Value::text(why.clone());
                            match fail {
                                Some(b) => {
                                    block = *b as usize;
                                    pc = 0;
                                }
                                None => return Ok(Outcome::Failed(why)),
                            }
                        }
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // The standard library
    // -----------------------------------------------------------------

    fn builtin(&mut self, which: Builtin, args: &[Value]) -> Result<Option<Value>, String> {
        let a0 = || args.first().cloned().unwrap_or(Value::Nothing);
        let a1 = || args.get(1).cloned().unwrap_or(Value::Nothing);
        let a2 = || args.get(2).cloned().unwrap_or(Value::Nothing);

        let value = match which {
            Builtin::Show => {
                let line = a0().showable();
                self.world.show(&line);
                None
            }

            Builtin::AskText => Some(Value::text(self.ask_for(&a0().showable(), None)?)),
            Builtin::AskWhole => {
                let text = self.ask_for(&a0().showable(), Some("a whole number"))?;
                Some(Value::Whole(
                    std_lib::text_number(&text)
                        .map(|v| v.as_whole())
                        .unwrap_or(0),
                ))
            }
            Builtin::AskDecimal => {
                let text = self.ask_for(&a0().showable(), Some("a number"))?;
                Some(Value::Decimal(
                    std_lib::text_number(&text)
                        .map(|v| v.as_decimal())
                        .unwrap_or(0.0),
                ))
            }
            Builtin::AskYesNo => {
                let text = self.ask_for(&a0().showable(), Some("a yes or a no"))?;
                Some(Value::YesNo(matches!(
                    text.trim().to_ascii_lowercase().as_str(),
                    "yes" | "y" | "true" | "ok" | "okay"
                )))
            }

            Builtin::NewList => Some(Value::list(Vec::new())),
            Builtin::NewLookup => Some(Value::lookup()),

            Builtin::ListItem => {
                let items = as_list(&a0())?;
                let position = a1().as_whole();
                let got = std_lib::list_item(&items.borrow(), position)?;
                Some(got)
            }
            Builtin::ListCount => {
                let items = as_list(&a0())?;
                let n = items.borrow().len() as i64;
                Some(Value::Whole(n))
            }
            Builtin::ListFirst => {
                let items = as_list(&a0())?;
                let v = std_lib::list_first(&items.borrow())?;
                Some(v)
            }
            Builtin::ListLast => {
                let items = as_list(&a0())?;
                let v = std_lib::list_last(&items.borrow())?;
                Some(v)
            }
            Builtin::ListAppend => {
                let items = as_list(&a0())?;
                std_lib::list_push(&mut items.borrow_mut(), a1());
                None
            }
            Builtin::ListPutAt => {
                let items = as_list(&a0())?;
                let position = a1().as_whole();
                std_lib::list_put_at(&mut items.borrow_mut(), position, a2())?;
                None
            }
            Builtin::ListRemoveAt => {
                let items = as_list(&a0())?;
                let position = a1().as_whole();
                std_lib::list_remove_at(&mut items.borrow_mut(), position)?;
                None
            }
            Builtin::ListSum => {
                let items = as_list(&a0())?;
                let v = std_lib::list_sum(&items.borrow());
                Some(v)
            }
            Builtin::ListBiggest => {
                let items = as_list(&a0())?;
                let v = std_lib::list_biggest(&items.borrow())?;
                Some(v)
            }
            Builtin::ListSmallest => {
                let items = as_list(&a0())?;
                let v = std_lib::list_smallest(&items.borrow())?;
                Some(v)
            }
            Builtin::ListSorted => {
                let items = as_list(&a0())?;
                let v = std_lib::list_sorted(&items.borrow());
                Some(Value::list(v))
            }
            Builtin::ListReversed => {
                let items = as_list(&a0())?;
                let mut v = items.borrow().clone();
                v.reverse();
                Some(Value::list(v))
            }
            Builtin::ListJoin => {
                let items = as_list(&a0())?;
                let separator = a1().as_text();
                let v = std_lib::list_join(&items.borrow(), &separator);
                Some(Value::text(v))
            }
            Builtin::ListContains => {
                let items = as_list(&a0())?;
                let wanted = a1();
                let found = items.borrow().iter().any(|v| v.same_as(&wanted));
                Some(Value::YesNo(found))
            }
            Builtin::ListPosition => {
                let items = as_list(&a0())?;
                let wanted = a1();
                let v = std_lib::list_position(&items.borrow(), &wanted)?;
                Some(v)
            }

            Builtin::LookupGet => {
                let map = as_lookup(&a0())?;
                let key = a1().as_text();
                let v = std_lib::lookup_get(&map.borrow(), &key)?;
                Some(v)
            }
            Builtin::LookupPut => {
                let map = as_lookup(&a0())?;
                let key = a1().as_text();
                map.borrow_mut().insert((*key).clone(), a2());
                None
            }
            Builtin::LookupForget => {
                let map = as_lookup(&a0())?;
                let key = a1().as_text();
                map.borrow_mut().remove(&*key as &str);
                None
            }
            Builtin::LookupKeys => {
                let map = as_lookup(&a0())?;
                let v = std_lib::lookup_keys(&map.borrow());
                Some(Value::list(v))
            }
            Builtin::LookupHas => {
                let map = as_lookup(&a0())?;
                let key = a1().as_text();
                let has = map.borrow().contains_key(&*key as &str);
                Some(Value::YesNo(has))
            }

            Builtin::TextLength => Some(Value::Whole(std_lib::text_length(&a0().as_text()))),
            Builtin::TextCapitals => Some(Value::text(a0().as_text().to_uppercase())),
            Builtin::TextSmall => Some(Value::text(a0().as_text().to_lowercase())),
            Builtin::TextTrimmed => Some(Value::text(a0().as_text().trim().to_string())),
            Builtin::TextWords => Some(Value::list(std_lib::text_words(&a0().as_text()))),
            Builtin::TextSplit => Some(Value::list(std_lib::text_split(
                &a0().as_text(),
                &a1().as_text(),
            ))),
            Builtin::TextNumber => Some(std_lib::text_number(&a0().as_text())?),
            Builtin::TextOf => Some(Value::text(a0().showable())),
            Builtin::TextStartsWith => Some(Value::YesNo(
                a0().as_text().starts_with(&*a1().as_text() as &str),
            )),
            Builtin::TextEndsWith => Some(Value::YesNo(
                a0().as_text().ends_with(&*a1().as_text() as &str),
            )),
            Builtin::TextContains => Some(Value::YesNo(
                a0().as_text().contains(&*a1().as_text() as &str),
            )),

            Builtin::RandomRange => {
                let v = self.dice.between(a0().as_whole(), a1().as_whole());
                Some(Value::Whole(v))
            }
            Builtin::Rounded => Some(std_lib::rounded(&a0())),
            Builtin::Absolute => Some(std_lib::absolute(&a0())),
            Builtin::SquareRoot => Some(std_lib::square_root(&a0())?),
            Builtin::DividesEvenly => Some(std_lib::divides_evenly(&a0(), &a1())),
            Builtin::DivideNumbers => Some(std_lib::divide(&a0(), &a1())?),

            Builtin::TextSlice => Some(Value::text(std_lib::text_slice(
                &a0().as_text(),
                a1().as_whole(),
                a2().as_whole(),
            ))),
            Builtin::TextReplace => Some(Value::text(std_lib::text_replace(
                &a0().as_text(),
                &a1().as_text(),
                &a2().as_text(),
            ))),
            Builtin::TextLetter => Some(std_lib::text_letter(&a0().as_text(), a1().as_whole())?),
            Builtin::TextLetters => Some(Value::list(std_lib::text_letters(&a0().as_text()))),
            Builtin::TextRepeated => Some(Value::text(std_lib::text_repeated(
                &a0().as_text(),
                a1().as_whole(),
            ))),
            Builtin::IsEmpty => Some(Value::YesNo(std_lib::is_empty(&a0()))),

            Builtin::Remainder => Some(std_lib::remainder(&a0(), &a1())?),
            Builtin::Smaller => Some(std_lib::smaller(&a0(), &a1())),
            Builtin::Larger => Some(std_lib::larger(&a0(), &a1())),
            Builtin::Power => Some(std_lib::power(&a0(), &a1())),
            Builtin::RoundedDown => Some(std_lib::rounded_down(&a0())),
            Builtin::RoundedUp => Some(std_lib::rounded_up(&a0())),

            Builtin::ListRest => {
                let items = as_list(&a0())?;
                let v = std_lib::list_rest(&items.borrow());
                Some(Value::list(v))
            }
            Builtin::ListFirstFew => {
                let items = as_list(&a0())?;
                let v = std_lib::list_first_few(&items.borrow(), a1().as_whole());
                Some(Value::list(v))
            }
            Builtin::ListAverage => {
                let items = as_list(&a0())?;
                let v = std_lib::list_average(&items.borrow())?;
                Some(v)
            }
            Builtin::ListCountIn => {
                let items = as_list(&a0())?;
                let wanted = a1();
                let n = std_lib::list_count_in(&items.borrow(), &wanted);
                Some(Value::Whole(n))
            }
            Builtin::LookupCount => {
                let map = as_lookup(&a0())?;
                let n = map.borrow().len() as i64;
                Some(Value::Whole(n))
            }

            Builtin::WaitFor => {
                std_lib::wait_for(a0().as_decimal());
                None
            }
            Builtin::FileAppend => {
                std_lib::file_append(&a1().as_text(), &a0().as_text())?;
                None
            }

            Builtin::FileContents => Some(std_lib::file_contents(&a0().as_text())?),
            Builtin::FileWrite => {
                std_lib::file_write(&a1().as_text(), &a0().as_text())?;
                None
            }
            Builtin::FileExists => Some(Value::YesNo(std_lib::file_exists(&a0().as_text()))),
            Builtin::TimeNow => Some(Value::Whole(std_lib::time_now())),
        };
        Ok(value)
    }

    /// Ask, and keep asking gently if the reply is not the kind of thing that was wanted.
    fn ask_for(&mut self, prompt: &str, wanted: Option<&str>) -> Result<String, String> {
        loop {
            let reply = match self.world.ask(prompt) {
                Some(r) => r,
                None => {
                    return Err(
                        "There is nothing left to read, and something is still being asked for."
                            .to_string(),
                    )
                }
            };
            match wanted {
                None => return Ok(reply),
                Some(kind) => {
                    let ok = if kind.contains("yes") {
                        true
                    } else {
                        std_lib::text_number(&reply).is_ok()
                    };
                    if ok {
                        return Ok(reply);
                    }
                    self.world.show(&format!(
                        "I was hoping for {kind} there, and \"{}\" is not one. Could you try again?",
                        reply.trim()
                    ));
                }
            }
        }
    }
}

fn as_list(v: &Value) -> Result<std::rc::Rc<std::cell::RefCell<Vec<Value>>>, String> {
    match v {
        Value::List(items) => Ok(items.clone()),
        other => Err(format!(
            "I was given {} where a list was needed",
            other.kind_name()
        )),
    }
}

type LookupRef = std::rc::Rc<std::cell::RefCell<std::collections::BTreeMap<String, Value>>>;

fn as_lookup(v: &Value) -> Result<LookupRef, String> {
    match v {
        Value::Lookup(map) => Ok(map.clone()),
        other => Err(format!(
            "I was given {} where a lookup was needed",
            other.kind_name()
        )),
    }
}

fn compare(a: &Value, b: &Value, op: Compare, kind: CmpKind) -> bool {
    let ordering = match kind {
        CmpKind::Whole => a.as_whole().cmp(&b.as_whole()),
        CmpKind::Decimal => a
            .as_decimal()
            .partial_cmp(&b.as_decimal())
            .unwrap_or(std::cmp::Ordering::Equal),
        CmpKind::Text => a.as_text().cmp(&b.as_text()),
        CmpKind::YesNo => a.as_yes_no().cmp(&b.as_yes_no()),
        CmpKind::Value => {
            return match op {
                Compare::Equal => a.same_as(b),
                Compare::NotEqual => !a.same_as(b),
                _ => false,
            }
        }
    };
    match op {
        Compare::Equal => ordering.is_eq(),
        Compare::NotEqual => !ordering.is_eq(),
        Compare::Over => ordering.is_gt(),
        Compare::Under => ordering.is_lt(),
        Compare::AtLeast => ordering.is_ge(),
        Compare::AtMost => ordering.is_le(),
    }
}

fn too_big() -> String {
    "That number grew larger than I can hold. Whole numbers here go up to about nine million \
     million million."
        .to_string()
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
