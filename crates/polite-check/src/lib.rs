//! Names, types, and things that might not work out.
//!
//! Spec 6.2: you never write a type; the language works them out and checks them before the
//! program runs. Spec 7: if something might not work out, you must say what happens if it does
//! not — and the language never crashes and never quietly invents an answer.

#![forbid(unsafe_code)]

pub mod types;

use polite_diag::{Bag, Diagnostic, Span};
use polite_syntax::ast::*;
use polite_syntax::Sym;
use polite_vocab::{Form, Vocabulary};
use std::collections::{HashMap, HashSet};
use types::{TyId, TyKind, Types};

pub const NO_SLOT: u32 = u32::MAX;

/// Messages are sentences, and sentences start with a capital letter.
fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_alphabetic() => c.to_uppercase().collect::<String>() + chars.as_str(),
        _ => s.to_string(),
    }
}

/// How `add {value} to {name}` should be carried out, decided by the type of the target.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum HowToAdd {
    Number,
    Text,
    List,
}

#[derive(Clone, Debug)]
pub struct FnInfo {
    /// `None` for the main body of the file.
    pub name: Option<Sym>,
    pub action: Option<ActionId>,
    pub body: BlockId,
    pub params: Vec<u32>,
    pub slot_count: u32,
    pub ret: TyId,
    /// Spec 7.4: an action that does something risky without handling it becomes risky itself.
    pub risky: bool,
    pub span: Span,
}

pub struct Checked {
    pub types: Types,
    pub expr_ty: Vec<TyId>,
    /// Slot a `Name` expression refers to; `NO_SLOT` for anything else.
    pub expr_slot: Vec<u32>,
    /// Whether a `Binary::Or` is the fallback of spec 7.2 rather than boolean `or`.
    pub expr_fallback: Vec<bool>,
    /// Action a `Call` expression reaches.
    pub expr_action: Vec<u32>,
    /// Slot a statement writes to.
    pub stmt_slot: Vec<u32>,
    /// Action a `Call` statement reaches.
    pub stmt_action: Vec<u32>,
    /// For `add ... to ...`, which kind of adding was meant.
    pub stmt_add: Vec<HowToAdd>,
    /// Type of the slot a statement writes to, where it writes to one.
    pub stmt_ty: Vec<TyId>,
    pub functions: Vec<FnInfo>,
    /// Index of the file's main body in `functions`.
    pub main: u32,
}

impl Checked {
    pub fn ty(&self, e: ExprId) -> TyId {
        self.expr_ty[e as usize]
    }
    pub fn kind_of(&mut self, e: ExprId) -> TyKind {
        let t = self.expr_ty[e as usize];
        let r = self.types.resolve(t);
        self.types.kind(r)
    }
}

/// Something that might not work out, and where it was written.
#[derive(Copy, Clone, Debug)]
struct Risk {
    span: Span,
    form: Option<Form>,
    action: Option<Sym>,
}

pub fn check(ast: &Ast, vocab: &Vocabulary) -> (Checked, Bag) {
    check_across(ast, vocab, &[])
}

/// Check a file together with everything it borrowed (spec section 5).
///
/// `files` gives the byte range of each gathered file. What one file may see of another is
/// decided here, because this is the only stage that knows where every name came from.
pub fn check_across(ast: &Ast, vocab: &Vocabulary, files: &[(u32, u32)]) -> (Checked, Bag) {
    let mut c = Checker::new(ast, vocab);
    c.modules = files.to_vec();
    c.collect_shared();
    c.prepare_functions();

    // Riskiness travels outward through calls (spec 7.4), so settle it before saying anything.
    // With no actions of your own there is nothing to travel, and one pass is the whole job.
    if !ast.actions.is_empty() {
        let rounds = ast.actions.len() + 2;
        for _ in 0..rounds {
            c.reporting = false;
            if !c.run_pass() {
                break;
            }
        }
    }
    c.reporting = true;
    c.run_pass();
    c.settle_unknowns();

    let bag = std::mem::take(&mut c.bag);
    (c.finish(), bag)
}

struct Checker<'a> {
    ast: &'a Ast,
    vocab: &'a Vocabulary,
    types: Types,
    bag: Bag,
    reporting: bool,

    expr_ty: Vec<TyId>,
    expr_slot: Vec<u32>,
    expr_fallback: Vec<bool>,
    expr_action: Vec<u32>,
    stmt_slot: Vec<u32>,
    stmt_action: Vec<u32>,
    stmt_add: Vec<HowToAdd>,
    stmt_ty: Vec<TyId>,

    functions: Vec<FnInfo>,
    fn_param_tys: Vec<Vec<TyId>>,
    by_action: HashMap<Sym, u32>,
    main: u32,

    // While walking one function.
    current: u32,
    scopes: Vec<HashMap<Sym, u32>>,
    slot_ty: HashMap<(u32, u32), TyId>,
    slot_span: HashMap<(u32, u32), Span>,
    slot_name: HashMap<(u32, u32), Sym>,
    slot_count: u32,
    loop_depth: u32,
    try_depth: u32,

    /// Byte range of each gathered file. Empty or single means there is only one.
    modules: Vec<(u32, u32)>,
    /// Which file the statement being read came from.
    current_module: usize,
    /// Which file each action was written in.
    action_module: Vec<usize>,
    /// Which file each name was introduced in.
    slot_module: HashMap<(u32, u32), usize>,
    /// The names each file offers to the others.
    shared: HashSet<(usize, Sym)>,
}

