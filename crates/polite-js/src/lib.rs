//! PoliteIR out, JavaScript in.
//!
//! Spec 9.4 promised a socket that backends plug into and said nothing above the middle language
//! would ever know one existed. This is the first thing plugged into it, and the whole of it reads
//! PoliteIR and never the sentence tree.
//!
//! ## Why JavaScript
//!
//! Because that is where the libraries are. A PoliteLang program that wants to be a Discord bot
//! does not want PoliteLang to grow a network stack, a TLS implementation and a gateway client; it
//! wants to be handed to something that already has them. Emitting JavaScript is how a language
//! with no dependencies borrows a world full of them without taking any of them on.
//!
//! ## Shape of what comes out
//!
//! Every action becomes one `async function`. Every slot becomes one `let`. Blocks become the arms
//! of a `switch` inside a `for(;;)`, and a jump is an assignment to the block number followed by
//! `continue` — which is the ordinary way basic blocks are written in a language that has no goto,
//! and it keeps the emitted code a flat mirror of the middle language rather than a reconstruction
//! of the loops and ifs that were there originally.
//!
//! Everything is `async` and every call is awaited, whether it needs to be or not. Awaiting
//! something that is not a promise costs a tick and nothing else, and the alternative is working
//! out which of a hundred and sixty builtins might one day become asynchronous.
//!
//! ## Failure
//!
//! Spec 7.2 and 7.4 in JavaScript's own terms: a builtin that might not work out throws
//! `Politely`, and a `try` becomes a `catch` that puts the reason in a slot and jumps to the
//! block that handles it. Where PoliteIR says the failure leaves the action altogether, nothing
//! catches it, and it propagates exactly as the middle language says it should. `stop everything`
//! and a broken `I am sure` throw something else, which no catch here will take — that being the
//! entire point of them.

#![forbid(unsafe_code)]

use std::fmt::Write as _;

use polite_diag::Diagnostic;
use polite_ir::{Backend, Builtin, CmpKind, Compare, Function, Instr, Program};

/// What a backend is allowed to leave out, and what it must say when it does.
///
/// Being told plainly that a thing is not here is worth a great deal more than a program that runs
/// and quietly does something else, so anything missing throws by name at the moment it is
/// reached rather than being left to fail strangely later.
const NOT_YET: &[Builtin] = &[
    Builtin::OpenCanvas,
    Builtin::ClearCanvas,
    Builtin::PaintPoint,
    Builtin::DrawLine,
    Builtin::DrawBox,
    Builtin::FillBox,
    Builtin::DrawCircle,
    Builtin::RevealCanvas,
    Builtin::RevealLetters,
    Builtin::MakeColour,
    Builtin::NamedColour,
    Builtin::CanvasWidth,
    Builtin::CanvasHeight,
    Builtin::ColourAt,
    Builtin::WriteText,
    Builtin::LetterSize,
    Builtin::WrittenWidth,
    Builtin::SaveCanvas,
    Builtin::PutInWindow,
    Builtin::DotSize,
];

pub struct JavaScript {
    pub out: String,
}

impl JavaScript {
    pub fn new() -> JavaScript {
        JavaScript { out: String::new() }
    }
}

impl Default for JavaScript {
    fn default() -> Self {
        JavaScript::new()
    }
}

impl Backend for JavaScript {
    fn name(&self) -> &str {
        "javascript"
    }

    fn emit(&mut self, program: &Program) -> Result<(), Diagnostic> {
        self.out = emit(program);
        Ok(())
    }
}

/// The name a builtin goes by in the runtime.
///
/// Taken from the name of the instruction itself rather than from a table of a hundred and sixty
/// lines, so that adding a builtin to the middle language and adding it to the runtime are the
/// only two places anybody has to touch.
pub fn runtime_name(which: Builtin) -> String {
    let name = format!("{which:?}");
    let mut letters = name.chars();
    match letters.next() {
        Some(first) => first.to_lowercase().collect::<String>() + letters.as_str(),
        None => name,
    }
}

/// A piece of text as JavaScript source for that same text.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // These two end a line in JavaScript but not in anything else, which is a famous way
            // for a piece of text to break the program it is sitting inside.
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A decimal number as JavaScript source for that same number.
fn decimal(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value.is_infinite() {
        if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string()
    } else {
        // `{:?}` on a float is the one that reads back as exactly the same number.
        let shown = format!("{value:?}");
        if shown.ends_with(".0") {
            shown[..shown.len() - 2].to_string()
        } else {
            shown
        }
    }
}

fn slot(s: u32) -> String {
    format!("s{s}")
}

fn args_of(args: &[u32]) -> String {
    args.iter().map(|a| slot(*a)).collect::<Vec<_>>().join(", ")
}

