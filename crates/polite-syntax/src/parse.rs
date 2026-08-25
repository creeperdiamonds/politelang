//! The parser.
//!
//! Hand-written recursive descent (spec 9.5). Statements are driven by the vocabulary table, so
//! adding a word to PoliteLang never means editing this file. Only the four structural rules and
//! the handful of genuinely structural constructs — `check`, `define`, `try`, and a bare courtesy
//! block — are written out here.

use crate::ast::*;
use crate::intern::{Interner, Sym};
use crate::lex::{Tok, Token};
use polite_diag::{Diagnostic, Span};
use polite_vocab::{Form, Kind, Piece, Vocabulary};

macro_rules! keys {
    ($($field:ident => $word:literal),* $(,)?) => {
        #[allow(dead_code)]
        struct Keys { $(pub $field: Sym),* }
        impl Keys {
            fn new(w: &mut Interner) -> Keys { Keys { $($field: w.intern($word)),* } }
        }
    };
}

keys! {
    please => "please",
    kindly => "kindly",
    would => "would",
    you => "you",
    r#if => "if",
    be => "be",
    so => "so",
    kind => "kind",
    as_ => "as",
    to => "to",
    it => "it",
    is => "is",
    not => "not",
    too => "too",
    much => "much",
    trouble => "trouble",
    thank => "thank",
    thanks => "thanks",
    for_ => "for",
    that => "that",
    otherwise => "otherwise",
    define => "define",
    with => "with",
    and => "and",
    or => "or",
    r#try => "try",
    does => "does",
    work => "work",
    out => "out",
    what => "what",
    went => "went",
    wrong => "wrong",
    yes => "yes",
    no => "no",
    nothing => "nothing",
    plus => "plus",
    minus => "minus",
    times => "times",
    divided => "divided",
    by => "by",
    over => "over",
    under => "under",
    at => "at",
    least => "least",
    most => "most",
    between => "between",
    i => "i",
    am => "am",
    sure => "sure",
    then => "then",
    a => "a",
    an => "an",
    the => "the",
    reason_name => "what went wrong",
}

/// Courtesy openers, longest first so the longest one wins.
///
/// Spec 3.2: every one of these means exactly the same thing. They exist so that somebody who
/// finds `please` too pleading has another way to be polite.
const OPENERS: &[&[&str]] = &[
    &["if", "you", "would", "be", "so", "kind", "as", "to"],
    &["if", "it", "is", "not", "too", "much", "trouble"],
    &["if", "you", "would", "be", "so", "kind"],
    &["would", "you", "kindly", "please"],
    &["if", "you", "would", "please"],
    &["would", "you", "please"],
    &["would", "you", "kindly"],
    &["kindly", "please"],
    &["if", "you", "would"],
    &["would", "you"],
    &["please"],
    &["kindly"],
];

/// What a `thank you for ...` may name, per construct.
fn allowed_closer_names(what: Closes) -> &'static [&'static str] {
    match what {
        Closes::Check => &["checking", "asking", "choosing", "that"],
        Closes::Loop => &["repeating", "looping", "the", "loop", "that"],
        Closes::Define => &["defining", "explaining", "that"],
        Closes::Try => &["trying", "that"],
        Closes::Courtesy => &["asking", "helping", "everything", "that", "all", "of"],
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Closes {
    Check,
    Loop,
    Define,
    Try,
    Courtesy,
}

impl Closes {
    fn describe(self) -> &'static str {
        match self {
            Closes::Check => "a check",
            Closes::Loop => "a loop",
            Closes::Define => "an action",
            Closes::Try => "a try",
            Closes::Courtesy => "a courtesy block",
        }
    }
}

/// What stopped a block.
#[derive(Copy, Clone, Debug)]
enum Stopper {
    Closer,
    Otherwise,
    End,
}

/// How much of the expression grammar a hole is allowed to swallow.
///
/// Spec 3.10: inside an argument list `and` separates arguments, and a phrase never swallows a
/// following `or` so that a fallback attaches to the whole phrase rather than to its last hole.
#[derive(Copy, Clone)]
struct Ctx {
    allow_and: bool,
    allow_or: bool,
}

impl Ctx {
    const FULL: Ctx = Ctx {
        allow_and: true,
        allow_or: true,
    };
    /// Inside a value-producing phrase: leave `and` and `or` to whoever wrote the phrase.
    const TIGHT: Ctx = Ctx {
        allow_and: false,
        allow_or: false,
    };
    /// A call argument: `and` separates arguments, but a fallback still attaches.
    const ARG: Ctx = Ctx {
        allow_and: false,
        allow_or: true,
    };
}

pub struct Parsed {
    pub ast: Ast,
    pub problems: Vec<Diagnostic>,
}

struct Parser<'a> {
    toks: Vec<Token>,
    pos: usize,
    ast: Ast,
    vocab: &'a Vocabulary,
    keys: Keys,
    problems: Vec<Diagnostic>,
    /// Action names collected up front, so an action may be called before it is defined.
    action_names: Vec<Vec<Sym>>,
    depth: u32,
}

