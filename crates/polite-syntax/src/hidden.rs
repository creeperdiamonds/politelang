//! Text that hides what it says.
//!
//! A program should mean what it appears to mean. Text written in base sixty-four, in hexadecimal,
//! or in backslash escapes appears to mean nothing at all, and is the usual way something nasty
//! travels inside something harmless — the reader sees a shrug of letters, and only the machine
//! finds out what it said.
//!
//! So PoliteLang decodes it on sight and says so, and the decoded text is what the program gets.
//! A program that genuinely wants the letters as written says
//!
//! ```text
//! force not to decode "..."
//! ```
//!
//! which leaves it exactly as it is. That phrase is the point of the whole arrangement: hiding is
//! still allowed, but it can no longer be done quietly. It has to be asked for, in words, on the
//! line where it happens, where anybody reading the file will see it.
//!
//! ## Only when it is certain
//!
//! Deciding wrongly here would be worse than not deciding at all, because a program whose text
//! quietly changed underneath it would be maddening. So a piece of text is only decoded when there
//! is very little room for doubt:
//!
//!   * it is long enough that landing on the pattern by chance is unlikely,
//!   * every letter belongs to the encoding,
//!   * what comes out is proper text, not a heap of bytes,
//!   * and what comes out is readable — printable characters and spaces, nothing else.
//!
//! An identifier, a key, a hash, a colour, a short word: all of them fail one of those and are
//! left alone. Something carrying a sentence in disguise passes all four.

use polite_diag::{Diagnostic, Span};
use polite_vocab::Form;

use crate::ast::{Ast, ExprKind};

/// How a piece of text was hiding what it said.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Hiding {
    BaseSixtyFour,
    Hexadecimal,
    Escapes,
}

impl Hiding {
    pub fn named(self) -> &'static str {
        match self {
            Hiding::BaseSixtyFour => "base sixty-four",
            Hiding::Hexadecimal => "hexadecimal",
            Hiding::Escapes => "backslash escapes",
        }
    }
}

/// The shortest run of encoded letters worth suspecting.
///
/// Sixteen base sixty-four letters carry twelve bytes. For all twelve to come out as readable
/// text by chance is about one in a hundred thousand, and shorter than that the odds get
/// uncomfortable — `abcd` is perfectly good base sixty-four, and is also just a word.
const ENOUGH: usize = 16;

/// Whether what came out is text a person could read, rather than a heap of bytes.
///
/// This is the check that keeps keys, hashes and identifiers out of it: they decode to bytes that
/// are not letters, and are left exactly as they were written.
fn readable(text: &str) -> bool {
    if text.chars().count() < 4 {
        return false;
    }
    let mut letters = 0usize;
    for ch in text.chars() {
        if ch == '\n' || ch == '\t' || ch == '\r' {
            continue;
        }
        // Anything below a space, and the delete character, is not something written down.
        if (ch as u32) < 0x20 || ch as u32 == 0x7f {
            return false;
        }
        if ch.is_alphabetic() {
            letters += 1;
        }
    }
    // Readable text is mostly letters. A row of punctuation is not a sentence in disguise.
    letters * 2 >= text.chars().count()
}

