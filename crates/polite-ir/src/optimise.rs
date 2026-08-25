//! Small, cheap passes over the middle language.
//!
//! Spec 10.3: because these run on PoliteIR, every future backend inherits every one of them for
//! free. That is the V + N argument showing up again — write the pass once, and native, JVM and
//! WebAssembly all get it without a line of their own.

use crate::*;
use std::collections::HashMap;

pub fn run(program: &mut Program) {
    for f in &mut program.funcs {
        fold_constants(f);
        drop_unreachable_blocks(f);
    }
}

/// Arithmetic and comparisons between values that are already known.
fn fold_constants(f: &mut Function) {
    for block in &mut f.blocks {
        let mut known_whole: HashMap<Slot, i64> = HashMap::new();
        let mut known_yes_no: HashMap<Slot, bool> = HashMap::new();
        let mut out: Vec<Instr> = Vec::with_capacity(block.instrs.len());

        for instr in block.instrs.drain(..) {
            let replacement = match &instr {
                Instr::ConstWhole { dst, value } => {
                    known_whole.insert(*dst, *value);
                    known_yes_no.remove(dst);
                    None
                }
                Instr::ConstYesNo { dst, value } => {
                    known_yes_no.insert(*dst, *value);
                    known_whole.remove(dst);
                    None
                }
                Instr::AddWhole { dst, a, b }
                | Instr::SubWhole { dst, a, b }
                | Instr::MulWhole { dst, a, b } => {
                    match (known_whole.get(a).copied(), known_whole.get(b).copied()) {
                        (Some(x), Some(y)) => {
                            let value = match instr {
                                Instr::AddWhole { .. } => x.checked_add(y),
                                Instr::SubWhole { .. } => x.checked_sub(y),
                                _ => x.checked_mul(y),
                            };
                            match value {
                                Some(v) => {
                                    known_whole.insert(*dst, v);
                                    known_yes_no.remove(dst);
                                    Some(Instr::ConstWhole { dst: *dst, value: v })
                                }
                                // Overflow: leave it alone and let the runtime say so kindly.
                                None => {
                                    known_whole.remove(dst);
                                    None
                                }
                            }
                        }
                        _ => {
                            known_whole.remove(dst);
                            known_yes_no.remove(dst);
                            None
                        }
                    }
                }
                Instr::Cmp {
                    dst,
                    op,
                    kind: CmpKind::Whole,
                    a,
                    b,
                } => match (known_whole.get(a).copied(), known_whole.get(b).copied()) {
                    (Some(x), Some(y)) => {
                        let value = match op {
                            Compare::Equal => x == y,
                            Compare::NotEqual => x != y,
                            Compare::Over => x > y,
                            Compare::Under => x < y,
                            Compare::AtLeast => x >= y,
                            Compare::AtMost => x <= y,
                        };
                        known_yes_no.insert(*dst, value);
                        known_whole.remove(dst);
                        Some(Instr::ConstYesNo { dst: *dst, value })
                    }
                    _ => {
                        known_yes_no.remove(dst);
                        known_whole.remove(dst);
                        None
                    }
                },
                Instr::Not { dst, src } => match known_yes_no.get(src).copied() {
                    Some(v) => {
                        known_yes_no.insert(*dst, !v);
                        Some(Instr::ConstYesNo { dst: *dst, value: !v })
                    }
                    None => {
                        known_yes_no.remove(dst);
                        None
                    }
                },
                Instr::Move { dst, src } => {
                    match (known_whole.get(src).copied(), known_yes_no.get(src).copied()) {
                        (Some(v), _) => {
                            known_whole.insert(*dst, v);
                            known_yes_no.remove(dst);
                        }
                        (_, Some(v)) => {
                            known_yes_no.insert(*dst, v);
                            known_whole.remove(dst);
                        }
                        _ => {
                            known_whole.remove(dst);
                            known_yes_no.remove(dst);
                        }
                    }
                    None
                }
                other => {
                    for dst in written_slot(other) {
                        known_whole.remove(&dst);
                        known_yes_no.remove(&dst);
                    }
                    None
                }
            };
            out.push(replacement.unwrap_or(instr));
        }
        block.instrs = out;
    }
}