fn compare(op: Compare, kind: CmpKind, a: u32, b: u32) -> String {
    let (a, b) = (slot(a), slot(b));
    match kind {
        // Plain JavaScript comparison is right for anything that is a number or a piece of text
        // by the time it gets here, which lowering has already settled.
        CmpKind::Number | CmpKind::Decimal | CmpKind::Text | CmpKind::YesNo => {
            let operator = match op {
                Compare::Equal => "===",
                Compare::NotEqual => "!==",
                Compare::Over => ">",
                Compare::Under => "<",
                Compare::AtLeast => ">=",
                Compare::AtMost => "<=",
            };
            format!("{a} {operator} {b}")
        }
        // Lists and lookups are compared item by item, which JavaScript will not do for you.
        CmpKind::Value => {
            let same = format!("P.same({a}, {b})");
            match op {
                Compare::Equal => same,
                Compare::NotEqual => format!("!{same}"),
                // Ordering two lists is not something the language offers, so this cannot arise;
                // if it ever does, saying so beats inventing an answer.
                _ => format!("P.cannotOrder({a}, {b})"),
            }
        }
    }
}

fn emit_function(out: &mut String, index: usize, f: &Function) {
    let params: Vec<String> = (0..f.param_slots.len()).map(|i| format!("a{i}")).collect();
    let _ = writeln!(
        out,
        "\n// {}\nasync function f{index}({}) {{",
        if f.name.is_empty() {
            "the file itself".to_string()
        } else {
            f.name.clone()
        },
        params.join(", ")
    );

    if f.slot_count > 0 {
        let names: Vec<String> = (0..f.slot_count).map(slot).collect();
        let _ = writeln!(out, "  let {};", names.join(", "));
    }
    for (i, s) in f.param_slots.iter().enumerate() {
        let _ = writeln!(out, "  {} = a{i};", slot(*s));
    }

    let _ = writeln!(out, "  let b = {};", f.entry);
    let _ = writeln!(out, "  for (;;) switch (b) {{");

    for (id, block) in f.blocks.iter().enumerate() {
        let _ = writeln!(out, "  case {id}: {{");
        let mut ended = false;
        for instr in &block.instrs {
            emit_instr(out, instr);
            if instr.is_terminator() {
                ended = true;
            }
        }
        // A block with no terminator falls out of the bottom of the action, which is what an
        // action with nothing to give back does.
        if !ended {
            let _ = writeln!(out, "    return;");
        }
        let _ = writeln!(out, "  }}");
    }

    let _ = writeln!(out, "  default: return;");
    let _ = writeln!(out, "  }}\n}}");
}

