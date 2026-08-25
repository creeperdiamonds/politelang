//! Sentence tree in, middle language out.
//!
//! This is the step that makes a large vocabulary cheap (spec 9.1). Every friendly way of saying
//! something collapses here, so backends never meet the vocabulary at all.

use crate::*;
use polite_check::types::TyKind;
use polite_check::{Checked, HowToAdd, NO_SLOT};
use polite_syntax::ast::{self as a, Ast, ExprId, ExprKind, InterpPart, StmtId, StmtKind};
use polite_vocab::{Form, Vocabulary};

pub fn lower(ast: &Ast, ck: &mut Checked, vocab: &Vocabulary) -> Program {
    let mut program = Program {
        funcs: Vec::new(),
        texts: ast.texts.clone(),
        main: ck.main,
    };

    let count = ck.functions.len();
    let names: Vec<String> = ck
        .functions
        .iter()
        .map(|f| match f.name {
            Some(n) => ast.words.text(n).to_string(),
            None => String::new(),
        })
        .collect();
    let risky: Vec<bool> = ck.functions.iter().map(|f| f.risky).collect();

    for index in 0..count {
        let info = ck.functions[index].clone();
        let mut lower = Lower {
            ast,
            ck,
            vocab,
            texts: std::mem::take(&mut program.texts),
            blocks: vec![Block::default()],
            cur: 0,
            next_slot: info.slot_count,
            fail: None,
            reason: 0,
            loops: Vec::new(),
        };
        // One scratch slot for failure reasons that nobody named.
        lower.reason = lower.temp();

        lower.lower_block(info.body);
        if !lower.terminated() {
            lower.emit(Instr::Return { src: None });
        }

        program.texts = std::mem::take(&mut lower.texts);
        let blocks = std::mem::take(&mut lower.blocks);
        let slot_count = lower.next_slot;

        program.funcs.push(Function {
            name: names[index].clone(),
            param_slots: info.params.clone(),
            slot_count,
            blocks,
            entry: 0,
            risky: risky[index],
        });
    }

    program
}

struct Lower<'a, 'b> {
    ast: &'a Ast,
    ck: &'b mut Checked,
    vocab: &'a Vocabulary,
    texts: Vec<String>,
    blocks: Vec<Block>,
    cur: BlockId,
    next_slot: u32,
    /// Where a failure goes right now. `None` means it leaves this action (spec 7.4).
    fail: Option<BlockId>,
    reason: Slot,
    /// (where `stop repeating` goes, where `skip to the next one` goes)
    loops: Vec<(BlockId, BlockId)>,
}

impl Lower<'_, '_> {
    // -----------------------------------------------------------------
    // Building
    // -----------------------------------------------------------------

    fn temp(&mut self) -> Slot {
        let s = self.next_slot;
        self.next_slot += 1;
        s
    }

    fn new_block(&mut self) -> BlockId {
        self.blocks.push(Block::default());
        self.blocks.len() as u32 - 1
    }

    fn switch(&mut self, b: BlockId) {
        self.cur = b;
    }

    fn terminated(&self) -> bool {
        self.blocks[self.cur as usize]
            .instrs
            .last()
            .map(|i| i.is_terminator())
            .unwrap_or(false)
    }

    fn emit(&mut self, instr: Instr) {
        if self.terminated() {
            return; // unreachable; nothing to do
        }
        self.blocks[self.cur as usize].instrs.push(instr);
    }

    fn text_const(&mut self, s: &str) -> u32 {
        if let Some(i) = self.texts.iter().position(|t| t == s) {
            return i as u32;
        }
        self.texts.push(s.to_string());
        self.texts.len() as u32 - 1
    }

    // -----------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------

    fn ty_of(&mut self, e: ExprId) -> Ty {
        let t = self.ck.expr_ty[e as usize];
        self.ty_from(t)
    }

    fn ty_from(&mut self, t: polite_check::types::TyId) -> Ty {
        let r = self.ck.types.resolve(t);
        match self.ck.types.kind(r) {
            TyKind::Whole => Ty::Whole,
            TyKind::Fraction => Ty::Fraction,
            TyKind::Decimal => Ty::Decimal,
            TyKind::Complex => Ty::Complex,
            TyKind::Text => Ty::Text,
            TyKind::YesNo => Ty::YesNo,
            TyKind::List(_) => Ty::List,
            TyKind::Lookup(_) => Ty::Lookup,
            TyKind::Nothing => Ty::Nothing,
            // Nothing pinned this down, and text is what an unsettled value is always used as.
            TyKind::Var(_) => Ty::Text,
        }
    }

    fn stmt_ty(&mut self, id: StmtId) -> Ty {
        let t = self.ck.stmt_ty[id as usize];
        self.ty_from(t)
    }

    /// Make a slot hold a decimal, if it is holding a whole number.
    fn as_decimal(&mut self, slot: Slot, ty: Ty) -> Slot {
        if ty == Ty::Decimal {
            return slot;
        }
        let dst = self.temp();
        self.emit(Instr::WholeToDecimal { dst, src: slot });
        dst
    }

    /// A slot holding this value as text, converting it if it is not text already.
    fn as_text(&mut self, e: ExprId, ty: Ty) -> Slot {
        let s = self.value(e);
        if ty == Ty::Text {
            return s;
        }
        let dst = self.temp();
        self.emit(Instr::Call {
            dst: Some(dst),
            which: Builtin::TextOf,
            args: vec![s],
        });
        dst
    }

    /// Make a slot hold a whole number, rounding if it is holding a decimal.
    fn as_whole(&mut self, slot: Slot, ty: Ty) -> Slot {
        if ty != Ty::Decimal {
            return slot;
        }
        let dst = self.temp();
        self.emit(Instr::Call {
            dst: Some(dst),
            which: Builtin::Rounded,
            args: vec![slot],
        });
        dst
    }

    // -----------------------------------------------------------------
    // Statements
    // -----------------------------------------------------------------