fn written_slot(instr: &Instr) -> Vec<Slot> {
    match instr {
        Instr::ConstWhole { dst, .. }
        | Instr::ConstDecimal { dst, .. }
        | Instr::ConstText { dst, .. }
        | Instr::ConstYesNo { dst, .. }
        | Instr::ConstNothing { dst }
        | Instr::Move { dst, .. }
        | Instr::WholeToDecimal { dst, .. }
        | Instr::NegateWhole { dst, .. }
        | Instr::NegateDecimal { dst, .. }
        | Instr::Not { dst, .. } => vec![*dst],
        Instr::AddWhole { dst, .. }
        | Instr::SubWhole { dst, .. }
        | Instr::MulWhole { dst, .. }
        | Instr::AddDecimal { dst, .. }
        | Instr::SubDecimal { dst, .. }
        | Instr::MulDecimal { dst, .. }
        | Instr::ConcatText { dst, .. }
        | Instr::Cmp { dst, .. } => vec![*dst],
        Instr::Call { dst, args, .. } => {
            let mut v: Vec<Slot> = dst.iter().copied().collect();
            // A builtin may change a list or lookup it is handed.
            v.extend(args.iter().copied());
            v
        }
        Instr::TryCall {
            dst, args, reason, ..
        } => {
            let mut v: Vec<Slot> = dst.iter().copied().collect();
            v.push(*reason);
            v.extend(args.iter().copied());
            v
        }
        Instr::CallAction {
            dst, args, reason, ..
        } => {
            let mut v: Vec<Slot> = dst.iter().copied().collect();
            v.push(*reason);
            v.extend(args.iter().copied());
            v
        }
        _ => Vec::new(),
    }
}

/// Blocks nothing can reach are dropped, and the ones that remain are renumbered.
fn drop_unreachable_blocks(f: &mut Function) {
    let mut reached = vec![false; f.blocks.len()];
    let mut stack = vec![f.entry];
    while let Some(b) = stack.pop() {
        if reached[b as usize] {
            continue;
        }
        reached[b as usize] = true;
        for instr in &f.blocks[b as usize].instrs {
            match instr {
                Instr::Jump { to } => stack.push(*to),
                Instr::Branch {
                    then_block,
                    else_block,
                    ..
                } => {
                    stack.push(*then_block);
                    stack.push(*else_block);
                }
                Instr::TryCall { fail: Some(t), .. } | Instr::CallAction { fail: Some(t), .. } => {
                    stack.push(*t)
                }
                _ => {}
            }
        }
    }

    if reached.iter().all(|r| *r) {
        return;
    }

    let mut renumber = vec![0u32; f.blocks.len()];
    let mut next = 0u32;
    for (i, keep) in reached.iter().enumerate() {
        if *keep {
            renumber[i] = next;
            next += 1;
        }
    }

    let mut kept = Vec::with_capacity(next as usize);
    for (i, block) in f.blocks.drain(..).enumerate() {
        if !reached[i] {
            continue;
        }
        let instrs = block
            .instrs
            .into_iter()
            .map(|instr| match instr {
                Instr::Jump { to } => Instr::Jump {
                    to: renumber[to as usize],
                },
                Instr::Branch {
                    cond,
                    then_block,
                    else_block,
                } => Instr::Branch {
                    cond,
                    then_block: renumber[then_block as usize],
                    else_block: renumber[else_block as usize],
                },
                Instr::TryCall {
                    dst,
                    which,
                    args,
                    reason,
                    fail,
                } => Instr::TryCall {
                    dst,
                    which,
                    args,
                    reason,
                    fail: fail.map(|b| renumber[b as usize]),
                },
                Instr::CallAction {
                    dst,
                    func,
                    args,
                    reason,
                    fail,
                } => Instr::CallAction {
                    dst,
                    func,
                    args,
                    reason,
                    fail: fail.map(|b| renumber[b as usize]),
                },
                other => other,
            })
            .collect();
        kept.push(Block { instrs });
    }

    f.entry = renumber[f.entry as usize];
    f.blocks = kept;
}
