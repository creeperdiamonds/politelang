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
        canvas: None,
        dot_size: 4,
        letter_size: 1,
        window: None,
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
    /// The surface being drawn on, once a program has asked for one.
    canvas: Option<std_lib::canvas::Canvas>,
    /// How many across each dot is drawn in a saved picture or a window.
    dot_size: usize,
    /// How many dots across one dot of a letter is drawn.
    letter_size: i64,
    /// Whether a window has already been opened, so it is only ever opened once.
    window: Option<String>,
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

                // These take the small fast path when both sides are small, and step out into
                // the wider tower only when they have to. A whole number has no limit.
                Instr::AddWhole { dst, a, b } | Instr::AddNumber { dst, a, b } => {
                    let v = std_lib::numbers::add(&slots[*a as usize], &slots[*b as usize]);
                    set!(dst, v);
                }
                Instr::SubWhole { dst, a, b } | Instr::SubNumber { dst, a, b } => {
                    let v = std_lib::numbers::sub(&slots[*a as usize], &slots[*b as usize]);
                    set!(dst, v);
                }
                Instr::MulWhole { dst, a, b } | Instr::MulNumber { dst, a, b } => {
                    let v = std_lib::numbers::mul(&slots[*a as usize], &slots[*b as usize]);
                    set!(dst, v);
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
                Instr::NegateWhole { dst, src } | Instr::NegateNumber { dst, src } => {
                    let v = std_lib::numbers::negate(&slots[*src as usize]);
                    set!(dst, v);
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
            Builtin::DivideNumbers => Some(std_lib::numbers::divide(&a0(), &a1())?),

            Builtin::IsPrime => Some(Value::YesNo(std_lib::maths::is_prime(a0().as_whole()))),
            Builtin::PrimeFactors => {
                Some(Value::list(std_lib::maths::prime_factors(a0().as_whole())))
            }
            Builtin::Divisors => Some(Value::list(std_lib::maths::divisors(a0().as_whole()))),
            Builtin::PowerWithin => Some(std_lib::maths::power_within(
                a0().as_whole(),
                a1().as_whole(),
                a2().as_whole(),
            )?),
            Builtin::InverseWithin => {
                Some(std_lib::maths::inverse_within(a0().as_whole(), a1().as_whole())?)
            }
            Builtin::WaysToChoose => {
                Some(std_lib::maths::ways_to_choose(a0().as_whole(), a1().as_whole())?)
            }
            Builtin::WaysToArrange => {
                Some(std_lib::maths::ways_to_arrange(a0().as_whole(), a1().as_whole())?)
            }

            Builtin::InBinary => Some(Value::text(std_lib::maths::in_base(a0().as_whole(), 2)?)),
            Builtin::InHexadecimal => {
                Some(Value::text(std_lib::maths::in_base(a0().as_whole(), 16)?))
            }
            Builtin::InBase => Some(Value::text(std_lib::maths::in_base(
                a0().as_whole(),
                a1().as_whole(),
            )?)),
            Builtin::ValueOfInBase => Some(std_lib::maths::value_of_in_base(
                &a0().as_text(),
                a1().as_whole(),
            )?),

            Builtin::BitwiseAnd => Some(std_lib::maths::bit_and(a0().as_whole(), a1().as_whole())),
            Builtin::BitwiseOr => Some(std_lib::maths::bit_or(a0().as_whole(), a1().as_whole())),
            Builtin::BitwiseExclusiveOr => Some(std_lib::maths::bit_exclusive_or(
                a0().as_whole(),
                a1().as_whole(),
            )),
            Builtin::BitwiseNot => Some(std_lib::maths::bit_not(a0().as_whole())),
            Builtin::ShiftedLeft => {
                Some(std_lib::maths::shifted_left(a0().as_whole(), a1().as_whole()))
            }
            Builtin::ShiftedRight => {
                Some(std_lib::maths::shifted_right(a0().as_whole(), a1().as_whole()))
            }

            Builtin::Mode => {
                let items = as_list(&a0())?;
                let v = std_lib::maths::mode(&items.borrow())?;
                Some(v)
            }
            Builtin::Variance => {
                let items = as_list(&a0())?;
                let v = std_lib::maths::variance(&items.borrow())?;
                Some(v)
            }
            Builtin::Correlation => {
                let first = as_list(&a0())?;
                let second = as_list(&a1())?;
                let v = std_lib::maths::correlation(&first.borrow(), &second.borrow())?;
                Some(v)
            }

            Builtin::PairwiseSum => {
                let first = as_list(&a0())?;
                let second = as_list(&a1())?;
                let v = std_lib::vectors::pairwise_sum(&first.borrow(), &second.borrow())?;
                Some(Value::list(v))
            }
            Builtin::PairwiseProduct => {
                let first = as_list(&a0())?;
                let second = as_list(&a1())?;
                let v = std_lib::vectors::pairwise_product(&first.borrow(), &second.borrow())?;
                Some(Value::list(v))
            }
            Builtin::DotProduct => {
                let first = as_list(&a0())?;
                let second = as_list(&a1())?;
                let v = std_lib::vectors::dot_product(&first.borrow(), &second.borrow())?;
                Some(v)
            }
            Builtin::CrossProduct => {
                let first = as_list(&a0())?;
                let second = as_list(&a1())?;
                let v = std_lib::vectors::cross_product(&first.borrow(), &second.borrow())?;
                Some(Value::list(v))
            }
            Builtin::Magnitude => {
                let items = as_list(&a0())?;
                let v = std_lib::vectors::magnitude(&items.borrow());
                Some(v)
            }
            Builtin::ScaledBy => {
                let items = as_list(&a0())?;
                let factor = a1();
                let v = std_lib::vectors::scaled_by(&items.borrow(), &factor);
                Some(Value::list(v))
            }

            Builtin::MatrixProduct => {
                let first = as_list(&a0())?;
                let second = as_list(&a1())?;
                let v = std_lib::vectors::matrix_product(&first.borrow(), &second.borrow())?;
                Some(Value::list(v))
            }
            Builtin::Transpose => {
                let m = as_list(&a0())?;
                let v = std_lib::vectors::transpose(&m.borrow())?;
                Some(Value::list(v))
            }
            Builtin::Determinant => {
                let m = as_list(&a0())?;
                let v = std_lib::vectors::determinant(&m.borrow())?;
                Some(v)
            }
            Builtin::MatrixInverse => {
                let m = as_list(&a0())?;
                let v = std_lib::vectors::matrix_inverse(&m.borrow())?;
                Some(Value::list(v))
            }
            Builtin::IdentityMatrix => {
                Some(Value::list(std_lib::vectors::identity_matrix(a0().as_whole())?))
            }

            // ---- drawing ------------------------------------------------------
            Builtin::OpenCanvas => {
                self.canvas = Some(std_lib::canvas::Canvas::new(a0().as_whole(), a1().as_whole()));
                None
            }
            Builtin::ClearCanvas => {
                self.surface()?.clear(a0().as_whole() as u32);
                None
            }
            Builtin::PaintPoint => {
                let (x, y) = as_point(&a0())?;
                self.surface()?.paint(x, y, a1().as_whole() as u32);
                None
            }
            Builtin::DrawLine | Builtin::DrawBox | Builtin::FillBox => {
                let (x0, y0) = as_point(&a0())?;
                let (x1, y1) = as_point(&a1())?;
                let colour = a2().as_whole() as u32;
                let surface = self.surface()?;
                match which {
                    Builtin::DrawLine => surface.line(x0, y0, x1, y1, colour),
                    Builtin::DrawBox => surface.outline_box(x0, y0, x1, y1, colour),
                    _ => surface.fill_box(x0, y0, x1, y1, colour),
                }
                None
            }
            Builtin::DrawCircle => {
                let (x, y) = as_point(&a0())?;
                let radius = a1().as_whole();
                let colour = a2().as_whole() as u32;
                self.surface()?.circle(x, y, radius, colour);
                None
            }
            Builtin::RevealCanvas => {
                let picture = self.surface()?.to_blocks();
                self.world.show(&picture);
                None
            }
            Builtin::RevealLetters => {
                let picture = self.surface()?.to_letters();
                self.world.show(&picture);
                None
            }
            Builtin::WriteText => {
                let words = a0().as_text();
                let (x, y) = as_point(&a1())?;
                let colour = a2().as_whole() as u32;
                let size = self.letter_size;
                self.surface()?.write(&words, x, y, colour, size);
                None
            }
            Builtin::LetterSize => {
                self.letter_size = a0().as_whole().clamp(1, 16);
                None
            }
            Builtin::WrittenWidth => Some(Value::Whole(std_lib::letters::width_of(
                &a0().as_text(),
                self.letter_size,
            ))),

            Builtin::DotSize => {
                self.dot_size = a0().as_whole().clamp(1, 16) as usize;
                None
            }
            Builtin::SaveCanvas => {
                let path = a0().as_text();
                let size = self.dot_size;
                let surface = self.surface()?;
                std_lib::picture::save_png(surface, &path, size)?;
                None
            }
            Builtin::PutInWindow => {
                self.open_window()?;
                None
            }

            Builtin::MakeColour => Some(Value::Whole(std_lib::canvas::colour_of(
                a0().as_whole(),
                a1().as_whole(),
                a2().as_whole(),
            ) as i64)),
            Builtin::NamedColour => {
                let name = a0().as_text();
                match std_lib::canvas::colour_called(&name) {
                    Some(c) => Some(Value::Whole(c as i64)),
                    None => {
                        return Err(format!(
                            "I do not know a colour called \"{}\"",
                            name.trim()
                        ))
                    }
                }
            }
            Builtin::CanvasWidth => Some(Value::Whole(self.surface()?.wide as i64)),
            Builtin::CanvasHeight => Some(Value::Whole(self.surface()?.tall as i64)),
            Builtin::ColourAt => {
                let (x, y) = as_point(&a0())?;
                let c = self.surface()?.dot_at(x, y);
                Some(Value::Whole(c as i64))
            }

            Builtin::MakeFraction => Some(std_lib::make_fraction(&a0(), &a1())?),
            Builtin::FractionTop => Some(std_lib::fraction_top(&a0())),
            Builtin::FractionBottom => Some(std_lib::fraction_bottom(&a0())),
            Builtin::AsFraction => Some(std_lib::as_fraction_value(&a0())),
            Builtin::AsDecimal => Some(std_lib::as_decimal_value(&a0())),
            Builtin::AsWholeNumber => Some(std_lib::as_whole_number_value(&a0())),
            Builtin::WholeNumberIn => Some(std_lib::whole_number_in(&a0().as_text())?),

            Builtin::ImaginaryNumber => Some(std_lib::imaginary_number(&a0())),
            Builtin::RealPart => Some(std_lib::real_part(&a0())),
            Builtin::ImaginaryPart => Some(std_lib::imaginary_part(&a0())),
            Builtin::Conjugate => Some(std_lib::conjugate(&a0())),
            Builtin::Direction => Some(std_lib::direction(&a0())),
            Builtin::ComplexSquareRoot => Some(std_lib::complex_square_root(&a0())),

            Builtin::Pi => Some(Value::Decimal(std_lib::PI)),
            Builtin::EulerE => Some(Value::Decimal(std_lib::E)),

            Builtin::Sine => Some(std_lib::sine(&a0())),
            Builtin::Cosine => Some(std_lib::cosine(&a0())),
            Builtin::Tangent => Some(std_lib::tangent(&a0())),
            Builtin::ArcSine => Some(std_lib::arc_sine(&a0())?),
            Builtin::ArcCosine => Some(std_lib::arc_cosine(&a0())?),
            Builtin::ArcTangent => Some(std_lib::arc_tangent(&a0())),
            Builtin::AngleOver => Some(std_lib::angle_over(&a0(), &a1())),
            Builtin::ToDegrees => Some(std_lib::to_degrees(&a0())),
            Builtin::ToRadians => Some(std_lib::to_radians(&a0())),

            Builtin::HyperbolicSine => Some(std_lib::hyperbolic_sine(&a0())),
            Builtin::HyperbolicCosine => Some(std_lib::hyperbolic_cosine(&a0())),
            Builtin::HyperbolicTangent => Some(std_lib::hyperbolic_tangent(&a0())),

            Builtin::NaturalLogarithm => Some(std_lib::natural_logarithm(&a0())?),
            Builtin::CommonLogarithm => Some(std_lib::common_logarithm(&a0())?),
            Builtin::LogarithmInBase => Some(std_lib::logarithm_in_base(&a0(), &a1())?),
            Builtin::Exponential => Some(std_lib::exponential(&a0())),

            Builtin::CubeRoot => Some(std_lib::cube_root(&a0())),
            Builtin::Squared => Some(std_lib::squared(&a0())?),
            Builtin::Cubed => Some(std_lib::cubed(&a0())?),

            Builtin::WholePart => Some(std_lib::whole_part(&a0())),
            Builtin::FractionPart => Some(std_lib::fraction_part(&a0())),
            Builtin::Sign => Some(std_lib::sign(&a0())),
            Builtin::RoundedTo => Some(std_lib::rounded_to(&a0(), &a1())),
            Builtin::KeptBetween => Some(std_lib::kept_between(&a0(), &a1(), &a2())),

            Builtin::GreatestCommonFactor => Some(std_lib::greatest_common_factor(&a0(), &a1())),
            Builtin::SmallestCommonMultiple => {
                Some(std_lib::smallest_common_multiple(&a0(), &a1())?)
            }
            Builtin::Factorial => Some(std_lib::factorial(&a0())?),

            Builtin::Median => {
                let items = as_list(&a0())?;
                let v = std_lib::median(&items.borrow())?;
                Some(v)
            }
            Builtin::Spread => {
                let items = as_list(&a0())?;
                let v = std_lib::spread(&items.borrow())?;
                Some(v)
            }
            Builtin::AsPercentageOf => Some(std_lib::as_percentage_of(&a0(), &a1())?),
            Builtin::PercentOf => Some(std_lib::percent_of(&a0(), &a1())),

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

    /// Put the canvas in a window of its own.
    ///
    /// The first time, a small page is written beside the picture and handed to whatever this
    /// machine opens pages with. After that only the picture is written again — the page asks for
    /// it over and over by itself, so the window keeps up without anybody doing anything.
    fn open_window(&mut self) -> Result<(), String> {
        const PICTURE: &str = "polite-window.png";
        const PAGE: &str = "polite-window.html";

        let size = self.dot_size;
        let surface = self.surface()?;
        std_lib::picture::save_png(surface, PICTURE, size)?;

        if self.window.is_some() {
            return Ok(());
        }

        let page = std_lib::picture::viewer_page("Drawn by PoliteLang", PICTURE);
        if std::fs::write(PAGE, page).is_err() {
            return Err(format!("I could not write \"{PAGE}\" to open a window with"));
        }

        let full = std::fs::canonicalize(PAGE)
            .map(|p| p.to_string_lossy().trim_start_matches("\\?\\").to_string())
            .unwrap_or_else(|_| PAGE.to_string());

        // Whatever opens the page must not be handed this program's own screen. If it were, a
        // program run with its output collected would appear to hang until the window was closed,
        // because the collecting would wait on a screen the window was still holding.
        let mut opener = if cfg!(target_os = "windows") {
            let mut c = std::process::Command::new("cmd");
            c.args(["/C", "start", "", &full]);
            c
        } else if cfg!(target_os = "macos") {
            let mut c = std::process::Command::new("open");
            c.arg(&full);
            c
        } else {
            let mut c = std::process::Command::new("xdg-open");
            c.arg(&full);
            c
        };
        let opened = opener
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        match opened {
            Ok(_) => {
                self.window = Some(PICTURE.to_string());
                self.world.show(&format!(
                    "  (a window was opened for the picture; it is also saved as {PICTURE})"
                ));
                Ok(())
            }
            Err(_) => {
                // The picture is written either way, so nothing is lost.
                self.window = Some(PICTURE.to_string());
                self.world.show(&format!(
                    "  (I could not open a window here, so the picture is in {PICTURE} and the                      page to view it is in {PAGE})"
                ));
                Ok(())
            }
        }
    }

    /// The canvas, or a word about why there is not one.
    fn surface(&mut self) -> Result<&mut std_lib::canvas::Canvas, String> {
        match self.canvas {
            Some(ref mut c) => Ok(c),
            None => Err("there is nothing to draw on yet. Open a canvas first, with something                          like: please open a canvas 120 across and 80 down"
                .to_string()),
        }
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

/// A point is a list of two numbers: how far across, then how far down.
fn as_point(v: &Value) -> Answer2 {
    match v {
        Value::List(items) => {
            let items = items.borrow();
            if items.len() != 2 {
                return Err(format!(
                    "a point is two numbers, across and down, and this list holds {}",
                    items.len()
                ));
            }
            Ok((items[0].as_whole(), items[1].as_whole()))
        }
        other => Err(format!(
            "a point is a list of two numbers, and this is {}",
            other.kind_name()
        )),
    }
}

type Answer2 = Result<(i64, i64), String>;

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
        CmpKind::Number => match std_lib::numbers::compare(a, b) {
            Some(o) => o,
            // Complex numbers have no order. Sameness still means something, and the checker has
            // already refused to let anything else be asked of them.
            None => {
                return match op {
                    Compare::Equal => std_lib::numbers::same(a, b),
                    Compare::NotEqual => !std_lib::numbers::same(a, b),
                    _ => false,
                }
            }
        },
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

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