fn emit_instr(out: &mut String, instr: &Instr) {
    let put = |out: &mut String, line: String| {
        let _ = writeln!(out, "    {line}");
    };

    match instr {
        Instr::ConstWhole { dst, value } => put(out, format!("{} = {value};", slot(*dst))),
        Instr::ConstDecimal { dst, value } => {
            put(out, format!("{} = {};", slot(*dst), decimal(*value)))
        }
        Instr::ConstText { dst, text } => put(out, format!("{} = T[{text}];", slot(*dst))),
        Instr::ConstYesNo { dst, value } => put(out, format!("{} = {value};", slot(*dst))),
        Instr::ConstNothing { dst } => put(out, format!("{} = null;", slot(*dst))),
        Instr::Move { dst, src } => put(out, format!("{} = {};", slot(*dst), slot(*src))),

        // Whole numbers go through a check, decimals do not. JavaScript stops counting exactly
        // at about nine thousand million million, and past that a whole number does not become
        // approximate — it becomes wrong while still looking like an answer. The language does
        // not quietly invent answers anywhere else and it will not start here, so the check costs
        // a call on every whole-number step and is worth it.
        Instr::AddWhole { dst, a, b } => put(
            out,
            format!("{} = P.whole({} + {});", slot(*dst), slot(*a), slot(*b)),
        ),
        Instr::SubWhole { dst, a, b } => put(
            out,
            format!("{} = P.whole({} - {});", slot(*dst), slot(*a), slot(*b)),
        ),
        Instr::MulWhole { dst, a, b } => put(
            out,
            format!("{} = P.whole({} * {});", slot(*dst), slot(*a), slot(*b)),
        ),
        Instr::AddDecimal { dst, a, b } => {
            put(out, format!("{} = {} + {};", slot(*dst), slot(*a), slot(*b)))
        }
        Instr::SubDecimal { dst, a, b } => {
            put(out, format!("{} = {} - {};", slot(*dst), slot(*a), slot(*b)))
        }
        Instr::MulDecimal { dst, a, b } => {
            put(out, format!("{} = {} * {};", slot(*dst), slot(*a), slot(*b)))
        }
        // Widening is a real step in the middle language and nothing at all in JavaScript, where
        // every number was already the same kind of number.
        Instr::WholeToDecimal { dst, src } => {
            put(out, format!("{} = {};", slot(*dst), slot(*src)))
        }

        Instr::AddNumber { dst, a, b } => put(
            out,
            format!("{} = P.add({}, {});", slot(*dst), slot(*a), slot(*b)),
        ),
        Instr::SubNumber { dst, a, b } => put(
            out,
            format!("{} = P.sub({}, {});", slot(*dst), slot(*a), slot(*b)),
        ),
        Instr::MulNumber { dst, a, b } => put(
            out,
            format!("{} = P.mul({}, {});", slot(*dst), slot(*a), slot(*b)),
        ),
        Instr::NegateNumber { dst, src } => {
            put(out, format!("{} = P.negate({});", slot(*dst), slot(*src)))
        }
        Instr::NegateWhole { dst, src } | Instr::NegateDecimal { dst, src } => {
            put(out, format!("{} = -{};", slot(*dst), slot(*src)))
        }
        Instr::ConcatText { dst, a, b } => {
            put(out, format!("{} = {} + {};", slot(*dst), slot(*a), slot(*b)))
        }
        Instr::Cmp { dst, op, kind, a, b } => put(
            out,
            format!("{} = {};", slot(*dst), compare(*op, *kind, *a, *b)),
        ),
        Instr::Not { dst, src } => put(out, format!("{} = !{};", slot(*dst), slot(*src))),

        Instr::Call { dst, which, args } => {
            let call = format!("await P.{}({})", runtime_name(*which), args_of(args));
            match dst {
                Some(d) => put(out, format!("{} = {call};", slot(*d))),
                None => put(out, format!("{call};")),
            }
        }

        Instr::TryCall {
            dst,
            which,
            args,
            reason,
            fail,
        } => {
            let call = format!("await P.{}({})", runtime_name(*which), args_of(args));
            emit_attempt(out, dst.map(slot), call, *reason, *fail);
        }

        Instr::CallAction {
            dst,
            func,
            args,
            reason,
            fail,
        } => {
            let call = format!("await f{}({})", func, args_of(args));
            emit_attempt(out, dst.map(slot), call, *reason, *fail);
        }

        Instr::StopBecauseSure { reason, what } => {
            put(out, format!("P.wasNotSure({}, T[{what}]);", slot(*reason)))
        }
        Instr::StopEverything => put(out, "P.stopEverything();".to_string()),

        Instr::Jump { to } => put(out, format!("b = {to}; continue;")),
        Instr::Branch {
            cond,
            then_block,
            else_block,
        } => put(
            out,
            format!(
                "b = {} ? {then_block} : {else_block}; continue;",
                slot(*cond)
            ),
        ),
        Instr::Return { src } => match src {
            Some(s) => put(out, format!("return {};", slot(*s))),
            None => put(out, "return;".to_string()),
        },
    }
}

/// Something that might not work out, and what happens if it does not.
fn emit_attempt(
    out: &mut String,
    dst: Option<String>,
    call: String,
    reason: u32,
    fail: Option<u32>,
) {
    let assign = match &dst {
        Some(d) => format!("{d} = {call};"),
        None => format!("{call};"),
    };
    match fail {
        // Nothing catches it, so it leaves this action — which is exactly what the middle
        // language means by a failure with nowhere to go.
        None => {
            let _ = writeln!(out, "    {assign}");
        }
        Some(block) => {
            let _ = writeln!(out, "    try {{ {assign} }} catch (e) {{");
            let _ = writeln!(out, "      if (!(e instanceof P.Politely)) throw e;");
            let _ = writeln!(out, "      {} = e.message; b = {block}; continue;", slot(reason));
            let _ = writeln!(out, "    }}");
        }
    }
}

/// A whole program as one JavaScript module.
pub fn emit(program: &Program) -> String {
    let mut out = String::with_capacity(program.instruction_count() * 40 + 2048);

    out.push_str(
        "// Written by PoliteLang. Every line of this came from a program somebody wrote in\n\
         // English; nothing here was typed by hand, and editing it edits the wrong file.\n\n\
         import * as P from \"./polite.mjs\";\n",
    );

    let _ = writeln!(out, "\nconst T = [");
    for text in &program.texts {
        let _ = writeln!(out, "  {},", quoted(text));
    }
    let _ = writeln!(out, "];");

    for (i, f) in program.funcs.iter().enumerate() {
        emit_function(&mut out, i, f);
    }

    let _ = writeln!(
        out,
        "\n// Where it starts.\nawait P.begin(() => f{}());",
        program.main
    );

    out
}