pub fn parse(src: &str, vocab: &Vocabulary) -> Parsed {
    let mut words = Interner::new();
    let (lexed, lex_problems) = crate::lex::lex(src, &mut words);
    let keys = Keys::new(&mut words);

    let mut ast = Ast::default();
    ast.words = words;
    ast.texts = lexed.texts;

    let mut p = Parser {
        toks: lexed.tokens,
        pos: 0,
        ast,
        vocab,
        keys,
        problems: lex_problems,
        action_names: Vec::new(),
        depth: 0,
    };

    p.collect_action_names();
    let top = p.parse_top();
    p.ast.top = top;

    Parsed {
        ast: p.ast,
        problems: p.problems,
    }
}

impl<'a> Parser<'a> {
    // -----------------------------------------------------------------
    // Token helpers
    // -----------------------------------------------------------------

    fn tok(&self) -> Tok {
        self.toks[self.pos].tok
    }

    fn span(&self) -> Span {
        self.toks[self.pos].span
    }

    fn at_word(&self, s: Sym) -> bool {
        matches!(self.tok(), Tok::Word(w) if w == s)
    }

    fn word_at(&self, offset: usize) -> Option<Sym> {
        match self.toks.get(self.pos + offset).map(|t| t.tok) {
            Some(Tok::Word(w)) => Some(w),
            _ => None,
        }
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos];
        if !matches!(t.tok, Tok::End) {
            self.pos += 1;
        }
        t
    }

    fn eat_word(&mut self, s: Sym) -> bool {
        if self.at_word(s) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat(&mut self, t: Tok) -> bool {
        if self.tok() == t {
            self.bump();
            true
        } else {
            false
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.tok(), Tok::Newline) {
            self.bump();
        }
    }

    fn skip_to_end_of_line(&mut self) {
        while !matches!(self.tok(), Tok::Newline | Tok::End) {
            self.bump();
        }
    }

    fn word_text(&self, s: Sym) -> &str {
        self.ast.words.text(s)
    }

    fn here_words(&self, n: usize) -> String {
        let mut out = String::new();
        for k in 0..n {
            match self.toks.get(self.pos + k).map(|t| t.tok) {
                Some(Tok::Word(w)) => {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(self.word_text(w));
                }
                _ => break,
            }
        }
        out
    }

    // -----------------------------------------------------------------
    // Pre-pass: action names
    // -----------------------------------------------------------------

    fn collect_action_names(&mut self) {
        let define = self.keys.define;
        let with = self.keys.with;
        let mut i = 0;
        while i < self.toks.len() {
            if matches!(self.toks[i].tok, Tok::Word(w) if w == define) {
                let mut j = i + 1;
                let mut words = Vec::new();
                while let Some(Tok::Word(w)) = self.toks.get(j).map(|t| t.tok) {
                    if w == with {
                        break;
                    }
                    words.push(w);
                    j += 1;
                }
                if !words.is_empty() {
                    self.action_names.push(words);
                }
                i = j;
            } else {
                i += 1;
            }
        }
        // Longest names first, so `load the score` beats `load`.
        self.action_names.sort_by(|a, b| b.len().cmp(&a.len()));
        self.action_names.dedup();
    }

    fn match_action_name(&self) -> Option<(Vec<Sym>, usize)> {
        for name in &self.action_names {
            let mut ok = true;
            for (k, w) in name.iter().enumerate() {
                if self.word_at(k) != Some(*w) {
                    ok = false;
                    break;
                }
            }
            if ok {
                return Some((name.clone(), name.len()));
            }
        }
        None
    }

    fn action_sym(&mut self, words: &[Sym]) -> Sym {
        let joined = words
            .iter()
            .map(|w| self.ast.words.text(*w).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        self.ast.words.intern(&joined)
    }

    // -----------------------------------------------------------------
    // Courtesy
    // -----------------------------------------------------------------

    fn eat_opener(&mut self) -> bool {
        for opener in OPENERS {
            let mut ok = true;
            for (k, word) in opener.iter().enumerate() {
                match self.word_at(k) {
                    Some(w) if self.word_text(w) == *word => {}
                    _ => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                for _ in 0..opener.len() {
                    self.bump();
                }
                return true;
            }
        }
        false
    }

    fn at_closer(&self) -> bool {
        if self.at_word(self.keys.thanks) {
            return true;
        }
        self.at_word(self.keys.thank) && self.word_at(1) == Some(self.keys.you)
    }

    /// Consume `thank you [for ...]` / `thanks [for ...]`, checking the name if one is given.
    fn eat_closer(&mut self, what: Closes) -> Option<Span> {
        if !self.at_closer() {
            return None;
        }
        let start = self.span();
        if self.eat_word(self.keys.thanks) {
            // done
        } else {
            self.bump(); // thank
            self.bump(); // you
        }
        let mut end = self.toks[self.pos.saturating_sub(1)].span;

        if self.at_word(self.keys.for_) {
            let for_span = self.span();
            self.bump();
            let mut named: Vec<Sym> = Vec::new();
            while let Tok::Word(w) = self.tok() {
                named.push(w);
                end = self.span();
                self.bump();
            }
            if named.is_empty() {
                self.problems.push(
                    Diagnostic::problem(for_span, "This `for` does not say what is being closed.")
                        .because(
                            "A closer may name what it closes, as in `thank you for repeating`. \
                             Plain `thank you` is just as good.",
                        )
                        .suggest("Either name it, or leave it out:", "thank you"),
                );
            } else {
                let allowed = allowed_closer_names(what);
                let ok = named
                    .iter()
                    .any(|w| allowed.contains(&self.word_text(*w)));
                if !ok {
                    let written = named
                        .iter()
                        .map(|w| self.word_text(*w))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let expected = allowed[0];
                    self.problems.push(
                        Diagnostic::problem(
                            Span::new(for_span.start, end.end),
                            format!(
                                "This closes {}, but it says `thank you for {}`.",
                                what.describe(),
                                written
                            ),
                        )
                        .because(
                            "Naming what you are closing is optional, and I check it when you do, \
                             so that crossed blocks are caught here rather than puzzled over later.",
                        )
                        .suggest(
                            "Either name it to match, or simply say thanks:",
                            format!("thank you for {expected}"),
                        ),
                    );
                }
            }
        }
        Some(Span::new(start.start, end.end))
    }

    // -----------------------------------------------------------------
    // Blocks
    // -----------------------------------------------------------------

    fn parse_top(&mut self) -> BlockId {
        let mut ids = Vec::new();
        loop {
            self.skip_newlines();
            match self.tok() {
                Tok::End => break,
                _ => {}
            }
            if self.at_closer() {
                let span = self.span();
                self.problems.push(
                    Diagnostic::problem(span, "There is nothing here for this to close.")
                        .because(
                            "`thank you` closes a block that a request opened. This one is at the \
                             top of the file, with no block open.",
                        )
                        .suggest("You can simply remove it, or open something first:", "please:"),
                );
                self.skip_to_end_of_line();
                continue;
            }
            if let Some(id) = self.parse_statement() {
                ids.push(id);
            }
        }
        self.ast.push_block(&ids)
    }

    fn parse_body(&mut self) -> (BlockId, Stopper) {
        self.depth += 1;
        let mut ids = Vec::new();
        let stopper = loop {
            self.skip_newlines();
            match self.tok() {
                Tok::End => break Stopper::End,
                Tok::Word(w) if w == self.keys.otherwise => break Stopper::Otherwise,
                _ => {}
            }
            if self.at_closer() {
                break Stopper::Closer;
            }
            match self.parse_statement() {
                Some(id) => ids.push(id),
                None => {}
            }
        };
        self.depth -= 1;
        (self.ast.push_block(&ids), stopper)
    }

    /// Finish a block: consume its closer, or explain gently that it is missing.
    fn finish(&mut self, what: Closes, opened_at: Span, stopper: Stopper) {
        match stopper {
            Stopper::Closer => {
                self.eat_closer(what);
            }
            Stopper::End => {
                self.problems.push(
                    Diagnostic::problem(
                        opened_at,
                        format!("This opens {} that is never closed.", what.describe()),
                    )
                    .because(
                        "A block runs until you say thank you. That is what takes the place of \
                         brackets, and it means the indentation is yours to arrange however you \
                         like.",
                    )
                    .suggest("Close it at the end:", "thanks"),
                );
            }
            Stopper::Otherwise => {
                let span = self.span();
                self.problems.push(
                    Diagnostic::problem(span, "There is no check here for `otherwise` to belong to.")
                        .because(
                            "`otherwise` offers an alternative to a check, so it has to follow one.",
                        )
                        .suggest("A check reads like this:", "check if score is over 10:"),
                );
                self.skip_to_end_of_line();
            }
        }
    }

    fn expect_colon(&mut self, what: &str) {
        if self.eat(Tok::Colon) {
            return;
        }
        let span = self.span();
        self.problems.push(
            Diagnostic::problem(span, format!("I was expecting a colon after {what}."))
                .because(
                    "A colon says that a block follows, the way it does in ordinary writing: \
                     here comes the list of what to do.",
                )
                .suggest("Put one at the end of the line:", format!("{what}:")),
        );
    }

    // -----------------------------------------------------------------
    // Statements
    // -----------------------------------------------------------------

    fn parse_statement(&mut self) -> Option<StmtId> {
        let start = self.span();
        let polite = self.eat_opener();

        // Rule 3: a courtesy word covers everything inside the block it opened, so only the
        // outermost level has to ask.
        if !polite && self.depth == 0 {
            let mut words = self.here_words(3);
            if !words.is_empty()
                && !matches!(
                    self.toks.get(self.pos + words.split(' ').count()).map(|t| t.tok),
                    Some(Tok::Newline) | Some(Tok::End) | None
                )
            {
                words.push_str(" ...");
            }
            self.problems.push(
                Diagnostic::problem(start, "This is missing a `please`.")
                    .because(
                        "Out here at the top of the file, every request asks for itself. Inside a \
                         block you only have to ask once, when you open it.",
                    )
                    .suggest(
                        "Ask, and I will happily do it:",
                        if words.is_empty() {
                            "please ...".to_string()
                        } else {
                            format!("please {words}")
                        },
                    ),
            );
        }

        // `please:` — a courtesy block.
        if self.eat(Tok::Colon) {
            let (body, stopper) = self.parse_body();
            self.finish(Closes::Courtesy, start, stopper);
            let span = Span::new(start.start, self.span().end);
            return Some(
                self.ast
                    .push_stmt(StmtKind::Courtesy { body }, span, polite),
            );
        }

        if self.at_word(self.keys.define) {
            return self.parse_define(start, polite);
        }

        if self.at_word(self.keys.r#try) && self.word_at(1) == Some(self.keys.to) {
            return self.parse_try(start, polite);
        }

        // An action you defined yourself wins over the shipped vocabulary. You taught me that
        // word on purpose, so it is the one you meant.
        if let Some(stmt) = self.parse_call_statement(start, polite) {
            return Some(stmt);
        }

        if let Some(stmt) = self.parse_phrase_statement(start, polite) {
            return Some(stmt);
        }

        self.unknown_statement(start);
        None
    }

    fn unknown_statement(&mut self, start: Span) {
        let phrase = self.here_words(4);
        let mut d = Diagnostic::problem(
            start,
            if phrase.is_empty() {
                "I do not know what is being asked for here.".to_string()
            } else {
                format!("I do not know how to `{phrase}`.")
            },
        );
        if let Tok::Word(w) = self.tok() {
            let word = self.word_text(w).to_string();
            if let Some(near) = self.vocab.nearest_word(&word) {
                if near != word {
                    d = d
                        .because(format!("There is no word `{word}` in the language."))
                        .suggest(
                            format!("Did you mean `{near}`?"),
                            format!("please {near} ..."),
                        );
                }
            }
        }
        if d.because.is_none() {
            d = d
                .because(
                    "Every request starts with a word I know. `polite words` lists them all, and \
                     `polite explain \"...\"` says what one means.",
                )
                .suggest(
                    "The everyday words are a good place to start:",
                    "please show \"hello\"",
                );
        }
        self.problems.push(d);
        self.skip_to_end_of_line();
    }

    fn parse_define(&mut self, start: Span, polite: bool) -> Option<StmtId> {
        self.bump(); // define

        let mut name_words = Vec::new();
        while let Tok::Word(w) = self.tok() {
            if w == self.keys.with {
                break;
            }
            name_words.push(w);
            self.bump();
        }
        if name_words.is_empty() {
            self.problems.push(
                Diagnostic::problem(start, "This defines an action but does not name it.")
                    .because("An action needs a name so you can ask for it later.")
                    .suggest("Give it one:", "please define greet:"),
            );
            self.skip_to_end_of_line();
            return None;
        }

        let mut params: Vec<Sym> = Vec::new();
        if self.eat_word(self.keys.with) {
            loop {
                // Articles read naturally and mean nothing: `with a name and a greeting`.
                while self.at_word(self.keys.a)
                    || self.at_word(self.keys.an)
                    || self.at_word(self.keys.the)
                {
                    self.bump();
                }
                match self.tok() {
                    Tok::Word(w) => {
                        params.push(w);
                        self.bump();
                    }
                    _ => {
                        let span = self.span();
                        self.problems.push(
                            Diagnostic::problem(span, "I was expecting the name of a value here.")
                                .because("After `with`, you name what the action needs.")
                                .suggest(
                                    "Name them like this:",
                                    "please define greet with a name and a greeting:",
                                ),
                        );
                        break;
                    }
                }
                if !self.eat_word(self.keys.and) && !self.eat(Tok::Comma) {
                    break;
                }
            }
        }

        self.expect_colon("the name of the action");
        let (body, stopper) = self.parse_body();
        self.finish(Closes::Define, start, stopper);

        let name = self.action_sym(&name_words);
        let params_range = self.ast.push_names(&params);
        let span = Span::new(start.start, self.span().end);
        self.ast.actions.push(Action {
            name,
            words: name_words,
            params: params_range,
            body,
            span,
            shared: false,
        });
        let action = self.ast.actions.len() as u32 - 1;
        Some(self.ast.push_stmt(StmtKind::Define { action }, span, polite))
    }

    fn parse_try(&mut self, start: Span, polite: bool) -> Option<StmtId> {
        self.bump(); // try
        self.bump(); // to
        self.expect_colon("`try to`");
        let (body, stopper) = self.parse_body();

        let otherwise = match stopper {
            Stopper::Otherwise => {
                self.bump(); // otherwise
                // `otherwise if it does not work out:` — every word after `otherwise` is
                // conversation, so simply run to the colon.
                while !matches!(self.tok(), Tok::Colon | Tok::Newline | Tok::End) {
                    self.bump();
                }
                self.expect_colon("`otherwise if it does not work out`");
                let (b, s2) = self.parse_body();
                self.finish(Closes::Try, start, s2);
                b
            }
            other => {
                self.problems.push(
                    Diagnostic::problem(start, "This tries something but never says what to do if it does not work out.")
                        .because(
                            "Trying is for the times when something might not work out. If nothing \
                             follows, there is nowhere for me to go when it does not.",
                        )
                        .suggest(
                            "Add the other half:",
                            "otherwise if it does not work out:\n    show \"Oh dear.\"\nthanks",
                        ),
                );
                self.finish(Closes::Try, start, other);
                self.ast.push_block(&[])
            }
        };

        let reason = self.keys.reason_name;
        let span = Span::new(start.start, self.span().end);
        Some(self.ast.push_stmt(
            StmtKind::Try {
                body,
                otherwise,
                reason,
            },
            span,
            polite,
        ))
    }

    fn parse_check(&mut self, start: Span, polite: bool, first_cond: ExprId) -> Option<StmtId> {
        self.expect_colon("the condition");
        let mut arms: Vec<CheckArm> = Vec::new();
        let (body, mut stopper) = self.parse_body();
        arms.push(CheckArm {
            cond: Some(first_cond),
            body,
            span: start,
        });

        loop {
            match stopper {
                Stopper::Otherwise => {
                    let arm_start = self.span();
                    self.bump(); // otherwise
                    let cond = if self.eat_word(self.keys.r#if) {
                        Some(self.parse_expr(Ctx::FULL, &[]))
                    } else {
                        None
                    };
                    self.expect_colon(if cond.is_some() {
                        "the condition"
                    } else {
                        "`otherwise`"
                    });
                    let (b, s) = self.parse_body();
                    arms.push(CheckArm {
                        cond,
                        body: b,
                        span: arm_start,
                    });
                    stopper = s;
                    if cond.is_none() {
                        // A plain `otherwise` is the last word on the matter.
                        self.finish(Closes::Check, start, stopper);
                        break;
                    }
                }
                other => {
                    self.finish(Closes::Check, start, other);
                    break;
                }
            }
        }

        let range = self.ast.push_arms(&arms);
        let span = Span::new(start.start, self.span().end);
        Some(
            self.ast
                .push_stmt(StmtKind::Check { arms: range }, span, polite),
        )
    }

    fn parse_phrase_statement(&mut self, start: Span, polite: bool) -> Option<StmtId> {
        let first = match self.tok() {
            Tok::Word(w) => w,
            _ => return None,
        };
        let word = self.word_text(first).to_string();
        let candidates: Vec<u32> = self.vocab.starting_with(&word).to_vec();
        let save = self.pos;

        for pid in candidates {
            let phrase = self.vocab.phrase(pid);
            if phrase.kind == Kind::Expr {
                continue;
            }
            let kind = phrase.kind;
            let form = phrase.form;
            match self.match_pieces(pid, Ctx::FULL) {
                Some((names, args)) => {
                    if form == Form::Check {
                        // Special shape: `check if ...:` carries arms rather than one body.
                        let cond = args[0];
                        return self.parse_check(start, polite, cond);
                    }
                    let body = if kind == Kind::Block {
                        self.expect_colon("the request");
                        let (b, stopper) = self.parse_body();
                        self.finish(Closes::Loop, start, stopper);
                        Some(b)
                    } else {
                        None
                    };
                    let names_range = self.ast.push_names(&names);
                    let args_range = self.ast.push_args(&args);
                    let span = Span::new(start.start, self.span().end);
                    return Some(self.ast.push_stmt(
                        StmtKind::Form {
                            form,
                            phrase: pid,
                            names: names_range,
                            args: args_range,
                            body,
                        },
                        span,
                        polite,
                    ));
                }
                None => {
                    self.pos = save;
                }
            }
        }
        None
    }

    fn parse_call_statement(&mut self, start: Span, polite: bool) -> Option<StmtId> {
        let (words, len) = self.match_action_name()?;
        for _ in 0..len {
            self.bump();
        }
        let name = self.action_sym(&words);
        let args = self.parse_call_args();
        let args_range = self.ast.push_args(&args);
        let span = Span::new(start.start, self.span().end);
        Some(self.ast.push_stmt(
            StmtKind::Call {
                name,
                args: args_range,
            },
            span,
            polite,
        ))
    }

    fn parse_call_args(&mut self) -> Vec<ExprId> {
        let mut args = Vec::new();
        if self.eat_word(self.keys.with) {
            loop {
                args.push(self.parse_expr(Ctx::ARG, &[]));
                if !self.eat_word(self.keys.and) && !self.eat(Tok::Comma) {
                    break;
                }
            }
        }
        args
    }

    // -----------------------------------------------------------------
    // Phrase matching
    // -----------------------------------------------------------------

    /// Try to match every piece of a phrase at the current position.
    ///
    /// Returns the `{name}` holes and the expression holes, in the order written.
    fn match_pieces(&mut self, pid: u32, ctx: Ctx) -> Option<(Vec<Sym>, Vec<ExprId>)> {
        let pieces: Vec<Piece> = self.vocab.phrase(pid).pieces.clone();
        let mut names = Vec::new();
        let mut args = Vec::new();
        self.match_pieces_from(&pieces, 0, ctx, &mut names, &mut args)?;
        Some((names, args))
    }

    fn match_pieces_from(
        &mut self,
        pieces: &[Piece],
        from: usize,
        ctx: Ctx,
        names: &mut Vec<Sym>,
        args: &mut Vec<ExprId>,
    ) -> Option<()> {
        for idx in from..pieces.len() {
            match &pieces[idx] {
                Piece::Word(word) => {
                    let w = self.ast.words.get(word)?;
                    if !self.eat_word(w) {
                        return None;
                    }
                }
                Piece::Hole { takes_name, .. } => {
                    if *takes_name {
                        match self.tok() {
                            Tok::Word(w) => {
                                // A name hole must not swallow the phrase's own next word.
                                if let Some(Piece::Word(next)) = pieces.get(idx + 1) {
                                    if self.word_text(w) == &**next {
                                        return None;
                                    }
                                }
                                names.push(w);
                                self.bump();
                            }
                            _ => return None,
                        }
                    } else {
                        let stops = self.stop_words(pieces, idx + 1);
                        if matches!(self.tok(), Tok::Colon | Tok::Newline | Tok::End) {
                            return None;
                        }
                        args.push(self.parse_expr(ctx, &stops));
                    }
                }
            }
        }
        Some(())
    }

    /// The literal words still to come in a pattern. An expression hole stops when it reaches one.
    fn stop_words(&self, pieces: &[Piece], from: usize) -> Vec<Sym> {
        let mut out = Vec::new();
        for p in &pieces[from..] {
            if let Piece::Word(w) = p {
                if let Some(s) = self.ast.words.get(w) {
                    out.push(s);
                }
            }
        }
        out
    }

    // -----------------------------------------------------------------
    // Expressions
    // -----------------------------------------------------------------

    fn parse_expr(&mut self, ctx: Ctx, stops: &[Sym]) -> ExprId {
        let e = self.parse_or(ctx, stops);
        // `..., I am sure` (spec 7.2, way three).
        if ctx.allow_or
            && matches!(self.tok(), Tok::Comma)
            && self.word_at(1) == Some(self.keys.i)
            && self.word_at(2) == Some(self.keys.am)
            && self.word_at(3) == Some(self.keys.sure)
        {
            let start = self.ast.expr(e).span;
            self.bump();
            self.bump();
            self.bump();
            let end = self.span();
            self.bump();
            return self
                .ast
                .push_expr(ExprKind::Sure { value: e }, start.join(end));
        }
        e
    }

    fn parse_or(&mut self, ctx: Ctx, stops: &[Sym]) -> ExprId {
        let mut lhs = self.parse_and(ctx, stops);
        while ctx.allow_or && self.at_word(self.keys.or) && !stops.contains(&self.keys.or) {
            self.bump();
            let rhs = self.parse_and(ctx, stops);
            lhs = self.binary(BinOp::Or, lhs, rhs);
        }
        lhs
    }

    fn parse_and(&mut self, ctx: Ctx, stops: &[Sym]) -> ExprId {
        let mut lhs = self.parse_cmp(ctx, stops);
        while ctx.allow_and && self.at_word(self.keys.and) && !stops.contains(&self.keys.and) {
            self.bump();
            let rhs = self.parse_cmp(ctx, stops);
            lhs = self.binary(BinOp::And, lhs, rhs);
        }
        lhs
    }

    fn parse_cmp(&mut self, ctx: Ctx, stops: &[Sym]) -> ExprId {
        let lhs = self.parse_add(ctx, stops);
        if stops.contains(&self.keys.is) {
            return lhs;
        }
        // Symbol comparisons, allowed by rule 4.
        let sym_op = match self.tok() {
            Tok::Greater => Some(BinOp::Over),
            Tok::Less => Some(BinOp::Under),
            Tok::GreaterEqual => Some(BinOp::AtLeast),
            Tok::LessEqual => Some(BinOp::AtMost),
            _ => None,
        };
        if let Some(op) = sym_op {
            self.bump();
            let rhs = self.parse_add(ctx, stops);
            return self.binary(op, lhs, rhs);
        }

        if !self.at_word(self.keys.is) {
            return lhs;
        }
        self.bump(); // is

        if self.eat_word(self.keys.not) {
            let rhs = self.parse_add(ctx, stops);
            return self.binary(BinOp::IsNot, lhs, rhs);
        }
        if self.eat_word(self.keys.over) {
            let rhs = self.parse_add(ctx, stops);
            return self.binary(BinOp::Over, lhs, rhs);
        }
        if self.eat_word(self.keys.under) {
            let rhs = self.parse_add(ctx, stops);
            return self.binary(BinOp::Under, lhs, rhs);
        }
        if self.at_word(self.keys.at) && self.word_at(1) == Some(self.keys.least) {
            self.bump();
            self.bump();
            let rhs = self.parse_add(ctx, stops);
            return self.binary(BinOp::AtLeast, lhs, rhs);
        }
        if self.at_word(self.keys.at) && self.word_at(1) == Some(self.keys.most) {
            self.bump();
            self.bump();
            let rhs = self.parse_add(ctx, stops);
            return self.binary(BinOp::AtMost, lhs, rhs);
        }
        if self.eat_word(self.keys.between) {
            let low = self.parse_add(Ctx::TIGHT, stops);
            if !self.eat_word(self.keys.and) {
                let span = self.span();
                self.problems.push(
                    Diagnostic::problem(span, "`is between` needs two limits.")
                        .because("Between means inside a pair of ends, so I need both of them.")
                        .suggest("Say both:", "score is between 1 and 10"),
                );
            }
            let high = self.parse_add(Ctx::TIGHT, stops);
            let span = self.ast.expr(lhs).span.join(self.ast.expr(high).span);
            return self.ast.push_expr(
                ExprKind::Between {
                    value: lhs,
                    low,
                    high,
                },
                span,
            );
        }
        let rhs = self.parse_add(ctx, stops);
        self.binary(BinOp::Is, lhs, rhs)
    }

    fn parse_add(&mut self, ctx: Ctx, stops: &[Sym]) -> ExprId {
        let mut lhs = self.parse_mul(ctx, stops);
        loop {
            let op = match self.tok() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                Tok::Word(w) if w == self.keys.plus && !stops.contains(&w) => BinOp::Add,
                Tok::Word(w) if w == self.keys.minus && !stops.contains(&w) => BinOp::Sub,
                Tok::Word(w) if w == self.keys.then && !stops.contains(&w) => BinOp::Then,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_mul(ctx, stops);
            lhs = self.binary(op, lhs, rhs);
        }
        lhs
    }

    fn parse_mul(&mut self, ctx: Ctx, stops: &[Sym]) -> ExprId {
        let mut lhs = self.parse_unary(ctx, stops);
        loop {
            let op = match self.tok() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                Tok::Word(w) if w == self.keys.times && !stops.contains(&w) => BinOp::Mul,
                Tok::Word(w)
                    if w == self.keys.divided
                        && self.word_at(1) == Some(self.keys.by)
                        && !stops.contains(&w) =>
                {
                    self.bump();
                    BinOp::Div
                }
                _ => break,
            };
            self.bump();
            let rhs = self.parse_unary(ctx, stops);
            lhs = self.binary(op, lhs, rhs);
        }
        lhs
    }

    fn parse_unary(&mut self, ctx: Ctx, stops: &[Sym]) -> ExprId {
        if self.at_word(self.keys.not) && !stops.contains(&self.keys.not) {
            let start = self.span();
            self.bump();
            let operand = self.parse_unary(ctx, stops);
            let span = start.join(self.ast.expr(operand).span);
            return self.ast.push_expr(
                ExprKind::Unary {
                    op: UnOp::Not,
                    operand,
                },
                span,
            );
        }
        if matches!(self.tok(), Tok::Minus) {
            let start = self.span();
            self.bump();
            let operand = self.parse_unary(ctx, stops);
            let span = start.join(self.ast.expr(operand).span);
            return self.ast.push_expr(
                ExprKind::Unary {
                    op: UnOp::Negate,
                    operand,
                },
                span,
            );
        }
        self.parse_postfix(ctx, stops)
    }

    /// Infix phrases from the table: `{list} contains {value}`, `{value} mentions {part}`.
    fn parse_postfix(&mut self, ctx: Ctx, stops: &[Sym]) -> ExprId {
        let mut lhs = self.parse_primary(ctx, stops);
        loop {
            let key = match self.tok() {
                Tok::Word(w) if !stops.contains(&w) => w,
                _ => break,
            };
            let key_text = self.word_text(key).to_string();
            let candidates: Vec<u32> = self.vocab.infix_with(&key_text).to_vec();
            if candidates.is_empty() {
                break;
            }
            let save = self.pos;
            let mut matched = false;
            for pid in candidates {
                let pieces: Vec<Piece> = self.vocab.phrase(pid).pieces.clone();
                let form = self.vocab.phrase(pid).form;
                let mut names = Vec::new();
                let mut args = vec![lhs];
                if self
                    .match_pieces_from(&pieces, 1, Ctx::TIGHT, &mut names, &mut args)
                    .is_some()
                {
                    let span = self.ast.expr(lhs).span.join(self.span());
                    let range = self.ast.push_args(&args);
                    lhs = self.ast.push_expr(
                        ExprKind::Phrase {
                            form,
                            phrase: pid,
                            args: range,
                        },
                        span,
                    );
                    matched = true;
                    break;
                }
                self.pos = save;
            }
            if !matched {
                break;
            }
        }
        let _ = ctx;
        lhs
    }

    fn parse_primary(&mut self, ctx: Ctx, stops: &[Sym]) -> ExprId {
        let span = self.span();
        match self.tok() {
            Tok::Int(v) => {
                self.bump();
                self.ast.push_expr(ExprKind::Int(v), span)
            }
            Tok::Dec(v) => {
                self.bump();
                self.ast.push_expr(ExprKind::Dec(v), span)
            }
            Tok::Text(id) => {
                self.bump();
                self.build_text(id, span)
            }
            Tok::OpenParen => {
                self.bump();
                let inner = self.parse_expr(Ctx::FULL, &[]);
                if !self.eat(Tok::CloseParen) {
                    let here = self.span();
                    self.problems.push(
                        Diagnostic::problem(here, "This bracket is never closed.")
                            .because("Brackets come in pairs, the way they do in ordinary writing.")
                            .suggest("Close it:", ")"),
                    );
                }
                inner
            }
            Tok::Word(w) => {
                if stops.contains(&w) {
                    return self.missing_value(span);
                }
                if w == self.keys.yes {
                    self.bump();
                    return self.ast.push_expr(ExprKind::Yes, span);
                }
                if w == self.keys.no {
                    self.bump();
                    return self.ast.push_expr(ExprKind::No, span);
                }
                if w == self.keys.nothing {
                    self.bump();
                    return self.ast.push_expr(ExprKind::Nothing, span);
                }
                // `what went wrong`, bound inside the otherwise arm of a try.
                if w == self.keys.what
                    && self.word_at(1) == Some(self.keys.went)
                    && self.word_at(2) == Some(self.keys.wrong)
                {
                    self.bump();
                    self.bump();
                    let end = self.span();
                    self.bump();
                    return self
                        .ast
                        .push_expr(ExprKind::Name(self.keys.reason_name), span.join(end));
                }

                // A value-producing phrase from the table.
                let word = self.word_text(w).to_string();
                let candidates: Vec<u32> = self.vocab.starting_with(&word).to_vec();
                let save = self.pos;
                for pid in candidates {
                    if self.vocab.phrase(pid).kind != Kind::Expr {
                        continue;
                    }
                    let form = self.vocab.phrase(pid).form;
                    if let Some((_names, args)) = self.match_pieces(pid, Ctx::TIGHT) {
                        let range = self.ast.push_args(&args);
                        let full = span.join(self.toks[self.pos.saturating_sub(1)].span);
                        return self.ast.push_expr(
                            ExprKind::Phrase {
                                form,
                                phrase: pid,
                                args: range,
                            },
                            full,
                        );
                    }
                    self.pos = save;
                }

                // One of your own actions.
                if let Some((words, len)) = self.match_action_name() {
                    for _ in 0..len {
                        self.bump();
                    }
                    let name = self.action_sym(&words);
                    let args = if self.at_word(self.keys.with) {
                        self.parse_call_args()
                    } else {
                        Vec::new()
                    };
                    let range = self.ast.push_args(&args);
                    let full = span.join(self.toks[self.pos.saturating_sub(1)].span);
                    return self.ast.push_expr(
                        ExprKind::Call {
                            name,
                            args: range,
                        },
                        full,
                    );
                }

                self.bump();
                self.ast.push_expr(ExprKind::Name(w), span)
            }
            _ => {
                let _ = ctx;
                self.missing_value(span)
            }
        }
    }

    fn missing_value(&mut self, span: Span) -> ExprId {
        self.problems.push(
            Diagnostic::problem(span, "I was expecting a value here, and the sentence stops.")
                .because(
                    "A value is a number, some text, yes or no, a name you have remembered, or a \
                     phrase that works one out.",
                )
                .suggest("Something like:", "score"),
        );
        self.ast.push_expr(ExprKind::Nothing, span)
    }

    fn binary(&mut self, op: BinOp, lhs: ExprId, rhs: ExprId) -> ExprId {
        let span = self.ast.expr(lhs).span.join(self.ast.expr(rhs).span);
        self.ast.push_expr(ExprKind::Binary { op, lhs, rhs }, span)
    }

    /// Split text into its pieces.
    ///
    /// Whatever sits between the braces is a value like any other, so it is worked out with the
    /// ordinary expression grammar rather than being limited to a bare name.
    fn build_text(&mut self, id: u32, span: Span) -> ExprId {
        let raw = self.ast.texts[id as usize].clone();
        if !raw.contains('{') {
            let cleaned = undo_brace_escapes(&raw);
            self.ast.texts[id as usize] = cleaned;
            return self.ast.push_expr(ExprKind::Text(id), span);
        }

        let mut parts: Vec<InterpPart> = Vec::new();
        let mut literal = String::new();
        let chars: Vec<char> = raw.chars().collect();
        let mut i = 0usize;

        while i < chars.len() {
            if chars[i] != '{' {
                literal.push(chars[i]);
                i += 1;
                continue;
            }
            i += 1;

            // Collect up to the matching close brace, stepping over any text written inside.
            let mut inner = String::new();
            let mut depth = 1usize;
            let mut in_text = false;
            let mut closed = false;
            while i < chars.len() {
                let c = chars[i];
                if in_text {
                    if c == '"' {
                        in_text = false;
                    }
                    inner.push(c);
                    i += 1;
                    continue;
                }
                match c {
                    '"' => {
                        in_text = true;
                        inner.push(c);
                    }
                    '{' => {
                        depth += 1;
                        inner.push(c);
                    }
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            closed = true;
                            i += 1;
                            break;
                        }
                        inner.push(c);
                    }
                    _ => inner.push(c),
                }
                i += 1;
            }

            if !closed {
                self.problems.push(
                    Diagnostic::problem(span, "This text opens a brace but never closes it.")
                        .because(
                            "Braces inside text drop a value in, as in \"hello, {name}\". To show                              a brace itself, write a backslash before it.",
                        )
                        .suggest("Close it:", "\"hello, {name}\""),
                );
            }

            if !literal.is_empty() {
                self.ast.texts.push(undo_brace_escapes(&literal));
                parts.push(InterpPart::Text(self.ast.texts.len() as u32 - 1));
                literal.clear();
            }

            let trimmed = inner.trim().to_string();
            if trimmed.is_empty() {
                self.problems.push(
                    Diagnostic::problem(span, "There is nothing between these braces.")
                        .because("Braces inside text are for dropping a value in.")
                        .suggest("Name what goes there:", "\"hello, {name}\""),
                );
            } else {
                let e = self.parse_inside_braces(&trimmed, span);
                parts.push(InterpPart::Value(e));
            }
        }

        if !literal.is_empty() {
            self.ast.texts.push(undo_brace_escapes(&literal));
            parts.push(InterpPart::Text(self.ast.texts.len() as u32 - 1));
        }
        let range = self.ast.push_interp(&parts);
        self.ast.push_expr(ExprKind::Interp(range), span)
    }

    /// Work out the value written between a pair of braces.
    ///
    /// The words in there are read with the same lexer and the same grammar as everywhere else,
    /// so anything that is a value in ordinary code is a value in here too.
    fn parse_inside_braces(&mut self, inner: &str, span: Span) -> ExprId {
        let mut words = std::mem::take(&mut self.ast.words);
        let (lexed, problems) = crate::lex::lex(inner, &mut words);
        self.ast.words = words;
        self.problems.extend(problems);

        let base = self.ast.texts.len() as u32;
        self.ast.texts.extend(lexed.texts);

        // Everything in here points at the text it was written inside, which is where a person
        // would look for it.
        let toks: Vec<Token> = lexed
            .tokens
            .into_iter()
            .map(|t| Token {
                tok: match t.tok {
                    Tok::Text(k) => Tok::Text(base + k),
                    other => other,
                },
                span,
            })
            .collect();

        let saved_toks = std::mem::replace(&mut self.toks, toks);
        let saved_pos = std::mem::replace(&mut self.pos, 0);
        let e = self.parse_expr(Ctx::FULL, &[]);
        self.toks = saved_toks;
        self.pos = saved_pos;
        e
    }
}

fn undo_brace_escapes(s: &str) -> String {
    s.replace('\u{1}', "{").replace('\u{2}', "}")
}