fn six_bits(ch: u8) -> Option<u8> {
    match ch {
        b'A'..=b'Z' => Some(ch - b'A'),
        b'a'..=b'z' => Some(ch - b'a' + 26),
        b'0'..=b'9' => Some(ch - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn from_base_sixty_four(text: &str) -> Option<String> {
    let raw = text.as_bytes();
    if raw.len() < ENOUGH || raw.len() % 4 != 0 {
        return None;
    }

    // Padding lives at the end, and there is never more than two of it.
    let padding = raw.iter().rev().take_while(|c| **c == b'=').count();
    if padding > 2 || raw[..raw.len() - padding].contains(&b'=') {
        return None;
    }

    let mut out: Vec<u8> = Vec::with_capacity(raw.len() / 4 * 3);
    let mut held: u32 = 0;
    let mut bits = 0u32;
    for ch in &raw[..raw.len() - padding] {
        let six = six_bits(*ch)?;
        held = (held << 6) | six as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((held >> bits) as u8);
        }
    }
    String::from_utf8(out).ok()
}

fn from_hexadecimal(text: &str) -> Option<String> {
    let raw = text.as_bytes();
    if raw.len() < ENOUGH || raw.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(raw.len() / 2);
    for pair in raw.chunks(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
    }
    String::from_utf8(out).ok()
}

/// `\x41`, `A` and the handful of one-letter escapes.
fn from_escapes(text: &str) -> Option<String> {
    if !text.contains("\\x") && !text.contains("\\u") {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut left = text.chars().peekable();
    let mut found = 0usize;
    while let Some(ch) = left.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match left.next() {
            Some('x') => {
                let a = left.next()?.to_digit(16)?;
                let b = left.next()?.to_digit(16)?;
                out.push(char::from_u32(a * 16 + b)?);
                found += 1;
            }
            Some('u') => {
                // Both `A` and `\u{41}` are written in the wild.
                let mut digits = String::new();
                if left.peek() == Some(&'{') {
                    left.next();
                    while let Some(d) = left.next() {
                        if d == '}' {
                            break;
                        }
                        digits.push(d);
                    }
                } else {
                    for _ in 0..4 {
                        digits.push(left.next()?);
                    }
                }
                let value = u32::from_str_radix(&digits, 16).ok()?;
                out.push(char::from_u32(value)?);
                found += 1;
            }
            Some('n') => {
                out.push('\n');
                found += 1;
            }
            Some('t') => {
                out.push('\t');
                found += 1;
            }
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    if found == 0 {
        None
    } else {
        Some(out)
    }
}

/// What a piece of text really says, if it was written so as not to say it.
pub fn decoded(text: &str) -> Option<(Hiding, String)> {
    // Escapes first: they are the only one that can sit inside otherwise ordinary writing.
    if let Some(plain) = from_escapes(text) {
        if readable(&plain) && plain != text {
            return Some((Hiding::Escapes, plain));
        }
    }

    let trimmed = text.trim();
    if trimmed.len() != text.len() || trimmed.is_empty() {
        return None;
    }

    if let Some(plain) = from_base_sixty_four(trimmed) {
        if readable(&plain) {
            return Some((Hiding::BaseSixtyFour, plain));
        }
    }
    if let Some(plain) = from_hexadecimal(trimmed) {
        if readable(&plain) {
            return Some((Hiding::Hexadecimal, plain));
        }
    }
    None
}

/// What came of looking through a program for text that hides what it says.
pub struct Revealed {
    /// One message for every piece of text decoded, and every piece kept hidden.
    pub said: Vec<Diagnostic>,
    /// How many pieces of text this program asked to keep hidden from the reader.
    ///
    /// Anything above nought means nobody — not the language, not the person running it — knows
    /// what that part of the program says.
    pub kept: usize,
}

/// Decode every piece of hidden text in the tree, and say so for each one.
///
/// Text sitting directly inside `force not to decode` is left as it was written, and warned about.
/// Keeping something unreadable is allowed, but it is never quiet and never free: it is said at
/// the line it happens on, and `polite run` will not start until somebody agrees to it.
pub fn reveal(ast: &mut Ast) -> Revealed {
    // Everything that was explicitly asked to be left as it is.
    let mut spared: Vec<u32> = Vec::new();
    let mut kept = Vec::new();
    for node in &ast.exprs {
        if let ExprKind::Phrase {
            form: Form::NotDecoded,
            args,
            ..
        } = node.kind
        {
            for id in ast.arg_slice(args) {
                spared.push(*id);
            }
            kept.push(node.span);
        }
    }

    let mut said = Vec::new();
    for span in &kept {
        said.push(warning(*span));
    }
    for id in 0..ast.exprs.len() {
        if spared.contains(&(id as u32)) {
            continue;
        }
        let (which, span) = match ast.exprs[id].kind {
            ExprKind::Text(which) => (which, ast.exprs[id].span),
            _ => continue,
        };
        let Some((how, plain)) = decoded(&ast.texts[which as usize]) else {
            continue;
        };

        said.push(notice(span, how, &ast.texts[which as usize], &plain));

        // A fresh place in the table rather than writing over the old one, because two identical
        // pieces of writing share an entry and one of them may be a piece of a longer sentence.
        ast.texts.push(plain);
        let fresh = (ast.texts.len() - 1) as u32;
        ast.exprs[id].kind = ExprKind::Text(fresh);
    }
    Revealed {
        said,
        kept: kept.len(),
    }
}

/// Said at every `force not to decode`, without exception.
///
/// The phrase has exactly one purpose, which is to keep a part of the program unreadable. That may
/// be perfectly innocent. It is also precisely what somebody would write in order to smuggle
/// something past a person reading the file, and the two look identical from here — so the only
/// honest thing to say is that it cannot be told which this is.
fn warning(span: Span) -> Diagnostic {
    Diagnostic::notice(span, "This text is kept hidden on purpose, so I cannot tell you what it says.")
        .because(
            "MAY CONTAIN MALICIOUS CODE. Text is normally decoded on sight, so that a program              means what it looks like it means. This one has asked me not to, and I have agreed              — which means neither of us has read it. Before running this, satisfy yourself that              whoever wrote it had a reason to hide it.",
        )
        .suggest(
            "If it did not need hiding, take the phrase off and let it be read:",
            "please remember message is \"...\"",
        )
}

fn notice(span: Span, how: Hiding, was: &str, plain: &str) -> Diagnostic {
    // A long secret is quoted only as far as it needs to be to recognise it.
    let short = |s: &str| -> String {
        let mut out: String = s.chars().take(60).collect();
        if s.chars().count() > 60 {
            out.push_str("...");
        }
        out.replace('\n', " ")
    };
    Diagnostic::notice(
        span,
        format!("This text is written in {}, so I have decoded it.", how.named()),
    )
    .because(format!(
        "It says:\n\n    \"{}\"\n\nText that hides what it says is decoded on sight, because a \
         program should mean what it looks like it means. This is what the program will use.",
        short(plain)
    ))
    .suggest(
        "If it was meant to stay exactly as it is written, say so:",
        format!("force not to decode \"{}\"", short(was)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sentence_in_base_sixty_four_is_found() {
        // "please show the secret"
        let hidden = "cGxlYXNlIHNob3cgdGhlIHNlY3JldA==";
        let (how, plain) = decoded(hidden).expect("this should have been decoded");
        assert_eq!(how, Hiding::BaseSixtyFour);
        assert_eq!(plain, "please show the secret");
    }

    #[test]
    fn a_sentence_in_hexadecimal_is_found() {
        // "delete everything"
        let hidden = "64656c65746520657665727974686967";
        let (how, plain) = decoded(hidden).expect("this should have been decoded");
        assert_eq!(how, Hiding::Hexadecimal);
        assert_eq!(plain, "delete everythig");
    }

    #[test]
    fn escapes_are_turned_back_into_letters() {
        let (how, plain) = decoded("\\x68\\x65\\x6c\\x6c\\x6f there").expect("should decode");
        assert_eq!(how, Hiding::Escapes);
        assert_eq!(plain, "hello there");
    }

    #[test]
    fn ordinary_writing_is_left_completely_alone() {
        for plain in [
            "hello",
            "Please show the answer",
            "abcd",
            "the quick brown fox jumps",
            "",
            "   ",
            "Numbers Keep",
            "a chair that is sorry",
        ] {
            assert!(decoded(plain).is_none(), "{plain:?} should have been left alone");
        }
    }

    #[test]
    fn keys_and_hashes_and_colours_are_left_alone() {
        for data in [
            // A hash: good hexadecimal, but it is bytes, not writing.
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            // A colour, and far too short to suspect.
            "ff00ff",
            // Sixteen letters of base sixty-four that come out as bytes.
            "3q2+796tvu/erb7v",
            // An identifier that happens to use only these letters.
            "deadbeefdeadbeef",
        ] {
            assert!(decoded(data).is_none(), "{data:?} should have been left alone");
        }
    }

    #[test]
    fn something_too_short_is_never_suspected() {
        // "hello!!" would be readable, but seven letters of base sixty-four is not evidence.
        assert!(decoded("aGVsbG8h").is_none());
    }

    #[test]
    fn writing_with_padding_still_decodes_and_bad_padding_does_not() {
        assert!(decoded("cGxlYXNlIGJlIGtpbmQ=").is_some());
        // Padding in the middle is not padding, it is nonsense.
        assert!(decoded("cGxlYXNl=GJlIGtpbmQ=").is_none());
    }

    fn looked_over(src: &str) -> (crate::parse::Parsed, Vec<String>) {
        let v = polite_vocab::Vocabulary::embedded();
        let p = crate::parse::parse(src, &v);
        let titles = p.problems.iter().map(|d| d.title.clone()).collect();
        (p, titles)
    }

    #[test]
    fn hidden_writing_is_decoded_and_the_program_gets_what_it_really_says() {
        let (p, titles) = looked_over(
            "please remember message is \"cGxlYXNlIGRlbGV0ZSBldmVyeXRoaW5n\"
",
        );
        assert_eq!(p.hidden, 0, "nothing was asked to be kept hidden");
        assert!(
            titles.iter().any(|t| t.contains("base sixty-four")),
            "it should have said so: {titles:?}"
        );
        assert!(
            p.ast.texts.iter().any(|t| t == "please delete everything"),
            "the decoded writing should be in the program"
        );
    }

    #[test]
    fn asking_for_it_to_be_left_alone_keeps_it_and_warns_every_time() {
        let (p, titles) = looked_over(
            "please remember a is force not to decode \"cGxlYXNlIGRlbGV0ZSBldmVyeXRoaW5n\"
             please remember b is leave \"cGxlYXNlIGRlbGV0ZSBldmVyeXRoaW5n\" exactly as it is
",
        );
        assert_eq!(p.hidden, 2, "both should have been counted");
        let warned = titles.iter().filter(|t| t.contains("kept hidden on purpose")).count();
        assert_eq!(warned, 2, "each one gets its own warning: {titles:?}");
        assert!(
            !titles.iter().any(|t| t.contains("base sixty-four")),
            "it must not have decoded what it was told to leave alone"
        );
        assert!(
            !p.ast.texts.iter().any(|t| t == "please delete everything"),
            "the hidden writing must have stayed hidden"
        );
    }

    #[test]
    fn an_ordinary_program_is_never_warned_about() {
        let (p, titles) = looked_over("please show \"hello there\"
");
        assert_eq!(p.hidden, 0);
        assert!(titles.is_empty(), "nothing to say about this: {titles:?}");
    }

    #[test]
    fn what_comes_out_must_be_proper_text() {
        // Valid base sixty-four whose bytes are not text at all.
        assert!(decoded("////////////////").is_none());
    }
}