/// Anything in a program that this backend cannot carry, said before a line of it is written.
pub fn cannot_carry(program: &Program) -> Vec<Builtin> {
    let mut found: Vec<Builtin> = Vec::new();
    for f in &program.funcs {
        for block in &f.blocks {
            for instr in &block.instrs {
                let which = match instr {
                    Instr::Call { which, .. } | Instr::TryCall { which, .. } => *which,
                    _ => continue,
                };
                if NOT_YET.contains(&which) && !found.contains(&which) {
                    found.push(which);
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every builtin the emitter can name has to exist on the other side.
    ///
    /// The two halves of this backend are `runtime_name` here and the exports in
    /// `runtime/polite.mjs`, and the only way they part company is quietly: somebody adds a
    /// builtin to the middle language, wires it through the checker and the lowering, and never
    /// touches the runtime. The program then emits a call to something that is not there and
    /// fails with a JavaScript error rather than a sentence. So both files are read here and set
    /// against each other.
    #[test]
    fn the_runtime_has_every_builtin_the_emitter_can_name() {
        const IR: &str = include_str!("../../polite-ir/src/lib.rs");
        const RUNTIME: &str = include_str!("../../../runtime/polite.mjs");

        let block = IR
            .split("pub enum Builtin {")
            .nth(1)
            .expect("the middle language should still have a Builtin")
            .split("
}")
            .next()
            .unwrap();

        let mut missing: Vec<String> = Vec::new();
        let mut counted = 0usize;
        for line in block.lines() {
            let name = line.trim().trim_end_matches(',');
            if name.is_empty()
                || name.starts_with("//")
                || !name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                || !name.chars().all(|c| c.is_ascii_alphanumeric())
            {
                continue;
            }
            counted += 1;
            let mut letters = name.chars();
            let wanted = letters.next().unwrap().to_ascii_lowercase().to_string()
                + letters.as_str();
            let exported = RUNTIME.contains(&format!("export function {wanted}("))
                || RUNTIME.contains(&format!("export const {wanted} ="))
                || RUNTIME.contains(&format!("export async function {wanted}("));
            if !exported {
                missing.push(wanted);
            }
        }

        assert!(counted > 150, "only found {counted} builtins, which cannot be right");
        assert!(
            missing.is_empty(),
            "{} builtins have no runtime to call: {:?}",
            missing.len(),
            missing
        );
    }

    #[test]
    fn text_is_quoted_so_that_it_cannot_break_out() {
        assert_eq!(quoted("hello"), "\"hello\"");
        assert_eq!(quoted("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(quoted("a\\b"), "\"a\\\\b\"");
        assert_eq!(quoted("line\nnext"), "\"line\\nnext\"");
        // The two separators that end a line in JavaScript and nowhere else.
        assert_eq!(quoted("a\u{2028}b"), "\"a\\u2028b\"");
        assert_eq!(quoted("a\u{2029}b"), "\"a\\u2029b\"");
        // A closing script tag inside text is harmless here, but the control characters are not.
        assert_eq!(quoted("\u{1}"), "\"\\u0001\"");
    }

    #[test]
    fn text_keeps_its_own_letters() {
        assert_eq!(quoted("שלום"), "\"שלום\"");
        assert_eq!(quoted("café"), "\"café\"");
    }

    #[test]
    fn decimals_read_back_as_the_same_number() {
        assert_eq!(decimal(1.0), "1");
        assert_eq!(decimal(0.5), "0.5");
        assert_eq!(decimal(-2.25), "-2.25");
        assert_eq!(decimal(f64::INFINITY), "Infinity");
        assert_eq!(decimal(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(decimal(f64::NAN), "NaN");
        // The one that catches a lazy formatter.
        assert_eq!(decimal(0.1 + 0.2), "0.30000000000000004");
    }

    #[test]
    fn a_builtin_is_named_the_same_on_both_sides() {
        assert_eq!(runtime_name(Builtin::Show), "show");
        assert_eq!(runtime_name(Builtin::ListItem), "listItem");
        assert_eq!(runtime_name(Builtin::TextStartsWith), "textStartsWith");
        assert_eq!(runtime_name(Builtin::Pi), "pi");
    }

    #[test]
    fn comparing_lists_does_not_become_a_javascript_identity_test() {
        // `===` on two lists asks whether they are the same list, which is not the question.
        let js = compare(Compare::Equal, CmpKind::Value, 1, 2);
        assert!(js.contains("P.same"), "{js}");
        let js = compare(Compare::NotEqual, CmpKind::Value, 1, 2);
        assert!(js.starts_with('!') && js.contains("P.same"), "{js}");
        // Text is fine with plain comparison.
        assert_eq!(compare(Compare::Equal, CmpKind::Text, 1, 2), "s1 === s2");
    }
}