impl<'a> Checker<'a> {
    fn new(ast: &'a Ast, vocab: &'a Vocabulary) -> Checker<'a> {
        let n = ast.exprs.len();
        let m = ast.stmts.len();
        Checker {
            ast,
            vocab,
            types: Types::new(),
            bag: Bag::new(),
            reporting: false,
            expr_ty: vec![TyId(4); n],
            expr_slot: vec![NO_SLOT; n],
            expr_fallback: vec![false; n],
            expr_action: vec![NO_SLOT; n],
            stmt_slot: vec![NO_SLOT; m],
            stmt_action: vec![NO_SLOT; m],
            stmt_add: vec![HowToAdd::Number; m],
            stmt_ty: vec![TyId(4); m],
            functions: Vec::new(),
            fn_param_tys: Vec::new(),
            by_action: HashMap::new(),
            main: 0,
            current: 0,
            scopes: Vec::new(),
            slot_ty: HashMap::new(),
            slot_span: HashMap::new(),
            slot_name: HashMap::new(),
            slot_count: 0,
            loop_depth: 0,
            try_depth: 0,
            modules: Vec::new(),
            current_module: 0,
            action_module: Vec::new(),
            slot_module: HashMap::new(),
            shared: HashSet::new(),
        }
    }

    /// Which gathered file an offset belongs to.
    fn module_of(&self, offset: u32) -> usize {
        for (i, (start, end)) in self.modules.iter().enumerate() {
            if offset >= *start && offset < *end {
                return i;
            }
        }
        self.modules.len().saturating_sub(1)
    }

    /// Read every share before anything else, so a file may offer a word before defining it and
    /// a borrowing file still sees it.
    fn collect_shared(&mut self) {
        let ids: Vec<StmtId> = self.ast.block(self.ast.top).to_vec();
        for id in ids {
            let node = *self.ast.stmt(id);
            if let StmtKind::Form {
                form: Form::Share,
                names,
                ..
            } = node.kind
            {
                let module = self.module_of(node.span.start);
                for name in self.ast.name_slice(names).to_vec() {
                    self.shared.insert((module, name));
                }
            }
        }
    }

    fn finish(self) -> Checked {
        Checked {
            types: self.types,
            expr_ty: self.expr_ty,
            expr_slot: self.expr_slot,
            expr_fallback: self.expr_fallback,
            expr_action: self.expr_action,
            stmt_slot: self.stmt_slot,
            stmt_action: self.stmt_action,
            stmt_add: self.stmt_add,
            stmt_ty: self.stmt_ty,
            functions: self.functions,
            main: self.main,
        }
    }

    fn say(&mut self, d: Diagnostic) {
        if self.reporting {
            self.bag.push(d);
        }
    }

    fn name_text(&self, s: Sym) -> String {
        self.ast.words.text(s).to_string()
    }

    // -----------------------------------------------------------------
    // Setting up
    // -----------------------------------------------------------------

    fn prepare_functions(&mut self) {
        for (i, action) in self.ast.actions.iter().enumerate() {
            self.functions.push(FnInfo {
                name: Some(action.name),
                action: Some(i as u32),
                body: action.body,
                params: Vec::new(),
                slot_count: 0,
                ret: TyId(4),
                risky: false,
                span: action.span,
            });
            self.by_action.insert(action.name, i as u32);
            let module = self.module_of(action.span.start);
            self.action_module.push(module);
        }
        self.main = self.functions.len() as u32;
        self.functions.push(FnInfo {
            name: None,
            action: None,
            body: self.ast.top,
            params: Vec::new(),
            slot_count: 0,
            ret: TyId(4),
            risky: false,
            span: Span::default(),
        });
    }

    /// One whole pass over every function. Types start fresh each time, because riskiness can
    /// change what `or` means, and that changes what unifies with what.
    ///
    /// Returns whether any action's riskiness changed.
    fn run_pass(&mut self) -> bool {
        self.types = Types::new();
        self.slot_ty.clear();
        self.slot_span.clear();
        self.slot_name.clear();

        self.fn_param_tys.clear();
        for i in 0..self.functions.len() {
            let ret = self.types.fresh();
            self.functions[i].ret = ret;
            let count = match self.functions[i].action {
                Some(a) => self.ast.actions[a as usize].params.len as usize,
                None => 0,
            };
            let tys: Vec<TyId> = (0..count).map(|_| self.types.fresh()).collect();
            self.fn_param_tys.push(tys);
        }

        let mut changed = false;
        for i in 0..self.functions.len() {
            let before = self.functions[i].risky;
            self.run_function(i as u32);
            if self.functions[i].risky != before {
                changed = true;
            }
        }
        changed
    }

    fn run_function(&mut self, index: u32) {
        self.current = index;
        self.scopes.clear();
        self.scopes.push(HashMap::new());
        self.slot_count = 0;
        self.loop_depth = 0;
        self.try_depth = 0;

        let info = self.functions[index as usize].clone();
        self.current_module = self.module_of(info.span.start);
        if let Some(a) = info.action {
            let action = self.ast.actions[a as usize].clone();
            let params: Vec<Sym> = self.ast.name_slice(action.params).to_vec();
            let mut slots = Vec::with_capacity(params.len());
            for (k, p) in params.iter().enumerate() {
                let slot = self.declare(*p, action.span);
                let ty = self.fn_param_tys[index as usize][k];
                self.slot_ty.insert((index, slot), ty);
                slots.push(slot);
            }
            self.functions[index as usize].params = slots;
        }

        self.check_block(info.body);
        self.functions[index as usize].slot_count = self.slot_count;
    }

    // -----------------------------------------------------------------
    // Scopes
    // -----------------------------------------------------------------

    fn declare(&mut self, name: Sym, span: Span) -> u32 {
        let slot = self.slot_count;
        self.slot_count += 1;
        self.scopes.last_mut().unwrap().insert(name, slot);
        self.slot_span.insert((self.current, slot), span);
        self.slot_name.insert((self.current, slot), name);
        let module = self.module_of(span.start);
        self.slot_module.insert((self.current, slot), module);
        slot
    }

    /// Find a name, but only one this file is allowed to see.
    ///
    /// Spec section 5: a file keeps everything to itself unless it shares it, and only actions
    /// can be shared. So a name introduced in another file is simply not here.
    fn look_up(&self, name: Sym) -> Option<u32> {
        for scope in self.scopes.iter().rev() {
            if let Some(s) = scope.get(&name) {
                if self.modules.len() > 1 {
                    let owner = self
                        .slot_module
                        .get(&(self.current, *s))
                        .copied()
                        .unwrap_or(self.current_module);
                    if owner != self.current_module {
                        continue;
                    }
                }
                return Some(*s);
            }
        }
        None
    }

    fn slot_type(&mut self, slot: u32) -> TyId {
        let key = (self.current, slot);
        if let Some(t) = self.slot_ty.get(&key) {
            return *t;
        }
        let t = self.types.fresh();
        self.slot_ty.insert(key, t);
        t
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    // -----------------------------------------------------------------
    // Statements
    // -----------------------------------------------------------------

    fn check_block(&mut self, block: BlockId) {
        let ids: Vec<StmtId> = self.ast.block(block).to_vec();
        for id in ids {
            self.check_stmt(id);
        }
    }

    fn scoped_block(&mut self, block: BlockId) {
        self.push_scope();
        self.check_block(block);
        self.pop_scope();
    }

    fn note_slot_type(&mut self, id: StmtId, slot: u32) {
        let t = self.slot_type(slot);
        self.stmt_ty[id as usize] = t;
    }

    fn check_stmt(&mut self, id: StmtId) {
        let node = *self.ast.stmt(id);
        self.current_module = self.module_of(node.span.start);
        match node.kind {
            StmtKind::Define { .. } => {
                // Its body is walked as its own function.
            }
            StmtKind::Courtesy { body } => self.scoped_block(body),
            StmtKind::Check { arms } => {
                let arms: Vec<CheckArm> = self.ast.arm_slice(arms).to_vec();
                for arm in arms {
                    if let Some(cond) = arm.cond {
                        let (ty, risk) = self.check_expr(cond);
                        self.want_yes_or_no(cond, ty, "a check");
                        self.handle_risk(risk, node.span);
                    }
                    self.scoped_block(arm.body);
                }
            }
            StmtKind::Try {
                body,
                otherwise,
                reason,
            } => {
                self.try_depth += 1;
                self.scoped_block(body);
                self.try_depth -= 1;

                self.push_scope();
                let slot = self.declare(reason, node.span);
                self.stmt_slot[id as usize] = slot;
                    self.note_slot_type(id, slot);
                let text = self.types.text;
                let t = self.slot_type(slot);
                let _ = self.types.unify(t, text);
                self.check_block(otherwise);
                self.pop_scope();
            }
            StmtKind::Call { name, args } => {
                let args: Vec<ExprId> = self.ast.arg_slice(args).to_vec();
                let risk = self.check_call(name, &args, node.span);
                self.stmt_action[id as usize] =
                    self.by_action.get(&name).copied().unwrap_or(NO_SLOT);
                self.handle_risk(risk, node.span);
            }
            StmtKind::Form {
                form,
                phrase,
                names,
                args,
                body,
            } => {
                let names: Vec<Sym> = self.ast.name_slice(names).to_vec();
                let args: Vec<ExprId> = self.ast.arg_slice(args).to_vec();
                self.check_form_stmt(id, node.span, form, phrase, &names, &args, body);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_form_stmt(
        &mut self,
        id: StmtId,
        span: Span,
        form: Form,
        phrase: u32,
        names: &[Sym],
        args: &[ExprId],
        body: Option<BlockId>,
    ) {
        let mut risks: Vec<Option<Risk>> = Vec::new();
        let mut arg_ty: Vec<TyId> = Vec::new();
        for a in args {
            let (t, r) = self.check_expr(*a);
            arg_ty.push(t);
            risks.push(r);
        }
        // The phrase itself may be a risky one (spec 7.5).
        let phrase_risky = self.vocab.phrase(phrase).risky;

        match form {
            Form::Show => {}

            Form::AskAs => {
                if let Some(p) = args.first() {
                    let text = self.types.text;
                    self.want(*p, arg_ty[0], text, "the words to ask with");
                }
                if let Some(n) = names.first() {
                    let slot = self.declare(*n, span);
                    self.stmt_slot[id as usize] = slot;
                    self.note_slot_type(id, slot);
                    // Left free on purpose: what the reply is read as follows from how it is
                    // used later (spec 6.2, "inference reaches through I/O").
                    let _ = self.slot_type(slot);
                }
            }

            Form::Remember => {
                if let (Some(n), Some(v)) = (names.first(), args.first()) {
                    if self.look_up(*n).is_some() && self.reporting {
                        let word = self.name_text(*n);
                        self.say(
                            Diagnostic::notice(
                                span,
                                format!("`{word}` was already remembered, so this starts it over."),
                            )
                            .because(
                                "Remembering introduces a name. To give an existing name a new \
                                 value, change it instead — that way a mistyped name is caught \
                                 rather than quietly becoming a second thing.",
                            )
                            .suggest("If you meant to update it:", format!("change {word} to ...")),
                        );
                    }
                    let vt = arg_ty[0];
                    let slot = self.declare(*n, span);
                    self.stmt_slot[id as usize] = slot;
                    self.note_slot_type(id, slot);
                    let st = self.slot_type(slot);
                    let _ = self.types.unify(st, vt);
                    let _ = v;
                }
            }

            Form::Assign => {
                if let (Some(n), Some(v)) = (names.first(), args.first()) {
                    match self.look_up(*n) {
                        Some(slot) => {
                            self.stmt_slot[id as usize] = slot;
                    self.note_slot_type(id, slot);
                            let st = self.slot_type(slot);
                            self.want(*v, arg_ty[0], st, "this name");
                        }
                        None => self.unknown_name(*n, span, true),
                    }
                }
            }

            Form::AddTo => {
                if let (Some(n), Some(v)) = (names.first(), args.first()) {
                    match self.look_up(*n) {
                        Some(slot) => {
                            self.stmt_slot[id as usize] = slot;
                    self.note_slot_type(id, slot);
                            let st = self.slot_type(slot);
                            let how = self.decide_add(*v, arg_ty[0], st, span, *n);
                            self.stmt_add[id as usize] = how;
                        }
                        None => self.unknown_name(*n, span, true),
                    }
                }
            }

            Form::TakeFrom | Form::MultiplyBy | Form::DivideBy => {
                if let (Some(n), Some(v)) = (names.first(), args.first()) {
                    match self.look_up(*n) {
                        Some(slot) => {
                            self.stmt_slot[id as usize] = slot;
                    self.note_slot_type(id, slot);
                            let st = self.slot_type(slot);
                            self.want_number(*v, arg_ty[0], "this");
                            let number = self.types.whole;
                            if self.types.is_unknown(st) {
                                let _ = self.types.unify(st, number);
                            } else if !self.types.is_number(st) {
                                let got = self.types.describe(st);
                                let word = self.name_text(*n);
                                self.say(
                                    Diagnostic::problem(
                                        span,
                                        format!("`{word}` holds {got}, so this arithmetic will not work."),
                                    )
                                    .because("Counting up and down is for numbers.")
                                    .suggest("Numbers look like this:", "please remember score is 0"),
                                );
                            }
                        }
                        None => self.unknown_name(*n, span, true),
                    }
                }
            }

            Form::GiveBack => {
                if self.functions[self.current as usize].action.is_none() {
                    self.say(
                        Diagnostic::problem(span, "There is nobody here to give this back to.")
                            .because(
                                "Giving back is how an action answers whoever asked it. Out here \
                                 in the main part of the file, nobody has asked for anything.",
                            )
                            .suggest("Perhaps you meant to show it:", "please show ..."),
                    );
                } else if let Some(v) = args.first() {
                    let ret = self.functions[self.current as usize].ret;
                    self.want(*v, arg_ty[0], ret, "what this action gives back");
                }
            }

            Form::StopRepeating | Form::SkipOne => {
                if self.loop_depth == 0 {
                    let what = if form == Form::StopRepeating {
                        "stop repeating"
                    } else {
                        "skip to the next one"
                    };
                    self.say(
                        Diagnostic::problem(span, format!("There is no loop here to `{what}` in."))
                            .because("This only makes sense inside something that repeats.")
                            .suggest(
                                "A loop reads like this:",
                                "please repeat 3 times:\n    show \"hello\"\nthanks",
                            ),
                    );
                }
            }

            Form::LoopCount => {
                if let Some(c) = args.first() {
                    self.want_number(*c, arg_ty[0], "how many times to repeat");
                }
                self.loop_body(body);
            }

            Form::LoopWhile | Form::LoopUntil => {
                if let Some(c) = args.first() {
                    self.want_yes_or_no(*c, arg_ty[0], "a loop");
                }
                self.loop_body(body);
            }

            Form::LoopForever => self.loop_body(body),

            Form::LoopEach => {
                self.push_scope();
                let item = self.types.fresh();
                if let Some(l) = args.first() {
                    let list = self.types.list_of(item);
                    self.want(*l, arg_ty[0], list, "what to go through");
                }
                if let Some(n) = names.first() {
                    let slot = self.declare(*n, span);
                    self.stmt_slot[id as usize] = slot;
                    self.note_slot_type(id, slot);
                    let st = self.slot_type(slot);
                    let _ = self.types.unify(st, item);
                }
                self.loop_depth += 1;
                if let Some(b) = body {
                    self.check_block(b);
                }
                self.loop_depth -= 1;
                self.pop_scope();
            }

            Form::LoopRange => {
                self.push_scope();
                for (k, a) in args.iter().enumerate() {
                    self.want_number(*a, arg_ty[k], "a limit of the count");
                }
                if let Some(n) = names.first() {
                    let slot = self.declare(*n, span);
                    self.stmt_slot[id as usize] = slot;
                    self.note_slot_type(id, slot);
                    let st = self.slot_type(slot);
                    let whole = self.types.whole;
                    let _ = self.types.unify(st, whole);
                }
                self.loop_depth += 1;
                if let Some(b) = body {
                    self.check_block(b);
                }
                self.loop_depth -= 1;
                self.pop_scope();
            }

            Form::PutAt => {
                // put {value} at position {index} in {name}
                if let Some(n) = names.first() {
                    match self.look_up(*n) {
                        Some(slot) => {
                            self.stmt_slot[id as usize] = slot;
                    self.note_slot_type(id, slot);
                            let st = self.slot_type(slot);
                            let item = self.types.fresh();
                            let list = self.types.list_of(item);
                            let _ = self.types.unify(st, list);
                            if let Some(v) = args.first() {
                                self.want(*v, arg_ty[0], item, "the new item");
                            }
                            if let Some(ix) = args.get(1) {
                                self.want_number(*ix, arg_ty[1], "a position");
                            }
                        }
                        None => self.unknown_name(*n, span, true),
                    }
                }
            }

            Form::RemoveAt => {
                if let Some(n) = names.first() {
                    match self.look_up(*n) {
                        Some(slot) => {
                            self.stmt_slot[id as usize] = slot;
                    self.note_slot_type(id, slot);
                            let st = self.slot_type(slot);
                            let item = self.types.fresh();
                            let list = self.types.list_of(item);
                            let _ = self.types.unify(st, list);
                            if let Some(ix) = args.first() {
                                self.want_number(*ix, arg_ty[0], "a position");
                            }
                        }
                        None => self.unknown_name(*n, span, true),
                    }
                }
            }

            Form::PutFor => {
                // put {value} for {key} in {name}
                if let Some(n) = names.first() {
                    match self.look_up(*n) {
                        Some(slot) => {
                            self.stmt_slot[id as usize] = slot;
                    self.note_slot_type(id, slot);
                            let st = self.slot_type(slot);
                            let value = self.types.fresh();
                            let lookup = self.types.lookup_of(value);
                            let _ = self.types.unify(st, lookup);
                            if let Some(v) = args.first() {
                                self.want(*v, arg_ty[0], value, "the value being stored");
                            }
                            if let Some(k) = args.get(1) {
                                let text = self.types.text;
                                self.want(*k, arg_ty[1], text, "a key");
                            }
                        }
                        None => self.unknown_name(*n, span, true),
                    }
                }
            }

            Form::ForgetKey => {
                if let Some(n) = names.first() {
                    match self.look_up(*n) {
                        Some(slot) => {
                            self.stmt_slot[id as usize] = slot;
                    self.note_slot_type(id, slot);
                            let st = self.slot_type(slot);
                            let value = self.types.fresh();
                            let lookup = self.types.lookup_of(value);
                            let _ = self.types.unify(st, lookup);
                            if let Some(k) = args.first() {
                                let text = self.types.text;
                                self.want(*k, arg_ty[0], text, "a key");
                            }
                        }
                        None => self.unknown_name(*n, span, true),
                    }
                }
            }

            Form::WaitFor => {
                if let Some(v) = args.first() {
                    self.want_number(*v, arg_ty[0], "how long to wait");
                }
            }

            Form::StopEverything => {}

            Form::AppendFile | Form::WriteFile => {
                let text = self.types.text;
                for (k, a) in args.iter().enumerate() {
                    self.want(*a, arg_ty[k], text, "text");
                }
            }

            Form::UseModule => {
                // Borrowing is settled before anything is read, by gathering the files together.
                // A use line that survives to here came from text rather than from a file, and
                // there is nothing left for it to do.
            }

            Form::Share => {
                for name in names {
                    if !self.by_action.contains_key(name) {
                        let word = self.name_text(*name);
                        self.say(
                            Diagnostic::problem(
                                span,
                                format!("There is no action called `{word}` here to share."),
                            )
                            .because(
                                "Sharing offers one of your own actions to any file that borrows \
                                 this one, so there has to be one to offer.",
                            )
                            .suggest(
                                "Define it first, and then share it:",
                                format!("please define {word}:\n    show \"hello\"\nthanks\n\nplease share {word}"),
                            ),
                        );
                    }
                }
            }

            other => {
                // Any expression form reaching statement position is a mistake in the table.
                self.say(Diagnostic::problem(
                    span,
                    format!(
                        "`{}` produces a value, so it needs somewhere to go.",
                        other.name()
                    ),
                ));
            }
        }

        // Anything risky in this statement has to be dealt with here.
        for r in risks.into_iter().flatten() {
            self.handle_risk(Some(r), span);
        }
        if phrase_risky && !self.cannot_fail(form, args) {
            self.handle_risk(
                Some(Risk {
                    span,
                    form: Some(form),
                    action: None,
                }),
                span,
            );
        }
    }

    fn loop_body(&mut self, body: Option<BlockId>) {
        self.loop_depth += 1;
        if let Some(b) = body {
            self.scoped_block(b);
        }
        self.loop_depth -= 1;
    }

    fn decide_add(
        &mut self,
        value: ExprId,
        value_ty: TyId,
        target: TyId,
        span: Span,
        name: Sym,
    ) -> HowToAdd {
        let t = self.types.resolve(target);
        match self.types.kind(t) {
            TyKind::Whole | TyKind::Decimal => {
                self.want_number(value, value_ty, "what is being added");
                HowToAdd::Number
            }
            TyKind::Text => {
                let text = self.types.text;
                self.want(value, value_ty, text, "what is being added to the text");
                HowToAdd::Text
            }
            TyKind::List(item) => {
                self.want(value, value_ty, item, "an item for this list");
                HowToAdd::List
            }
            TyKind::Var(_) => {
                // Nothing has said what this holds yet; adding a number is the usual meaning.
                let whole = self.types.whole;
                let _ = self.types.unify(t, whole);
                self.want_number(value, value_ty, "what is being added");
                HowToAdd::Number
            }
            _ => {
                let got = self.types.describe(t);
                let word = self.name_text(name);
                self.say(
                    Diagnostic::problem(span, format!("I cannot add anything to {got}."))
                        .because(format!(
                            "`{word}` holds {got}. Adding works on numbers, on text, and on lists."
                        ))
                        .suggest("Perhaps you meant to show them together:", "please show ..."),
                );
                HowToAdd::Number
            }
        }
    }

    // -----------------------------------------------------------------
    // Expressions
    // -----------------------------------------------------------------

    fn check_expr(&mut self, id: ExprId) -> (TyId, Option<Risk>) {
        let node = *self.ast.expr(id);
        let (ty, risk) = match node.kind {
            ExprKind::Int(_) => (self.types.whole, None),
            ExprKind::Dec(_) => (self.types.decimal, None),
            ExprKind::Text(_) => (self.types.text, None),
            ExprKind::Yes | ExprKind::No => (self.types.yes_no, None),
            ExprKind::Nothing => (self.types.nothing, None),

            ExprKind::Name(sym) => match self.look_up(sym) {
                Some(slot) => {
                    self.expr_slot[id as usize] = slot;
                    (self.slot_type(slot), None)
                }
                None => {
                    self.unknown_name(sym, node.span, false);
                    (self.types.fresh(), None)
                }
            },

            ExprKind::Interp(range) => {
                let parts: Vec<InterpPart> = self.ast.interp_slice(range).to_vec();
                let mut risk = None;
                for p in parts {
                    if let InterpPart::Value(e) = p {
                        let (_, r) = self.check_expr(e);
                        risk = risk.or(r);
                    }
                }
                (self.types.text, risk)
            }

            ExprKind::Unary { op, operand } => {
                let (t, r) = self.check_expr(operand);
                match op {
                    UnOp::Not => {
                        self.want_yes_or_no(operand, t, "`not`");
                        (self.types.yes_no, r)
                    }
                    UnOp::Negate => {
                        self.want_number(operand, t, "a number to make negative");
                        (t, r)
                    }
                }
            }

            ExprKind::Between { value, low, high } => {
                let (tv, r1) = self.check_expr(value);
                let (tl, r2) = self.check_expr(low);
                let (th, r3) = self.check_expr(high);
                self.want_number(value, tv, "what is being compared");
                self.want_number(low, tl, "the lower limit");
                self.want_number(high, th, "the upper limit");
                (self.types.yes_no, r1.or(r2).or(r3))
            }

            ExprKind::Sure { value } => {
                let (t, _) = self.check_expr(value);
                (t, None)
            }

            ExprKind::Binary { op, lhs, rhs } => self.check_binary(id, node.span, op, lhs, rhs),

            ExprKind::Call { name, args } => {
                let args: Vec<ExprId> = self.ast.arg_slice(args).to_vec();
                let risk = self.check_call(name, &args, node.span);
                self.expr_action[id as usize] =
                    self.by_action.get(&name).copied().unwrap_or(NO_SLOT);
                let ty = match self.by_action.get(&name) {
                    Some(f) => self.functions[*f as usize].ret,
                    None => self.types.fresh(),
                };
                (ty, risk)
            }

            ExprKind::Phrase {
                form,
                phrase,
                args,
            } => {
                let args: Vec<ExprId> = self.ast.arg_slice(args).to_vec();
                let mut risk = None;
                let mut tys = Vec::with_capacity(args.len());
                for a in &args {
                    let (t, r) = self.check_expr(*a);
                    tys.push(t);
                    risk = risk.or(r);
                }
                let ty = self.check_phrase_expr(form, &args, &tys);
                if self.vocab.phrase(phrase).risky && !self.cannot_fail(form, &args) {
                    risk = Some(Risk {
                        span: node.span,
                        form: Some(form),
                        action: None,
                    });
                }
                (ty, risk)
            }
        };
        self.expr_ty[id as usize] = ty;
        (ty, risk)
    }

    fn check_binary(
        &mut self,
        id: ExprId,
        span: Span,
        op: BinOp,
        lhs: ExprId,
        rhs: ExprId,
    ) -> (TyId, Option<Risk>) {
        // Spec 3.10: `or` after something that might not work out is the fallback.
        if op == BinOp::Or {
            let (lt, lr) = self.check_expr(lhs);
            if lr.is_some() {
                self.expr_fallback[id as usize] = true;
                let (rt, rr) = self.check_expr(rhs);
                if self.types.unify(lt, rt).is_err() {
                    let a = self.types.describe(lt);
                    let b = self.types.describe(rt);
                    self.say(
                        Diagnostic::problem(
                            span,
                            format!("The fallback here is {b}, but the thing it stands in for is {a}."),
                        )
                        .because(
                            "Whichever way it turns out, the answer has to be the same kind of \
                             thing, or whatever comes next would not know what it is holding.",
                        )
                        .suggest("Make them match:", "... or 0"),
                    );
                }
                return (lt, rr);
            }
            // Nothing here can fail, so this is boolean `or` — unless neither side is a yes or
            // no, in which case somebody has written a fallback for something that always works
            // out. Say so kindly and carry on rather than making a fuss.
            let (rt, rr) = self.check_expr(rhs);
            let yes_no = self.types.yes_no;
            let boolean = !self.types.is_number(lt) || !self.types.is_number(rt);
            if boolean {
                self.expr_fallback[id as usize] = false;
                self.want_yes_or_no(lhs, lt, "`or`");
                self.want_yes_or_no(rhs, rt, "`or`");
                return (self.types.yes_no, rr);
            }
            let _ = yes_no;

            self.expr_fallback[id as usize] = true;
            let what = self.describe_expr(lhs);
            self.say(
                Diagnostic::notice(span, format!("This fallback is never needed, because {what}."))
                    .because(
                        "I only ask you to say what happens if something does not work out when it \
                         genuinely might. This one cannot, so the fallback will never be reached.",
                    )
                    .suggest("You can simply leave it off:", "..."),
            );
            let _ = self.types.unify(lt, rt);
            return (lt, rr);
        }

        let (lt, lr) = self.check_expr(lhs);
        let (rt, rr) = self.check_expr(rhs);
        let risk = lr.or(rr);

        match op {
            // `then` joins whatever it is given, as text. Nothing has to match.
            BinOp::Then => (self.types.text, risk),
            BinOp::And => {
                self.want_yes_or_no(lhs, lt, "`and`");
                self.want_yes_or_no(rhs, rt, "`and`");
                (self.types.yes_no, risk)
            }
            BinOp::Add => {
                let l = self.types.resolve(lt);
                let r = self.types.resolve(rt);
                let lk = self.types.kind(l);
                let rk = self.types.kind(r);
                if matches!(lk, TyKind::Text) || matches!(rk, TyKind::Text) {
                    let text = self.types.text;
                    if self.types.unify(lt, text).is_err() || self.types.unify(rt, text).is_err() {
                        self.mixed_add(span, lt, rt);
                    }
                    (self.types.text, risk)
                } else if self.types.is_number(l) || self.types.is_number(r) {
                    self.want_number(lhs, lt, "what is being added");
                    self.want_number(rhs, rt, "what is being added");
                    (self.types.widen(lt, rt), risk)
                } else {
                    // Neither side has settled: assume numbers, the usual meaning of `+`.
                    let whole = self.types.whole;
                    let _ = self.types.unify(lt, whole);
                    let _ = self.types.unify(rt, whole);
                    (self.types.widen(lt, rt), risk)
                }
            }
            BinOp::Sub | BinOp::Mul => {
                self.want_number(lhs, lt, "a number");
                self.want_number(rhs, rt, "a number");
                (self.types.widen(lt, rt), risk)
            }
            BinOp::Div => {
                self.want_number(lhs, lt, "a number");
                self.want_number(rhs, rt, "a number");
                let could_fail = !matches!(self.plain_number(rhs), Some(v) if v != 0.0);
                let mine = if could_fail {
                    Some(Risk {
                        span,
                        form: Some(Form::DivideBy),
                        action: None,
                    })
                } else {
                    None
                };
                (self.types.decimal, risk.or(mine))
            }
            BinOp::Is | BinOp::IsNot => {
                if self.types.unify(lt, rt).is_err() {
                    let a = self.types.describe(lt);
                    let b = self.types.describe(rt);
                    self.say(
                        Diagnostic::problem(
                            span,
                            format!("I cannot compare {a} with {b}."),
                        )
                        .because(
                            "Two things can only be the same if they are the same kind of thing \
                             to begin with.",
                        )
                        .suggest(
                            "If you meant to compare them as text:",
                            "the text of ... is the text of ...",
                        ),
                    );
                }
                (self.types.yes_no, risk)
            }
            BinOp::Over | BinOp::Under | BinOp::AtLeast | BinOp::AtMost => {
                if !self.types.has_order(lt) || !self.types.has_order(rt) {
                    self.say(
                        Diagnostic::problem(
                            span,
                            "Complex numbers cannot be put in order.",
                        )
                        .because(
                            "A complex number is a place on a plane rather than a point on a line, \
                             and there is no sense in which one place comes before another. They \
                             can still be the same or different.",
                        )
                        .suggest(
                            "Compare how far each one is from zero:",
                            "the size of a is over the size of b",
                        ),
                    );
                    return (self.types.yes_no, risk);
                }
                let l = self.types.resolve(lt);
                if matches!(self.types.kind(l), TyKind::Text) {
                    let text = self.types.text;
                    let _ = self.types.unify(rt, text);
                } else {
                    self.want_number(lhs, lt, "what is being compared");
                    self.want_number(rhs, rt, "what it is compared against");
                }
                (self.types.yes_no, risk)
            }
            BinOp::Or => unreachable!("handled above"),
        }
    }

    fn check_phrase_expr(&mut self, form: Form, args: &[ExprId], tys: &[TyId]) -> TyId {
        let arg = |k: usize| -> Option<(ExprId, TyId)> {
            match (args.get(k), tys.get(k)) {
                (Some(a), Some(t)) => Some((*a, *t)),
                _ => None,
            }
        };
        match form {
            Form::EmptyList => {
                let item = self.types.fresh();
                self.types.list_of(item)
            }
            Form::EmptyLookup => {
                let value = self.types.fresh();
                self.types.lookup_of(value)
            }
            Form::ItemOf => {
                // item {index} of {list}
                let item = self.types.fresh();
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a position");
                }
                if let Some((e, t)) = arg(1) {
                    let list = self.types.list_of(item);
                    self.want(e, t, list, "a list");
                }
                item
            }
            Form::FirstItem | Form::LastItem => {
                let item = self.types.fresh();
                if let Some((e, t)) = arg(0) {
                    let list = self.types.list_of(item);
                    self.want(e, t, list, "a list");
                }
                item
            }
            Form::CountOf => {
                if let Some((e, t)) = arg(0) {
                    let item = self.types.fresh();
                    let list = self.types.list_of(item);
                    self.want(e, t, list, "a list");
                }
                self.types.whole
            }
            Form::SumOf | Form::BiggestOf | Form::SmallestOf => {
                let number = self.types.decimal;
                if let Some((e, t)) = arg(0) {
                    let whole = self.types.whole;
                    let list = self.types.list_of(whole);
                    self.want(e, t, list, "a list of numbers");
                }
                if form == Form::SumOf {
                    self.types.whole
                } else {
                    let _ = number;
                    self.types.whole
                }
            }
            Form::SortedOf | Form::ReverseOf => {
                let item = self.types.fresh();
                let list = self.types.list_of(item);
                if let Some((e, t)) = arg(0) {
                    self.want(e, t, list, "a list");
                }
                list
            }
            Form::JoinOf => {
                let item = self.types.fresh();
                if let Some((e, t)) = arg(0) {
                    let list = self.types.list_of(item);
                    self.want(e, t, list, "a list");
                }
                if let Some((e, t)) = arg(1) {
                    let text = self.types.text;
                    self.want(e, t, text, "what goes between the items");
                }
                self.types.text
            }
            Form::ContainsItem => {
                let item = self.types.fresh();
                if let Some((e, t)) = arg(0) {
                    let list = self.types.list_of(item);
                    self.want(e, t, list, "a list");
                }
                if let Some((e, t)) = arg(1) {
                    self.want(e, t, item, "an item of that list");
                }
                self.types.yes_no
            }
            Form::PositionOf => {
                let item = self.types.fresh();
                if let Some((e, t)) = arg(0) {
                    self.want(e, t, item, "an item");
                }
                if let Some((e, t)) = arg(1) {
                    let list = self.types.list_of(item);
                    self.want(e, t, list, "a list");
                }
                self.types.whole
            }
            Form::ValueFor => {
                let value = self.types.fresh();
                if let Some((e, t)) = arg(0) {
                    let text = self.types.text;
                    self.want(e, t, text, "a key");
                }
                if let Some((e, t)) = arg(1) {
                    let lookup = self.types.lookup_of(value);
                    self.want(e, t, lookup, "a lookup");
                }
                value
            }
            Form::KeysOf => {
                if let Some((e, t)) = arg(0) {
                    let value = self.types.fresh();
                    let lookup = self.types.lookup_of(value);
                    self.want(e, t, lookup, "a lookup");
                }
                let text = self.types.text;
                self.types.list_of(text)
            }
            Form::HasKey => {
                if let Some((e, t)) = arg(0) {
                    let value = self.types.fresh();
                    let lookup = self.types.lookup_of(value);
                    self.want(e, t, lookup, "a lookup");
                }
                if let Some((e, t)) = arg(1) {
                    let text = self.types.text;
                    self.want(e, t, text, "a key");
                }
                self.types.yes_no
            }
            Form::LengthOf => {
                // The length of text is its letters; the length of a list is its items. Both are
                // the same question, so both are the same word.
                if let Some((e, t)) = arg(0) {
                    let r = self.types.resolve(t);
                    if !matches!(self.types.kind(r), TyKind::List(_)) {
                        let text = self.types.text;
                        self.want(e, t, text, "text, or a list");
                    }
                }
                self.types.whole
            }

            Form::SliceOf => {
                if let Some((e, t)) = arg(0) {
                    let text = self.types.text;
                    self.want(e, t, text, "text");
                }
                for k in 1..3 {
                    if let Some((e, t)) = arg(k) {
                        self.want_number(e, t, "a position");
                    }
                }
                self.types.text
            }

            Form::ReplaceIn => {
                let text = self.types.text;
                for k in 0..3 {
                    if let Some((e, t)) = arg(k) {
                        self.want(e, t, text, "text");
                    }
                }
                self.types.text
            }

            Form::LetterOf => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a position");
                }
                if let Some((e, t)) = arg(1) {
                    let text = self.types.text;
                    self.want(e, t, text, "text");
                }
                self.types.text
            }

            Form::LettersOf => {
                if let Some((e, t)) = arg(0) {
                    let text = self.types.text;
                    self.want(e, t, text, "text");
                }
                let text = self.types.text;
                self.types.list_of(text)
            }

            Form::RepeatedText => {
                if let Some((e, t)) = arg(0) {
                    let text = self.types.text;
                    self.want(e, t, text, "text");
                }
                if let Some((e, t)) = arg(1) {
                    self.want_number(e, t, "how many times");
                }
                self.types.text
            }

            Form::IsEmpty => self.types.yes_no,

            Form::RemainderOf => {
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want_number(e, t, "a number");
                    }
                }
                self.types.whole
            }

            Form::SmallerOf | Form::LargerOf => {
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want_number(e, t, "a number");
                    }
                }
                match (arg(0), arg(1)) {
                    (Some((_, a)), Some((_, b))) => self.types.widen(a, b),
                    _ => self.types.whole,
                }
            }

            Form::PowerOf => {
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want_number(e, t, "a number");
                    }
                }
                self.types.decimal
            }

            Form::RoundedDown | Form::RoundedUp => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a number");
                }
                self.types.whole
            }

            Form::RestOf => {
                let item = self.types.fresh();
                let list = self.types.list_of(item);
                if let Some((e, t)) = arg(0) {
                    self.want(e, t, list, "a list");
                }
                list
            }

            Form::FirstFew => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "how many to take");
                }
                let item = self.types.fresh();
                let list = self.types.list_of(item);
                if let Some((e, t)) = arg(1) {
                    self.want(e, t, list, "a list");
                }
                list
            }

            Form::AverageOf => {
                if let Some((e, t)) = arg(0) {
                    let whole = self.types.whole;
                    let list = self.types.list_of(whole);
                    self.want(e, t, list, "a list of numbers");
                }
                self.types.decimal
            }

            Form::CountIn => {
                let item = self.types.fresh();
                if let Some((e, t)) = arg(0) {
                    self.want(e, t, item, "an item");
                }
                if let Some((e, t)) = arg(1) {
                    let list = self.types.list_of(item);
                    self.want(e, t, list, "a list");
                }
                self.types.whole
            }

            Form::IsPrime => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a whole number");
                }
                self.types.yes_no
            }

            Form::PrimeFactors | Form::DivisorsOf => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a whole number");
                }
                let whole = self.types.whole;
                self.types.list_of(whole)
            }

            // Whole numbers in, one whole number out.
            Form::PowerWithin
            | Form::InverseWithin
            | Form::WaysToChoose
            | Form::WaysToArrange
            | Form::BitwiseAnd
            | Form::BitwiseOr
            | Form::BitwiseExclusiveOr
            | Form::BitwiseNot
            | Form::ShiftedLeft
            | Form::ShiftedRight => {
                for k in 0..args.len().min(3) {
                    if let Some((e, t)) = arg(k) {
                        self.want_number(e, t, "a whole number");
                    }
                }
                self.types.whole
            }

            Form::InBinary | Form::InHexadecimal | Form::InBase => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a whole number");
                }
                if let Some((e, t)) = arg(1) {
                    self.want_number(e, t, "which base to use");
                }
                self.types.text
            }

            Form::ValueOfInBase => {
                if let Some((e, t)) = arg(0) {
                    let text = self.types.text;
                    self.want(e, t, text, "text");
                }
                if let Some((e, t)) = arg(1) {
                    self.want_number(e, t, "which base it is written in");
                }
                self.types.whole
            }

            Form::ModeOf => {
                let item = self.types.fresh();
                if let Some((e, t)) = arg(0) {
                    let list = self.types.list_of(item);
                    self.want(e, t, list, "a list");
                }
                item
            }

            Form::VarianceOf | Form::MagnitudeOf | Form::DeterminantOf => {
                if let Some((e, t)) = arg(0) {
                    let whole = self.types.whole;
                    let list = if form == Form::DeterminantOf {
                        let row = self.types.list_of(whole);
                        self.types.list_of(row)
                    } else {
                        self.types.list_of(whole)
                    };
                    self.want(e, t, list, "a list of numbers");
                }
                self.types.decimal
            }

            Form::CorrelationOf => {
                let whole = self.types.whole;
                let list = self.types.list_of(whole);
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want(e, t, list, "a list of numbers");
                    }
                }
                self.types.decimal
            }

            Form::DotProduct => {
                let whole = self.types.whole;
                let list = self.types.list_of(whole);
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want(e, t, list, "a list of numbers");
                    }
                }
                self.types.whole
            }

            Form::PairwiseSum | Form::PairwiseProduct | Form::CrossProduct => {
                let whole = self.types.whole;
                let list = self.types.list_of(whole);
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want(e, t, list, "a list of numbers");
                    }
                }
                list
            }

            Form::ScaledBy => {
                let whole = self.types.whole;
                let list = self.types.list_of(whole);
                if let Some((e, t)) = arg(0) {
                    self.want(e, t, list, "a list of numbers");
                }
                if let Some((e, t)) = arg(1) {
                    self.want_number(e, t, "what to multiply by");
                }
                list
            }

            Form::MatrixProduct | Form::TransposeOf | Form::MatrixInverse => {
                let whole = self.types.whole;
                let row = self.types.list_of(whole);
                let matrix = self.types.list_of(row);
                for k in 0..args.len().min(2) {
                    if let Some((e, t)) = arg(k) {
                        self.want(e, t, matrix, "a matrix, which is a list of rows");
                    }
                }
                matrix
            }

            Form::IdentityMatrix => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "how many rows");
                }
                let whole = self.types.whole;
                let row = self.types.list_of(whole);
                self.types.list_of(row)
            }

            Form::MakeFraction => {
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want_number(e, t, "a whole number");
                    }
                }
                self.types.fraction
            }

            Form::TopOf | Form::BottomOf | Form::AsWholeNumber => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a number");
                }
                self.types.whole
            }

            Form::WholeNumberIn => {
                if let Some((e, t)) = arg(0) {
                    let text = self.types.text;
                    self.want(e, t, text, "text");
                }
                self.types.whole
            }

            Form::AsFraction => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a number");
                }
                self.types.fraction
            }

            Form::AsDecimal | Form::RealPart | Form::ImaginaryPart | Form::DirectionOf => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a number");
                }
                self.types.decimal
            }

            Form::ImaginaryNumber | Form::ConjugateOf | Form::ComplexSquareRoot => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a number");
                }
                self.types.complex
            }

            // Constants.
            Form::NumberPi | Form::NumberE => self.types.decimal,

            // One number in, one decimal out. Most of arithmetic is this shape.
            Form::SineOf
            | Form::CosineOf
            | Form::TangentOf
            | Form::ArcSineOf
            | Form::ArcCosineOf
            | Form::ArcTangentOf
            | Form::ToDegrees
            | Form::ToRadians
            | Form::HyperbolicSine
            | Form::HyperbolicCosine
            | Form::HyperbolicTangent
            | Form::NaturalLogarithm
            | Form::CommonLogarithm
            | Form::ExponentialOf
            | Form::CubeRootOf
            | Form::FractionPartOf => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a number");
                }
                self.types.decimal
            }

            // Two numbers in, one decimal out.
            Form::AngleOver | Form::LogarithmInBase | Form::AsPercentageOf | Form::PercentOf => {
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want_number(e, t, "a number");
                    }
                }
                self.types.decimal
            }

            // These keep the kind of number they were given: a whole number squared is still whole.
            Form::Squared | Form::Cubed => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a number");
                    self.types.widen(t, t)
                } else {
                    self.types.whole
                }
            }

            Form::KeptBetween => {
                for k in 0..3 {
                    if let Some((e, t)) = arg(k) {
                        self.want_number(e, t, "a number");
                    }
                }
                match (arg(0), arg(1)) {
                    (Some((_, a)), Some((_, b))) => self.types.widen(a, b),
                    _ => self.types.whole,
                }
            }

            Form::RoundedTo => {
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want_number(e, t, "a number");
                    }
                }
                self.types.decimal
            }

            // Whole numbers out.
            Form::WholePartOf | Form::SignOf | Form::FactorialOf => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a number");
                }
                self.types.whole
            }

            Form::GreatestCommonFactor | Form::SmallestCommonMultiple => {
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want_number(e, t, "a whole number");
                    }
                }
                self.types.whole
            }

            // A list of numbers in, one decimal out.
            Form::MedianOf | Form::SpreadOf => {
                if let Some((e, t)) = arg(0) {
                    let whole = self.types.whole;
                    let list = self.types.list_of(whole);
                    self.want(e, t, list, "a list of numbers");
                }
                self.types.decimal
            }

            Form::LookupCount => {
                if let Some((e, t)) = arg(0) {
                    let value = self.types.fresh();
                    let lookup = self.types.lookup_of(value);
                    self.want(e, t, lookup, "a lookup");
                }
                self.types.whole
            }
            Form::CapitalsOf | Form::SmallOf | Form::TrimmedOf => {
                if let Some((e, t)) = arg(0) {
                    let text = self.types.text;
                    self.want(e, t, text, "text");
                }
                self.types.text
            }
            Form::WordsIn => {
                if let Some((e, t)) = arg(0) {
                    let text = self.types.text;
                    self.want(e, t, text, "text");
                }
                let text = self.types.text;
                self.types.list_of(text)
            }
            Form::SplitOf => {
                let text = self.types.text;
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want(e, t, text, "text");
                    }
                }
                self.types.list_of(text)
            }
            Form::NumberIn => {
                if let Some((e, t)) = arg(0) {
                    let text = self.types.text;
                    self.want(e, t, text, "text");
                }
                self.types.decimal
            }
            Form::TextOf => self.types.text,
            Form::StartsWith | Form::EndsWith | Form::ContainsText => {
                let text = self.types.text;
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want(e, t, text, "text");
                    }
                }
                self.types.yes_no
            }
            Form::RandomRange => {
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want_number(e, t, "a limit");
                    }
                }
                self.types.whole
            }
            Form::RoundedOf => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a number");
                }
                self.types.whole
            }
            Form::AbsoluteOf => {
                match arg(0) {
                    Some((e, t)) => {
                        self.want_number(e, t, "a number");
                        // How far a complex number is from zero is a plain distance.
                        let r = self.types.resolve(t);
                        if matches!(self.types.kind(r), TyKind::Complex) {
                            self.types.decimal
                        } else {
                            self.types.widen(t, t)
                        }
                    }
                    None => self.types.whole,
                }
            }
            Form::SquareRootOf => {
                if let Some((e, t)) = arg(0) {
                    self.want_number(e, t, "a number");
                }
                self.types.decimal
            }
            Form::DividesEvenly => {
                for k in 0..2 {
                    if let Some((e, t)) = arg(k) {
                        self.want_number(e, t, "a number");
                    }
                }
                self.types.yes_no
            }
            Form::ContentsOf => {
                if let Some((e, t)) = arg(0) {
                    let text = self.types.text;
                    self.want(e, t, text, "where the file is");
                }
                self.types.text
            }
            Form::FileExists => {
                if let Some((e, t)) = arg(0) {
                    let text = self.types.text;
                    self.want(e, t, text, "where the file is");
                }
                self.types.yes_no
            }
            Form::TimeNow => self.types.whole,
            other => {
                let _ = other;
                self.types.fresh()
            }
        }
    }

    fn check_call(&mut self, name: Sym, args: &[ExprId], span: Span) -> Option<Risk> {
        let mut risk = None;
        let mut tys = Vec::with_capacity(args.len());
        for a in args {
            let (t, r) = self.check_expr(*a);
            tys.push(t);
            risk = risk.or(r);
        }

        let index = match self.by_action.get(&name) {
            Some(i) => *i,
            None => {
                let word = self.name_text(name);
                self.say(
                    Diagnostic::problem(span, format!("I do not know an action called `{word}`."))
                        .because("Actions are the words you define yourself.")
                        .suggest(
                            "You can teach me one:",
                            format!("please define {word}:\n    show \"hello\"\nthanks"),
                        ),
                );
                return risk;
            }
        };

        let expected = self.fn_param_tys[index as usize].clone();
        if expected.len() != args.len() {
            let word = self.name_text(name);
            let (wanted, given) = (expected.len(), args.len());
            self.say(
                Diagnostic::problem(
                    span,
                    format!(
                        "`{word}` needs {} {}, and {} {} given here.",
                        wanted,
                        if wanted == 1 { "value" } else { "values" },
                        given,
                        if given == 1 { "was" } else { "were" }
                    ),
                )
                .because("An action asks for exactly what it needs, by name, when it is defined.")
                .suggest(
                    "Give it what it asks for:",
                    format!("please {word} with ..."),
                ),
            );
        }
        for (k, want) in expected.iter().enumerate() {
            if let (Some(a), Some(t)) = (args.get(k), tys.get(k)) {
                self.want(*a, *t, *want, "a value this action needs");
            }
        }

        // Spec section 5: a file offers nothing unless it shares it.
        if self.modules.len() > 1 {
            let owner = self
                .action_module
                .get(index as usize)
                .copied()
                .unwrap_or(self.current_module);
            if owner != self.current_module && !self.shared.contains(&(owner, name)) {
                let word = self.name_text(name);
                self.say(
                    Diagnostic::problem(
                        span,
                        format!("`{word}` is not shared, so it is not mine to hand over."),
                    )
                    .because(
                        "A file keeps everything to itself unless it offers it. That way nothing \
                         of yours is used by another file by accident.",
                    )
                    .suggest(
                        "Add this to the file that defines it:",
                        format!("please share {word}"),
                    ),
                );
            }
        }

        if self.functions[index as usize].risky {
            risk = Some(Risk {
                span,
                form: None,
                action: Some(name),
            });
        }
        risk
    }

    // -----------------------------------------------------------------
    // Wanting things to be a certain type
    // -----------------------------------------------------------------

    fn want(&mut self, e: ExprId, got: TyId, wanted: TyId, what: &str) {
        if self.types.unify(got, wanted).is_ok() {
            return;
        }
        let span = self.ast.expr(e).span;
        let g = self.types.describe(got);
        let w = self.types.describe(wanted);
        self.say(
            Diagnostic::problem(span, format!("I was expecting {w} here, and this is {g}."))
                .because(format!("This place wants {what}."))
                .suggest(
                    "If you want it as text, ask for that:",
                    "the text of ...",
                ),
        );
    }

    fn want_number(&mut self, e: ExprId, got: TyId, what: &str) {
        let r = self.types.resolve(got);
        if self.types.is_number(r) {
            return;
        }
        if self.types.is_unknown(r) {
            let whole = self.types.whole;
            let _ = self.types.unify(r, whole);
            return;
        }
        let span = self.ast.expr(e).span;
        let g = self.types.describe(got);
        self.say(
            Diagnostic::problem(span, format!("I was expecting a number here, and this is {g}."))
                .because(format!("This place wants {what}."))
                .suggest("You can read a number out of text:", "the number in ..."),
        );
    }

    fn want_yes_or_no(&mut self, e: ExprId, got: TyId, what: &str) {
        let yes_no = self.types.yes_no;
        if self.types.unify(got, yes_no).is_ok() {
            return;
        }
        let span = self.ast.expr(e).span;
        let g = self.types.describe(got);
        self.say(
            Diagnostic::problem(
                span,
                format!("I was expecting a yes or no here, and this is {g}."),
            )
            .because(format!(
                "{} needs something that is settled one way or the other.",
                capitalise(what)
            ))
            .suggest("A comparison gives a yes or no:", "score is over 10"),
        );
    }

    fn mixed_add(&mut self, span: Span, lt: TyId, rt: TyId) {
        let a = self.types.describe(lt);
        let b = self.types.describe(rt);
        self.say(
            Diagnostic::problem(span, format!("I cannot add {b} to {a}."))
                .because(
                    "Adding joins two numbers, or joins two pieces of text. It cannot mix the two, \
                     because there is no sensible answer.",
                )
                .suggest(
                    "To show them together, put the value inside the text:",
                    "please show \"the score is {score}\"",
                ),
        );
    }

    fn unknown_name(&mut self, name: Sym, span: Span, being_changed: bool) {
        let word = self.name_text(name);
        let nearest = self.nearest_local(name);
        let mut d = Diagnostic::problem(span, format!("I do not know a name called `{word}`."));
        match nearest {
            Some(near) => {
                d = d
                    .because(format!(
                        "Nothing here has been remembered under that name. There is a `{near}` \
                         though, which is very close."
                    ))
                    .suggest("Did you mean:", near);
            }
            None => {
                d = d
                    .because(
                        "A name has to be remembered before it can be used, so that a mistyped \
                         name is caught rather than quietly becoming a second thing.",
                    )
                    .suggest(
                        if being_changed {
                            "Remember it first:"
                        } else {
                            "You can remember it like this:"
                        },
                        format!("please remember {word} is 0"),
                    );
            }
        }
        self.say(d);
    }

    fn nearest_local(&self, name: Sym) -> Option<String> {
        let target = self.ast.words.text(name);
        let mut best: Option<(usize, String)> = None;
        for scope in &self.scopes {
            for known in scope.keys() {
                let text = self.ast.words.text(*known);
                let d = polite_vocab::edit_distance(target, text);
                if d <= 2 && best.as_ref().map_or(true, |(bd, _)| d < *bd) {
                    best = Some((d, text.to_string()));
                }
            }
        }
        best.map(|(_, w)| w)
    }

    // -----------------------------------------------------------------
    // Things that might not work out
    // -----------------------------------------------------------------

    /// A short way of naming an expression, for a message about it.
    fn describe_expr(&self, e: ExprId) -> String {
        match self.ast.expr(e).kind {
            ExprKind::Phrase { phrase, .. } => {
                let p = self.vocab.phrase(phrase);
                format!("`{}` always works out here", p.pattern)
            }
            ExprKind::Binary { op: BinOp::Div, .. } => {
                "dividing by that number always works out".to_string()
            }
            _ => "this always works out".to_string(),
        }
    }

    /// The value of an expression, when it is written out in full right there.
    fn plain_number(&self, e: ExprId) -> Option<f64> {
        match self.ast.expr(e).kind {
            ExprKind::Int(v) => Some(v as f64),
            ExprKind::Dec(v) => Some(v),
            ExprKind::Unary {
                op: UnOp::Negate,
                operand,
            } => self.plain_number(operand).map(|v| -v),
            _ => None,
        }
    }

    /// Whether a use of a risky phrase genuinely cannot go wrong.
    ///
    /// `1 over 3` can no more fail than `1 plus 3` can, and being made to say what happens if it
    /// does would be nagging rather than helping. Where the answer is written out in front of the
    /// language, it works it out and stays quiet. Where anything is unknown it asks, as always.
    fn cannot_fail(&self, form: Form, args: &[ExprId]) -> bool {
        let at = |k: usize| args.get(k).and_then(|e| self.plain_number(*e));
        match form {
            Form::MakeFraction | Form::RemainderOf | Form::AsPercentageOf => {
                matches!(at(1), Some(v) if v != 0.0)
            }
            Form::SmallestCommonMultiple => {
                matches!((at(0), at(1)), (Some(a), Some(b)) if a != 0.0 && b != 0.0)
            }
            Form::SquareRootOf => matches!(at(0), Some(v) if v >= 0.0),
            Form::ArcSineOf | Form::ArcCosineOf => {
                matches!(at(0), Some(v) if (-1.0..=1.0).contains(&v))
            }
            Form::NaturalLogarithm | Form::CommonLogarithm => {
                matches!(at(0), Some(v) if v > 0.0)
            }
            Form::LogarithmInBase => {
                matches!((at(0), at(1)), (Some(v), Some(b)) if v > 0.0 && b > 0.0 && b != 1.0)
            }
            Form::FactorialOf => matches!(at(0), Some(v) if (0.0..=20.0).contains(&v)),
            Form::DivideBy => matches!(at(0), Some(v) if v != 0.0),
            _ => false,
        }
    }

    fn handle_risk(&mut self, risk: Option<Risk>, _at: Span) {
        let risk = match risk {
            Some(r) => r,
            None => return,
        };

        // Inside a `try`, that is exactly what the try is for.
        if self.try_depth > 0 {
            return;
        }

        // Spec 7.4: inside an action, the action itself becomes risky and the caller decides.
        if self.functions[self.current as usize].action.is_some() {
            self.functions[self.current as usize].risky = true;
            if self.reporting {
                let name = self.functions[self.current as usize]
                    .name
                    .map(|n| self.name_text(n))
                    .unwrap_or_default();
                let reason = self.describe_risk(&risk);
                self.say(
                    Diagnostic::notice(
                        risk.span,
                        format!("`{name}` might not work out, because {reason}."),
                    )
                    .because(
                        "That is perfectly fine. I am telling you so that whoever uses this action \
                         can plan for it.",
                    ),
                );
            }
            return;
        }

        let reason = self.describe_risk(&risk);
        self.say(
            Diagnostic::problem(risk.span, format!("This might not work out, because {reason}."))
                .because(
                    "I will not crash on you, and I will not quietly invent an answer. So tell me \
                     what should happen if it does not work out, and I will do that instead.",
                )
                .suggest(
                    "Give me something to fall back on, or handle it properly:",
                    "... or 0\n\nplease try to:\n    ...\notherwise if it does not work out:\n    show \"Oh dear: {what went wrong}\"\nthanks",
                ),
        );
    }

    fn describe_risk(&self, risk: &Risk) -> String {
        if let Some(name) = risk.action {
            return format!("`{}` might not", self.ast.words.text(name));
        }
        match risk.form {
            Some(form) => {
                let ways = self.vocab.ways_to_say(form);
                match ways.first() {
                    Some(p) => format!("`{}` might not", p.pattern),
                    None => "it might not".to_string(),
                }
            }
            None => "it might not".to_string(),
        }
    }

    // -----------------------------------------------------------------
    // Anything still unsettled
    // -----------------------------------------------------------------

    /// A type that nothing pinned down becomes text.
    ///
    /// Spec 6.2 has the language speak up when the code does not say enough. In practice a name
    /// only stays unsettled when it is shown, or dropped into some text, or simply passed along —
    /// and text is what all of those want. Refusing to run over it would be a lecture rather than
    /// help, so the friendly default is taken quietly instead.
    fn settle_unknowns(&mut self) {
        let entries: Vec<TyId> = self.slot_ty.values().copied().collect();
        let text = self.types.text;
        for ty in entries {
            if self.types.is_unknown(ty) {
                let _ = self.types.unify(ty, text);
            }
        }
        for i in 0..self.expr_ty.len() {
            let ty = self.expr_ty[i];
            if self.types.is_unknown(ty) {
                let _ = self.types.unify(ty, text);
            }
        }
    }
}