    fn lower_block(&mut self, block: a::BlockId) {
        let ids: Vec<StmtId> = self.ast.block(block).to_vec();
        for id in ids {
            self.lower_stmt(id);
        }
    }

    fn lower_stmt(&mut self, id: StmtId) {
        let node = *self.ast.stmt(id);
        match node.kind {
            StmtKind::Define { .. } => {}
            StmtKind::Courtesy { body } => self.lower_block(body),

            StmtKind::Check { arms } => {
                let arms: Vec<a::CheckArm> = self.ast.arm_slice(arms).to_vec();
                let end = self.new_block();
                for arm in arms {
                    match arm.cond {
                        Some(cond) => {
                            let c = self.value(cond);
                            let then_block = self.new_block();
                            let else_block = self.new_block();
                            self.emit(Instr::Branch {
                                cond: c,
                                then_block,
                                else_block,
                            });
                            self.switch(then_block);
                            self.lower_block(arm.body);
                            self.emit(Instr::Jump { to: end });
                            self.switch(else_block);
                        }
                        None => {
                            self.lower_block(arm.body);
                            self.emit(Instr::Jump { to: end });
                        }
                    }
                }
                self.emit(Instr::Jump { to: end });
                self.switch(end);
            }

            StmtKind::Try {
                body, otherwise, ..
            } => {
                let reason_slot = self.ck.stmt_slot[id as usize];
                let other_block = self.new_block();
                let end = self.new_block();

                let saved_fail = self.fail;
                let saved_reason = self.reason;
                self.fail = Some(other_block);
                if reason_slot != NO_SLOT {
                    self.reason = reason_slot;
                }
                self.lower_block(body);
                self.fail = saved_fail;
                self.reason = saved_reason;
                self.emit(Instr::Jump { to: end });

                self.switch(other_block);
                self.lower_block(otherwise);
                self.emit(Instr::Jump { to: end });
                self.switch(end);
            }

            StmtKind::Call { args, .. } => {
                let args: Vec<ExprId> = self.ast.arg_slice(args).to_vec();
                let func = self.ck.stmt_action[id as usize];
                if func == NO_SLOT {
                    return;
                }
                let arg_slots: Vec<Slot> = args.iter().map(|a| self.value(*a)).collect();
                self.call_action(None, func, arg_slots);
            }

            StmtKind::Form {
                form,
                phrase,
                names,
                args,
                body,
            } => {
                let names: Vec<polite_syntax::Sym> = self.ast.name_slice(names).to_vec();
                let args: Vec<ExprId> = self.ast.arg_slice(args).to_vec();
                self.lower_form(id, form, phrase, &names, &args, body);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_form(
        &mut self,
        id: StmtId,
        form: Form,
        _phrase: u32,
        _names: &[polite_syntax::Sym],
        args: &[ExprId],
        body: Option<a::BlockId>,
    ) {
        let slot = self.ck.stmt_slot[id as usize];
        match form {
            Form::Show => {
                if let Some(v) = args.first() {
                    let s = self.value(*v);
                    self.emit(Instr::Call {
                        dst: None,
                        which: Builtin::Show,
                        args: vec![s],
                    });
                }
            }

            Form::AskAs => {
                let prompt = match args.first() {
                    Some(p) => self.value(*p),
                    None => {
                        let t = self.temp();
                        let text = self.text_const("");
                        self.emit(Instr::ConstText { dst: t, text });
                        t
                    }
                };
                let which = match self.stmt_ty(id) {
                    Ty::Whole => Builtin::AskWhole,
                    Ty::Decimal => Builtin::AskDecimal,
                    Ty::YesNo => Builtin::AskYesNo,
                    _ => Builtin::AskText,
                };
                if slot != NO_SLOT {
                    self.emit(Instr::Call {
                        dst: Some(slot),
                        which,
                        args: vec![prompt],
                    });
                }
            }

            Form::Remember | Form::Assign => {
                if let Some(v) = args.first() {
                    if slot != NO_SLOT {
                        let want = self.stmt_ty(id);
                        self.lower_into_typed(*v, slot, want);
                    }
                }
            }

            Form::AddTo => {
                if let (Some(v), true) = (args.first(), slot != NO_SLOT) {
                    match self.ck.stmt_add[id as usize] {
                        HowToAdd::Number => {
                            let want = self.stmt_ty(id);
                            let vs = self.value_as(*v, want);
                            self.emit(if want == Ty::Decimal {
                                Instr::AddDecimal {
                                    dst: slot,
                                    a: slot,
                                    b: vs,
                                }
                            } else {
                                Instr::AddWhole {
                                    dst: slot,
                                    a: slot,
                                    b: vs,
                                }
                            });
                        }
                        HowToAdd::Text => {
                            let vs = self.value(*v);
                            self.emit(Instr::ConcatText {
                                dst: slot,
                                a: slot,
                                b: vs,
                            });
                        }
                        HowToAdd::List => {
                            let vs = self.value(*v);
                            self.emit(Instr::Call {
                                dst: None,
                                which: Builtin::ListAppend,
                                args: vec![slot, vs],
                            });
                        }
                    }
                }
            }

            Form::TakeFrom | Form::MultiplyBy => {
                if let (Some(v), true) = (args.first(), slot != NO_SLOT) {
                    let want = self.stmt_ty(id);
                    let vs = self.value_as(*v, want);
                    let decimal = want == Ty::Decimal;
                    self.emit(match (form, decimal) {
                        (Form::TakeFrom, false) => Instr::SubWhole {
                            dst: slot,
                            a: slot,
                            b: vs,
                        },
                        (Form::TakeFrom, true) => Instr::SubDecimal {
                            dst: slot,
                            a: slot,
                            b: vs,
                        },
                        (_, false) => Instr::MulWhole {
                            dst: slot,
                            a: slot,
                            b: vs,
                        },
                        (_, true) => Instr::MulDecimal {
                            dst: slot,
                            a: slot,
                            b: vs,
                        },
                    });
                }
            }

            Form::DivideBy => {
                if let (Some(v), true) = (args.first(), slot != NO_SLOT) {
                    let vs = self.value(*v);
                    self.try_call(Some(slot), Builtin::DivideNumbers, vec![slot, vs]);
                }
            }

            Form::GiveBack => {
                let s = args.first().map(|v| self.value(*v));
                self.emit(Instr::Return { src: s });
            }

            Form::StopRepeating => {
                if let Some((brk, _)) = self.loops.last().copied() {
                    self.emit(Instr::Jump { to: brk });
                }
            }
            Form::SkipOne => {
                if let Some((_, cont)) = self.loops.last().copied() {
                    self.emit(Instr::Jump { to: cont });
                }
            }

            Form::LoopCount => self.loop_count(args, body),
            Form::LoopWhile => self.loop_while(args, body, false),
            Form::LoopUntil => self.loop_while(args, body, true),
            Form::LoopForever => self.loop_forever(body),
            Form::LoopEach => self.loop_each(id, args, body),
            Form::LoopRange => self.loop_range(id, args, body),

            Form::PutAt => {
                if let (Some(v), Some(ix), true) = (args.first(), args.get(1), slot != NO_SLOT) {
                    let vs = self.value(*v);
                    let is = self.value_whole(*ix);
                    self.try_call(None, Builtin::ListPutAt, vec![slot, is, vs]);
                }
            }
            Form::RemoveAt => {
                if let (Some(ix), true) = (args.first(), slot != NO_SLOT) {
                    let is = self.value_whole(*ix);
                    self.try_call(None, Builtin::ListRemoveAt, vec![slot, is]);
                }
            }
            Form::PutFor => {
                if let (Some(v), Some(k), true) = (args.first(), args.get(1), slot != NO_SLOT) {
                    let vs = self.value(*v);
                    let ks = self.value(*k);
                    self.emit(Instr::Call {
                        dst: None,
                        which: Builtin::LookupPut,
                        args: vec![slot, ks, vs],
                    });
                }
            }
            Form::ForgetKey => {
                if let (Some(k), true) = (args.first(), slot != NO_SLOT) {
                    let ks = self.value(*k);
                    self.emit(Instr::Call {
                        dst: None,
                        which: Builtin::LookupForget,
                        args: vec![slot, ks],
                    });
                }
            }
            Form::WriteFile | Form::AppendFile => {
                if let (Some(v), Some(p)) = (args.first(), args.get(1)) {
                    let vs = self.value(*v);
                    let ps = self.value(*p);
                    let which = if form == Form::WriteFile {
                        Builtin::FileWrite
                    } else {
                        Builtin::FileAppend
                    };
                    self.try_call(None, which, vec![vs, ps]);
                }
            }

            Form::SaveCanvas | Form::PutInWindow | Form::DotSize => {
                let which = match form {
                    Form::SaveCanvas => Builtin::SaveCanvas,
                    Form::PutInWindow => Builtin::PutInWindow,
                    _ => Builtin::DotSize,
                };
                let slots: Vec<Slot> = args.iter().map(|a| self.value(*a)).collect();
                // Saving and showing might not work out; setting a size cannot.
                if form == Form::DotSize {
                    self.emit(Instr::Call {
                        dst: None,
                        which,
                        args: slots,
                    });
                } else {
                    self.try_call(None, which, slots);
                }
            }

            // Every one of these might not work out: a connection can refuse, drop, or be
            // told no by the other end, and none of that is the program's fault.
            Form::DiscordLogIn
            | Form::DiscordNext
            | Form::DiscordReply
            | Form::DiscordSend
            | Form::DiscordStatus => {
                let which = match form {
                    Form::DiscordLogIn => Builtin::DiscordLogIn,
                    Form::DiscordNext => Builtin::DiscordNext,
                    Form::DiscordReply => Builtin::DiscordReply,
                    Form::DiscordSend => Builtin::DiscordSend,
                    _ => Builtin::DiscordStatus,
                };
                let slots: Vec<Slot> = args.iter().map(|a| self.value(*a)).collect();
                self.try_call(None, which, slots);
            }

            Form::WriteText | Form::LetterSize => {
                let which = if form == Form::WriteText {
                    Builtin::WriteText
                } else {
                    Builtin::LetterSize
                };
                let slots: Vec<Slot> = args.iter().map(|a| self.value(*a)).collect();
                self.emit(Instr::Call {
                    dst: None,
                    which,
                    args: slots,
                });
            }

            Form::OpenCanvas
            | Form::ClearCanvas
            | Form::PaintPoint
            | Form::DrawLine
            | Form::DrawBox
            | Form::FillBox
            | Form::DrawCircle
            | Form::RevealCanvas
            | Form::RevealLetters => {
                let which = match form {
                    Form::OpenCanvas => Builtin::OpenCanvas,
                    Form::ClearCanvas => Builtin::ClearCanvas,
                    Form::PaintPoint => Builtin::PaintPoint,
                    Form::DrawLine => Builtin::DrawLine,
                    Form::DrawBox => Builtin::DrawBox,
                    Form::FillBox => Builtin::FillBox,
                    Form::DrawCircle => Builtin::DrawCircle,
                    Form::RevealCanvas => Builtin::RevealCanvas,
                    _ => Builtin::RevealLetters,
                };
                let slots: Vec<Slot> = args.iter().map(|a| self.value(*a)).collect();
                self.emit(Instr::Call {
                    dst: None,
                    which,
                    args: slots,
                });
            }

            Form::WaitFor => {
                if let Some(v) = args.first() {
                    let s = self.value(*v);
                    self.emit(Instr::Call {
                        dst: None,
                        which: Builtin::WaitFor,
                        args: vec![s],
                    });
                }
            }

            Form::StopEverything => self.emit(Instr::StopEverything),

            // Anything else in statement position was already reported by the checker.
            _ => {}
        }
    }

    // -----------------------------------------------------------------
    // Loops — every phrasing lands on this same shape (spec 9.3)
    // -----------------------------------------------------------------

    fn loop_count(&mut self, args: &[ExprId], body: Option<a::BlockId>) {
        let counter = self.temp();
        self.emit(Instr::ConstWhole {
            dst: counter,
            value: 0,
        });
        let limit = match args.first() {
            Some(c) => self.value_whole(*c),
            None => {
                let t = self.temp();
                self.emit(Instr::ConstWhole { dst: t, value: 0 });
                t
            }
        };

        let head = self.new_block();
        let body_block = self.new_block();
        let step = self.new_block();
        let end = self.new_block();

        self.emit(Instr::Jump { to: head });
        self.switch(head);
        let c = self.temp();
        self.emit(Instr::Cmp {
            dst: c,
            op: Compare::Under,
            kind: CmpKind::Number,
            a: counter,
            b: limit,
        });
        self.emit(Instr::Branch {
            cond: c,
            then_block: body_block,
            else_block: end,
        });

        self.switch(body_block);
        self.loops.push((end, step));
        if let Some(b) = body {
            self.lower_block(b);
        }
        self.loops.pop();
        self.emit(Instr::Jump { to: step });

        self.switch(step);
        let one = self.temp();
        self.emit(Instr::ConstWhole { dst: one, value: 1 });
        self.emit(Instr::AddWhole {
            dst: counter,
            a: counter,
            b: one,
        });
        self.emit(Instr::Jump { to: head });

        self.switch(end);
    }

    fn loop_while(&mut self, args: &[ExprId], body: Option<a::BlockId>, until: bool) {
        let head = self.new_block();
        let body_block = self.new_block();
        let end = self.new_block();

        self.emit(Instr::Jump { to: head });
        self.switch(head);
        let cond = match args.first() {
            Some(c) => self.value(*c),
            None => {
                let t = self.temp();
                self.emit(Instr::ConstYesNo {
                    dst: t,
                    value: !until,
                });
                t
            }
        };
        let (then_block, else_block) = if until {
            (end, body_block)
        } else {
            (body_block, end)
        };
        self.emit(Instr::Branch {
            cond,
            then_block,
            else_block,
        });

        self.switch(body_block);
        self.loops.push((end, head));
        if let Some(b) = body {
            self.lower_block(b);
        }
        self.loops.pop();
        self.emit(Instr::Jump { to: head });

        self.switch(end);
    }

    fn loop_forever(&mut self, body: Option<a::BlockId>) {
        let head = self.new_block();
        let end = self.new_block();
        self.emit(Instr::Jump { to: head });
        self.switch(head);
        self.loops.push((end, head));
        if let Some(b) = body {
            self.lower_block(b);
        }
        self.loops.pop();
        self.emit(Instr::Jump { to: head });
        self.switch(end);
    }

    fn loop_each(&mut self, id: StmtId, args: &[ExprId], body: Option<a::BlockId>) {
        let var = self.ck.stmt_slot[id as usize];
        let list = match args.first() {
            Some(l) => self.value(*l),
            None => return,
        };
        let count = self.temp();
        self.emit(Instr::Call {
            dst: Some(count),
            which: Builtin::ListCount,
            args: vec![list],
        });
        let index = self.temp();
        self.emit(Instr::ConstWhole {
            dst: index,
            value: 0,
        });

        let head = self.new_block();
        let body_block = self.new_block();
        let step = self.new_block();
        let end = self.new_block();

        self.emit(Instr::Jump { to: head });
        self.switch(head);
        let c = self.temp();
        self.emit(Instr::Cmp {
            dst: c,
            op: Compare::Under,
            kind: CmpKind::Number,
            a: index,
            b: count,
        });
        self.emit(Instr::Branch {
            cond: c,
            then_block: body_block,
            else_block: end,
        });

        self.switch(body_block);
        // Positions read from 1, the way people count.
        let one = self.temp();
        self.emit(Instr::ConstWhole { dst: one, value: 1 });
        let pos = self.temp();
        self.emit(Instr::AddWhole {
            dst: pos,
            a: index,
            b: one,
        });
        if var != NO_SLOT {
            // The index is always inside the list here, so this one cannot fail.
            self.emit(Instr::Call {
                dst: Some(var),
                which: Builtin::ListItem,
                args: vec![list, pos],
            });
        }
        self.loops.push((end, step));
        if let Some(b) = body {
            self.lower_block(b);
        }
        self.loops.pop();
        self.emit(Instr::Jump { to: step });

        self.switch(step);
        let one2 = self.temp();
        self.emit(Instr::ConstWhole {
            dst: one2,
            value: 1,
        });
        self.emit(Instr::AddWhole {
            dst: index,
            a: index,
            b: one2,
        });
        self.emit(Instr::Jump { to: head });

        self.switch(end);
    }

    fn loop_range(&mut self, id: StmtId, args: &[ExprId], body: Option<a::BlockId>) {
        let var = self.ck.stmt_slot[id as usize];
        let from = match args.first() {
            Some(f) => self.value_whole(*f),
            None => return,
        };
        let to = match args.get(1) {
            Some(t) => self.value_whole(*t),
            None => return,
        };
        if var != NO_SLOT {
            self.emit(Instr::Move {
                dst: var,
                src: from,
            });
        }

        let head = self.new_block();
        let body_block = self.new_block();
        let step = self.new_block();
        let end = self.new_block();

        self.emit(Instr::Jump { to: head });
        self.switch(head);
        let c = self.temp();
        self.emit(Instr::Cmp {
            dst: c,
            op: Compare::AtMost,
            kind: CmpKind::Number,
            a: var,
            b: to,
        });
        self.emit(Instr::Branch {
            cond: c,
            then_block: body_block,
            else_block: end,
        });

        self.switch(body_block);
        self.loops.push((end, step));
        if let Some(b) = body {
            self.lower_block(b);
        }
        self.loops.pop();
        self.emit(Instr::Jump { to: step });

        self.switch(step);
        let one = self.temp();
        self.emit(Instr::ConstWhole { dst: one, value: 1 });
        self.emit(Instr::AddWhole {
            dst: var,
            a: var,
            b: one,
        });
        self.emit(Instr::Jump { to: head });

        self.switch(end);
    }

    // -----------------------------------------------------------------
    // Calls that might not work out
    // -----------------------------------------------------------------

    fn try_call(&mut self, dst: Option<Slot>, which: Builtin, args: Vec<Slot>) {
        let reason = self.reason;
        let fail = self.fail;
        self.emit(Instr::TryCall {
            dst,
            which,
            args,
            reason,
            fail,
        });
    }

    fn call_action(&mut self, dst: Option<Slot>, func: FuncId, args: Vec<Slot>) {
        let risky = self.ck.functions[func as usize].risky;
        let reason = self.reason;
        let fail = if risky { self.fail } else { None };
        self.emit(Instr::CallAction {
            dst,
            func,
            args,
            reason,
            fail,
        });
    }

    // -----------------------------------------------------------------
    // Expressions
    // -----------------------------------------------------------------

    /// A slot holding this expression's value. Names give back their own slot, with no copy.
    fn value(&mut self, e: ExprId) -> Slot {
        if let ExprKind::Name(_) = self.ast.expr(e).kind {
            let slot = self.ck.expr_slot[e as usize];
            if slot != NO_SLOT {
                return slot;
            }
        }
        let dst = self.temp();
        self.lower_into(e, dst);
        dst
    }

    fn value_as(&mut self, e: ExprId, want: Ty) -> Slot {
        let s = self.value(e);
        let got = self.ty_of(e);
        match (got, want) {
            (Ty::Whole, Ty::Decimal) => self.as_decimal(s, got),
            (Ty::Decimal, Ty::Whole) => self.as_whole(s, got),
            _ => s,
        }
    }

    fn value_whole(&mut self, e: ExprId) -> Slot {
        self.value_as(e, Ty::Whole)
    }

    fn lower_into_typed(&mut self, e: ExprId, dst: Slot, want: Ty) {
        let got = self.ty_of(e);
        if got == want || !matches!(want, Ty::Whole | Ty::Decimal) {
            self.lower_into(e, dst);
            return;
        }
        let s = self.value_as(e, want);
        self.emit(Instr::Move { dst, src: s });
    }

    fn lower_into(&mut self, e: ExprId, dst: Slot) {
        let node = *self.ast.expr(e);
        match node.kind {
            ExprKind::Int(v) => self.emit(Instr::ConstWhole { dst, value: v }),
            ExprKind::Dec(v) => self.emit(Instr::ConstDecimal { dst, value: v }),
            ExprKind::Text(t) => self.emit(Instr::ConstText { dst, text: t }),
            ExprKind::Yes => self.emit(Instr::ConstYesNo { dst, value: true }),
            ExprKind::No => self.emit(Instr::ConstYesNo { dst, value: false }),
            ExprKind::Nothing => self.emit(Instr::ConstNothing { dst }),

            ExprKind::Name(_) => {
                let slot = self.ck.expr_slot[e as usize];
                if slot != NO_SLOT {
                    self.emit(Instr::Move { dst, src: slot });
                } else {
                    self.emit(Instr::ConstNothing { dst });
                }
            }

            ExprKind::Interp(range) => {
                let parts: Vec<InterpPart> = self.ast.interp_slice(range).to_vec();
                let empty = self.text_const("");
                self.emit(Instr::ConstText { dst, text: empty });
                for part in parts {
                    let piece = match part {
                        InterpPart::Text(t) => {
                            let s = self.temp();
                            self.emit(Instr::ConstText { dst: s, text: t });
                            s
                        }
                        InterpPart::Value(v) => {
                            let s = self.value(v);
                            if self.ty_of(v) == Ty::Text {
                                s
                            } else {
                                let t = self.temp();
                                self.emit(Instr::Call {
                                    dst: Some(t),
                                    which: Builtin::TextOf,
                                    args: vec![s],
                                });
                                t
                            }
                        }
                    };
                    self.emit(Instr::ConcatText {
                        dst,
                        a: dst,
                        b: piece,
                    });
                }
            }

            ExprKind::Unary { op, operand } => {
                let s = self.value(operand);
                match op {
                    a::UnOp::Not => self.emit(Instr::Not { dst, src: s }),
                    a::UnOp::Negate => {
                        let t = self.ty_of(operand);
                        self.emit(match t {
                            Ty::Decimal => Instr::NegateDecimal { dst, src: s },
                            Ty::Whole => Instr::NegateWhole { dst, src: s },
                            _ => Instr::NegateNumber { dst, src: s },
                        });
                    }
                }
            }

            ExprKind::Between { value, low, high } => {
                let vt = self.ty_of(value);
                let lt = self.ty_of(low);
                let ht = self.ty_of(high);
                let decimal = vt == Ty::Decimal || lt == Ty::Decimal || ht == Ty::Decimal;
                let want = if decimal { Ty::Decimal } else { Ty::Whole };
                let kind = if decimal {
                    CmpKind::Decimal
                } else {
                    CmpKind::Number
                };
                let v = self.value_as(value, want);
                let l = self.value_as(low, want);
                let h = self.value_as(high, want);
                self.emit(Instr::Cmp {
                    dst,
                    op: Compare::AtLeast,
                    kind,
                    a: v,
                    b: l,
                });
                let upper = self.temp();
                let rest = self.new_block();
                let end = self.new_block();
                self.emit(Instr::Branch {
                    cond: dst,
                    then_block: rest,
                    else_block: end,
                });
                self.switch(rest);
                self.emit(Instr::Cmp {
                    dst: upper,
                    op: Compare::AtMost,
                    kind,
                    a: v,
                    b: h,
                });
                self.emit(Instr::Move {
                    dst,
                    src: upper,
                });
                self.emit(Instr::Jump { to: end });
                self.switch(end);
            }

            ExprKind::Sure { value } => {
                let fail_block = self.new_block();
                let end = self.new_block();
                let saved_fail = self.fail;
                let saved_reason = self.reason;
                let reason = self.temp();
                self.fail = Some(fail_block);
                self.reason = reason;
                self.lower_into(value, dst);
                self.fail = saved_fail;
                self.reason = saved_reason;
                self.emit(Instr::Jump { to: end });

                self.switch(fail_block);
                let what = self.text_const("you said you were sure about this");
                self.emit(Instr::StopBecauseSure { reason, what });
                self.emit(Instr::Return { src: None });

                self.switch(end);
            }

            ExprKind::Binary { op, lhs, rhs } => self.lower_binary(e, op, lhs, rhs, dst),

            ExprKind::Call { args, .. } => {
                let args: Vec<ExprId> = self.ast.arg_slice(args).to_vec();
                let func = self.ck.expr_action[e as usize];
                if func == NO_SLOT {
                    self.emit(Instr::ConstNothing { dst });
                    return;
                }
                let slots: Vec<Slot> = args.iter().map(|a| self.value(*a)).collect();
                self.call_action(Some(dst), func, slots);
            }

            ExprKind::Phrase {
                form,
                phrase,
                args,
            } => {
                let args: Vec<ExprId> = self.ast.arg_slice(args).to_vec();
                self.lower_phrase(form, phrase, &args, dst);
            }
        }
    }

    fn lower_binary(&mut self, e: ExprId, op: a::BinOp, lhs: ExprId, rhs: ExprId, dst: Slot) {
        use a::BinOp as B;

        // Spec 7.2: a fallback, rather than boolean `or`.
        if op == B::Or && self.ck.expr_fallback[e as usize] {
            let fallback = self.new_block();
            let end = self.new_block();
            let saved_fail = self.fail;
            let saved_reason = self.reason;
            let reason = self.temp();
            self.fail = Some(fallback);
            self.reason = reason;
            self.lower_into(lhs, dst);
            self.fail = saved_fail;
            self.reason = saved_reason;
            self.emit(Instr::Jump { to: end });

            self.switch(fallback);
            self.lower_into(rhs, dst);
            self.emit(Instr::Jump { to: end });
            self.switch(end);
            return;
        }

        // Boolean `and` and `or` stop as soon as the answer is settled.
        if op == B::And || op == B::Or {
            self.lower_into(lhs, dst);
            let rest = self.new_block();
            let end = self.new_block();
            let (then_block, else_block) = if op == B::And {
                (rest, end)
            } else {
                (end, rest)
            };
            self.emit(Instr::Branch {
                cond: dst,
                then_block,
                else_block,
            });
            self.switch(rest);
            self.lower_into(rhs, dst);
            self.emit(Instr::Jump { to: end });
            self.switch(end);
            return;
        }

        let lt = self.ty_of(lhs);
        let rt = self.ty_of(rhs);

        match op {
            B::Then => {
                let a = self.as_text(lhs, lt);
                let b = self.as_text(rhs, rt);
                self.emit(Instr::ConcatText { dst, a, b });
            }
            B::Add if lt == Ty::Text || rt == Ty::Text => {
                let a = self.value(lhs);
                let b = self.value(rhs);
                self.emit(Instr::ConcatText { dst, a, b });
            }
            B::Add | B::Sub | B::Mul => {
                let want = wider(lt, rt);
                let a = self.value_as(lhs, want);
                let b = self.value_as(rhs, want);
                self.emit(match (op, want) {
                    (B::Add, Ty::Whole) => Instr::AddWhole { dst, a, b },
                    (B::Add, Ty::Decimal) => Instr::AddDecimal { dst, a, b },
                    (B::Add, _) => Instr::AddNumber { dst, a, b },
                    (B::Sub, Ty::Whole) => Instr::SubWhole { dst, a, b },
                    (B::Sub, Ty::Decimal) => Instr::SubDecimal { dst, a, b },
                    (B::Sub, _) => Instr::SubNumber { dst, a, b },
                    (_, Ty::Whole) => Instr::MulWhole { dst, a, b },
                    (_, Ty::Decimal) => Instr::MulDecimal { dst, a, b },
                    (_, _) => Instr::MulNumber { dst, a, b },
                });
            }
            B::Div => {
                let a = self.value(lhs);
                let b = self.value(rhs);
                self.try_call(Some(dst), Builtin::DivideNumbers, vec![a, b]);
            }
            B::Is | B::IsNot | B::Over | B::Under | B::AtLeast | B::AtMost => {
                let kind = match (lt, rt) {
                    (Ty::Text, _) | (_, Ty::Text) => CmpKind::Text,
                    (Ty::YesNo, _) | (_, Ty::YesNo) => CmpKind::YesNo,
                    (Ty::List, _) | (_, Ty::List) => CmpKind::Value,
                    (Ty::Lookup, _) | (_, Ty::Lookup) => CmpKind::Value,
                    // Fractions and big whole numbers are compared exactly, so only a pair that
                    // is genuinely decimal takes the decimal path.
                    (Ty::Decimal, Ty::Decimal) => CmpKind::Decimal,
                    (Ty::Nothing, Ty::Nothing) => CmpKind::Value,
                    _ => CmpKind::Number,
                };
                let a = self.value(lhs);
                let b = self.value(rhs);
                let cmp = match op {
                    B::Is => Compare::Equal,
                    B::IsNot => Compare::NotEqual,
                    B::Over => Compare::Over,
                    B::Under => Compare::Under,
                    B::AtLeast => Compare::AtLeast,
                    _ => Compare::AtMost,
                };
                self.emit(Instr::Cmp {
                    dst,
                    op: cmp,
                    kind,
                    a,
                    b,
                });
            }
            B::And | B::Or => unreachable!("handled above"),
        }
    }

    fn lower_phrase(&mut self, form: Form, phrase: u32, args: &[ExprId], dst: Slot) {
        // `force not to decode` is a promise about what happens before this point, not an action.
        // By the time anything is lowered the decision has been made, so it is simply its own text.
        if form == Form::NotDecoded {
            if let Some(inner) = args.first() {
                self.lower_into(*inner, dst);
            }
            return;
        }

        // A point is a list of two numbers, built the way any short list is. There is nothing
        // new here for a backend to learn, which is the whole idea.
        if form == Form::APoint {
            self.emit(Instr::Call {
                dst: Some(dst),
                which: Builtin::NewList,
                args: Vec::new(),
            });
            for a in args {
                let v = self.value_whole(*a);
                self.emit(Instr::Call {
                    dst: None,
                    which: Builtin::ListAppend,
                    args: vec![dst, v],
                });
            }
            return;
        }

        // The length of text and the length of a list are the same question, so they are the
        // same word; which one it is was settled by the checker.
        if form == Form::LengthOf {
            let which = match args.first().map(|a| self.ty_of(*a)) {
                Some(Ty::List) => Builtin::ListCount,
                _ => Builtin::TextLength,
            };
            let slots: Vec<Slot> = args.iter().map(|a| self.value(*a)).collect();
            self.emit(Instr::Call {
                dst: Some(dst),
                which,
                args: slots,
            });
            return;
        }

        let which = match builtin_for(form) {
            Some(b) => b,
            None => {
                self.emit(Instr::ConstNothing { dst });
                return;
            }
        };

        // Argument order in the middle language is the order the builtin wants, which is not
        // always the order that reads best in English.
        let slots: Vec<Slot> = match form {
            Form::ItemOf => {
                // item {index} of {list}  ->  ListItem(list, index)
                let index = self.value_whole(args[0]);
                let list = self.value(args[1]);
                vec![list, index]
            }
            Form::PositionOf => {
                // the position of {value} in {list}
                let value = self.value(args[0]);
                let list = self.value(args[1]);
                vec![list, value]
            }
            Form::ValueFor => {
                // the value for {key} in {lookup}
                let key = self.value(args[0]);
                let lookup = self.value(args[1]);
                vec![lookup, key]
            }
            Form::HasKey => {
                // {lookup} knows about {key}
                let lookup = self.value(args[0]);
                let key = self.value(args[1]);
                vec![lookup, key]
            }
            Form::LetterOf => {
                // the letter {index} of {value}
                let index = self.value_whole(args[0]);
                let text = self.value(args[1]);
                vec![text, index]
            }
            Form::FirstFew => {
                // the first {count} items of {list}
                let count = self.value_whole(args[0]);
                let list = self.value(args[1]);
                vec![list, count]
            }
            Form::CountIn => {
                // the count of {value} in {list}
                let value = self.value(args[0]);
                let list = self.value(args[1]);
                vec![list, value]
            }
            _ => args.iter().map(|a| self.value(*a)).collect(),
        };

        if self.vocab.phrase(phrase).risky || which.might_not_work_out() {
            self.try_call(Some(dst), which, slots);
        } else {
            self.emit(Instr::Call {
                dst: Some(dst),
                which,
                args: slots,
            });
        }
    }
}

/// The further out of two kinds of number.
fn wider(a: Ty, b: Ty) -> Ty {
    let rank = |t: Ty| match t {
        Ty::Whole => 0,
        Ty::Fraction => 1,
        Ty::Decimal => 2,
        Ty::Complex => 3,
        _ => 0,
    };
    if rank(a) >= rank(b) {
        if rank(a) == 0 {
            Ty::Whole
        } else {
            a
        }
    } else {
        b
    }
}

/// Which standard-library operation a form means.
///
/// `the point x and y` has none: a point is a list of two numbers, so it is made the way any
/// short list is and there is nothing new for a backend to learn. This is the whole of the V-to-N collapse: many
/// phrasings, one entry each.
fn builtin_for(form: Form) -> Option<Builtin> {
    Some(match form {
        Form::EmptyList => Builtin::NewList,
        Form::EmptyLookup => Builtin::NewLookup,
        Form::ItemOf => Builtin::ListItem,
        Form::CountOf => Builtin::ListCount,
        Form::FirstItem => Builtin::ListFirst,
        Form::LastItem => Builtin::ListLast,
        Form::SumOf => Builtin::ListSum,
        Form::BiggestOf => Builtin::ListBiggest,
        Form::SmallestOf => Builtin::ListSmallest,
        Form::SortedOf => Builtin::ListSorted,
        Form::ReverseOf => Builtin::ListReversed,
        Form::JoinOf => Builtin::ListJoin,
        Form::ContainsItem => Builtin::ListContains,
        Form::PositionOf => Builtin::ListPosition,
        Form::ValueFor => Builtin::LookupGet,
        Form::KeysOf => Builtin::LookupKeys,
        Form::HasKey => Builtin::LookupHas,
        Form::LengthOf => Builtin::TextLength,
        Form::CapitalsOf => Builtin::TextCapitals,
        Form::SmallOf => Builtin::TextSmall,
        Form::TrimmedOf => Builtin::TextTrimmed,
        Form::WordsIn => Builtin::TextWords,
        Form::SplitOf => Builtin::TextSplit,
        Form::NumberIn => Builtin::TextNumber,
        Form::TextOf => Builtin::TextOf,
        Form::StartsWith => Builtin::TextStartsWith,
        Form::EndsWith => Builtin::TextEndsWith,
        Form::ContainsText => Builtin::TextContains,
        Form::RandomRange => Builtin::RandomRange,
        Form::RoundedOf => Builtin::Rounded,
        Form::AbsoluteOf => Builtin::Absolute,
        Form::SquareRootOf => Builtin::SquareRoot,
        Form::DividesEvenly => Builtin::DividesEvenly,
        Form::ContentsOf => Builtin::FileContents,
        Form::FileExists => Builtin::FileExists,
        Form::TimeNow => Builtin::TimeNow,

        Form::SliceOf => Builtin::TextSlice,
        Form::ReplaceIn => Builtin::TextReplace,
        Form::LetterOf => Builtin::TextLetter,
        Form::LettersOf => Builtin::TextLetters,
        Form::RepeatedText => Builtin::TextRepeated,
        Form::IsEmpty => Builtin::IsEmpty,

        Form::RemainderOf => Builtin::Remainder,
        Form::SmallerOf => Builtin::Smaller,
        Form::LargerOf => Builtin::Larger,
        Form::PowerOf => Builtin::Power,
        Form::RoundedDown => Builtin::RoundedDown,
        Form::RoundedUp => Builtin::RoundedUp,

        Form::DiscordSaid => Builtin::DiscordSaid,
        Form::DiscordName => Builtin::DiscordName,
        Form::DiscordIsBot => Builtin::DiscordIsBot,
        Form::DiscordChannel => Builtin::DiscordChannel,
        Form::DiscordServer => Builtin::DiscordServer,
        Form::SecretCalled => Builtin::SecretCalled,
        Form::WrittenWidth => Builtin::WrittenWidth,
        Form::MakeColour => Builtin::MakeColour,
        Form::NamedColour => Builtin::NamedColour,
        Form::CanvasWidth => Builtin::CanvasWidth,
        Form::CanvasHeight => Builtin::CanvasHeight,
        Form::ColourAt => Builtin::ColourAt,

        Form::IsPrime => Builtin::IsPrime,
        Form::PrimeFactors => Builtin::PrimeFactors,
        Form::DivisorsOf => Builtin::Divisors,
        Form::PowerWithin => Builtin::PowerWithin,
        Form::InverseWithin => Builtin::InverseWithin,
        Form::WaysToChoose => Builtin::WaysToChoose,
        Form::WaysToArrange => Builtin::WaysToArrange,

        Form::InBinary => Builtin::InBinary,
        Form::InHexadecimal => Builtin::InHexadecimal,
        Form::InBase => Builtin::InBase,
        Form::ValueOfInBase => Builtin::ValueOfInBase,

        Form::BitwiseAnd => Builtin::BitwiseAnd,
        Form::BitwiseOr => Builtin::BitwiseOr,
        Form::BitwiseExclusiveOr => Builtin::BitwiseExclusiveOr,
        Form::BitwiseNot => Builtin::BitwiseNot,
        Form::ShiftedLeft => Builtin::ShiftedLeft,
        Form::ShiftedRight => Builtin::ShiftedRight,

        Form::ModeOf => Builtin::Mode,
        Form::VarianceOf => Builtin::Variance,
        Form::CorrelationOf => Builtin::Correlation,

        Form::PairwiseSum => Builtin::PairwiseSum,
        Form::PairwiseProduct => Builtin::PairwiseProduct,
        Form::DotProduct => Builtin::DotProduct,
        Form::CrossProduct => Builtin::CrossProduct,
        Form::MagnitudeOf => Builtin::Magnitude,
        Form::ScaledBy => Builtin::ScaledBy,

        Form::MatrixProduct => Builtin::MatrixProduct,
        Form::TransposeOf => Builtin::Transpose,
        Form::DeterminantOf => Builtin::Determinant,
        Form::MatrixInverse => Builtin::MatrixInverse,
        Form::IdentityMatrix => Builtin::IdentityMatrix,

        Form::MakeFraction => Builtin::MakeFraction,
        Form::TopOf => Builtin::FractionTop,
        Form::BottomOf => Builtin::FractionBottom,
        Form::AsFraction => Builtin::AsFraction,
        Form::AsDecimal => Builtin::AsDecimal,
        Form::AsWholeNumber => Builtin::AsWholeNumber,
        Form::WholeNumberIn => Builtin::WholeNumberIn,

        Form::ImaginaryNumber => Builtin::ImaginaryNumber,
        Form::RealPart => Builtin::RealPart,
        Form::ImaginaryPart => Builtin::ImaginaryPart,
        Form::ConjugateOf => Builtin::Conjugate,
        Form::DirectionOf => Builtin::Direction,
        Form::ComplexSquareRoot => Builtin::ComplexSquareRoot,

        Form::NumberPi => Builtin::Pi,
        Form::NumberE => Builtin::EulerE,

        Form::SineOf => Builtin::Sine,
        Form::CosineOf => Builtin::Cosine,
        Form::TangentOf => Builtin::Tangent,
        Form::ArcSineOf => Builtin::ArcSine,
        Form::ArcCosineOf => Builtin::ArcCosine,
        Form::ArcTangentOf => Builtin::ArcTangent,
        Form::AngleOver => Builtin::AngleOver,
        Form::ToDegrees => Builtin::ToDegrees,
        Form::ToRadians => Builtin::ToRadians,

        Form::HyperbolicSine => Builtin::HyperbolicSine,
        Form::HyperbolicCosine => Builtin::HyperbolicCosine,
        Form::HyperbolicTangent => Builtin::HyperbolicTangent,

        Form::NaturalLogarithm => Builtin::NaturalLogarithm,
        Form::CommonLogarithm => Builtin::CommonLogarithm,
        Form::LogarithmInBase => Builtin::LogarithmInBase,
        Form::ExponentialOf => Builtin::Exponential,

        Form::CubeRootOf => Builtin::CubeRoot,
        Form::Squared => Builtin::Squared,
        Form::Cubed => Builtin::Cubed,

        Form::WholePartOf => Builtin::WholePart,
        Form::FractionPartOf => Builtin::FractionPart,
        Form::SignOf => Builtin::Sign,
        Form::RoundedTo => Builtin::RoundedTo,
        Form::KeptBetween => Builtin::KeptBetween,

        Form::GreatestCommonFactor => Builtin::GreatestCommonFactor,
        Form::SmallestCommonMultiple => Builtin::SmallestCommonMultiple,
        Form::FactorialOf => Builtin::Factorial,

        Form::MedianOf => Builtin::Median,
        Form::SpreadOf => Builtin::Spread,
        Form::AsPercentageOf => Builtin::AsPercentageOf,
        Form::PercentOf => Builtin::PercentOf,

        Form::RestOf => Builtin::ListRest,
        Form::FirstFew => Builtin::ListFirstFew,
        Form::AverageOf => Builtin::ListAverage,
        Form::CountIn => Builtin::ListCountIn,
        Form::LookupCount => Builtin::LookupCount,

        _ => return None,
    })
}
