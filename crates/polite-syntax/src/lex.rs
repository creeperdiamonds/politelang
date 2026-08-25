//! The lexer.
//!
//! Hand-written, per spec 9.5: no parser-generator crates, because they are procedural macros and
//! proc macros are the single slowest thing in Rust compilation.
//!
//! Works over bytes, produces tokens that borrow nothing and carry a [`Span`] of byte offsets.

use crate::intern::{Interner, Sym};
use polite_diag::{Diagnostic, Span};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Tok {
    /// A word: a name, or a word belonging to a phrase. Always interned lowercased.
    Word(Sym),
    Int(i64),
    Dec(f64),
    /// Index into [`Lexed::texts`]. Interpolation braces are still in there.
    Text(u32),
    /// `:` opens a block. English punctuation, not programming notation.
    Colon,
    Comma,
    OpenParen,
    CloseParen,
    Plus,
    Minus,
    Star,
    Slash,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    /// End of a statement.
    Newline,
    End,
}

#[derive(Copy, Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

pub struct Lexed {
    pub tokens: Vec<Token>,
    pub texts: Vec<String>,
}

pub fn lex(src: &str, words: &mut Interner) -> (Lexed, Vec<Diagnostic>) {
    let bytes = src.as_bytes();
    let mut tokens: Vec<Token> = Vec::with_capacity(src.len() / 4 + 8);
    let mut texts: Vec<String> = Vec::new();
    let mut problems: Vec<Diagnostic> = Vec::new();
    let mut i = 0usize;
    let mut at_line_start = true;

    while i < bytes.len() {
        let c = bytes[i];

        // Spaces and tabs. Indentation is purely cosmetic (spec rule 2), so it is simply skipped.
        if c == b' ' || c == b'\t' || c == b'\r' {
            i += 1;
            continue;
        }

        if c == b'\n' {
            push_newline(&mut tokens, i);
            i += 1;
            at_line_start = true;
            continue;
        }

        // Comments: `-- to the end of the line`, and `by the way, ...` at the start of one.
        if c == b'-' && bytes.get(i + 1) == Some(&b'-') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if at_line_start && starts_with_ignore_case(&bytes[i..], b"by the way") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        at_line_start = false;

        let start = i as u32;

        // Text.
        if c == b'"' {
            i += 1;
            let mut value = String::new();
            let mut closed = false;
            // How deep inside `{ ... }` we are. A quote mark inside braces belongs to whatever is
            // being worked out in there, so it does not end the text.
            let mut depth = 0usize;
            while i < bytes.len() {
                let b = bytes[i];
                if b == b'"' && depth == 0 {
                    i += 1;
                    closed = true;
                    break;
                }
                if b == b'{' {
                    depth += 1;
                } else if b == b'}' && depth > 0 {
                    depth -= 1;
                }
                if b == b'\n' {
                    break;
                }
                if b == b'\\' && i + 1 < bytes.len() {
                    let e = bytes[i + 1];
                    let replacement = match e {
                        b'n' => Some('\n'),
                        b't' => Some('\t'),
                        b'r' => Some('\r'),
                        b'"' => Some('"'),
                        b'\\' => Some('\\'),
                        b'{' => Some('\u{1}'), // stands for a literal brace, undone when split
                        b'}' => Some('\u{2}'),
                        _ => None,
                    };
                    match replacement {
                        Some(ch) => {
                            value.push(ch);
                            i += 2;
                            continue;
                        }
                        None => {
                            problems.push(
                                Diagnostic::problem(
                                    Span::new(i as u32, i as u32 + 2),
                                    format!(
                                        "I do not know what `\\{}` means inside text.",
                                        e as char
                                    ),
                                )
                                .because(
                                    "A backslash inside text introduces something special. \
                                     I understand n for a new line, t for a tab, a quote mark, \
                                     another backslash, and a brace.",
                                )
                                .suggest(
                                    "For a backslash on its own, write two of them:",
                                    "\"a backslash looks like \\\\\"",
                                ),
                            );
                            i += 2;
                            continue;
                        }
                    }
                }
                let ch_len = utf8_len(b);
                value.push_str(&src[i..i + ch_len]);
                i += ch_len;
            }
            if !closed {
                problems.push(
                    Diagnostic::problem(
                        Span::new(start, i as u32),
                        "This text opens with a quote mark but never closes.",
                    )
                    .because("Text runs from one quote mark to the next, on the same line.")
                    .suggest("Add the closing quote mark:", "\"...\""),
                );
            }
            texts.push(value);
            tokens.push(Token {
                tok: Tok::Text(texts.len() as u32 - 1),
                span: Span::new(start, i as u32),
            });
            continue;
        }

        // Numbers.
        if c.is_ascii_digit() {
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let mut is_decimal = false;
            if j < bytes.len()
                && bytes[j] == b'.'
                && j + 1 < bytes.len()
                && bytes[j + 1].is_ascii_digit()
            {
                is_decimal = true;
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
            }
            let raw = &src[i..j];
            let span = Span::new(start, j as u32);
            let tok = if is_decimal {
                match raw.parse::<f64>() {
                    Ok(v) => Tok::Dec(v),
                    Err(_) => {
                        problems.push(
                            Diagnostic::problem(
                                span,
                                format!("I could not read `{raw}` as a number."),
                            )
                            .because("Decimal numbers are written with a single dot in them.")
                            .suggest("Like this:", "3.5"),
                        );
                        Tok::Dec(0.0)
                    }
                }
            } else {
                match raw.parse::<i64>() {
                    Ok(v) => Tok::Int(v),
                    Err(_) => {
                        problems.push(
                            Diagnostic::problem(
                                span,
                                format!("The number `{raw}` is larger than I can hold."),
                            )
                            .because(
                                "Whole numbers here go up to about 9 million million million.",
                            )
                            .suggest(
                                "A decimal number reaches much further, if you can spare the \
                                 exactness:",
                                "9000000000000000000.0",
                            ),
                        );
                        Tok::Int(0)
                    }
                }
            };
            tokens.push(Token { tok, span });
            i = j;
            continue;
        }

        // Words.
        if c.is_ascii_alphabetic() || c == b'_' {
            let mut j = i;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            let sym = words.intern(&src[i..j]);
            tokens.push(Token {
                tok: Tok::Word(sym),
                span: Span::new(start, j as u32),
            });
            i = j;
            continue;
        }

        // Punctuation and the arithmetic symbols rule 4 allows.
        let (tok, len) = match c {
            b':' => (Tok::Colon, 1),
            b',' => (Tok::Comma, 1),
            b'(' => (Tok::OpenParen, 1),
            b')' => (Tok::CloseParen, 1),
            b'+' => (Tok::Plus, 1),
            b'-' => (Tok::Minus, 1),
            b'*' => (Tok::Star, 1),
            b'/' => (Tok::Slash, 1),
            b'>' if bytes.get(i + 1) == Some(&b'=') => (Tok::GreaterEqual, 2),
            b'<' if bytes.get(i + 1) == Some(&b'=') => (Tok::LessEqual, 2),
            b'>' => (Tok::Greater, 1),
            b'<' => (Tok::Less, 1),
            _ => {
                let ch_len = utf8_len(c);
                let ch = &src[i..i + ch_len];
                problems.push(
                    Diagnostic::problem(
                        Span::new(start, (i + ch_len) as u32),
                        format!("I do not know what to do with `{ch}` here."),
                    )
                    .because(
                        "PoliteLang is written in words. The only marks it uses are quotes for \
                         text, a colon to open a block, brackets for grouping, a comma, and the \
                         arithmetic signs + - * / > < >= <=.",
                    )
                    .suggest(
                        "If you meant it as part of some text, put it in quotes:",
                        format!("please show \"{ch}\""),
                    ),
                );
                i += ch_len;
                continue;
            }
        };
        tokens.push(Token {
            tok,
            span: Span::new(start, (i + len) as u32),
        });
        i += len;
    }

    push_newline(&mut tokens, bytes.len());
    tokens.push(Token {
        tok: Tok::End,
        span: Span::new(bytes.len() as u32, bytes.len() as u32),
    });

    (Lexed { tokens, texts }, problems)
}

fn push_newline(tokens: &mut Vec<Token>, at: usize) {
    // Collapse runs of blank lines: one statement terminator is enough.
    if matches!(tokens.last().map(|t| t.tok), None | Some(Tok::Newline)) {
        return;
    }
    tokens.push(Token {
        tok: Tok::Newline,
        span: Span::new(at as u32, at as u32),
    });
}

fn starts_with_ignore_case(hay: &[u8], needle: &[u8]) -> bool {
    hay.len() >= needle.len()
        && hay[..needle.len()]
            .iter()
            .zip(needle)
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        let mut w = Interner::new();
        let (l, problems) = lex(src, &mut w);
        assert!(problems.is_empty(), "unexpected problems");
        l.tokens.into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn indentation_is_ignored_entirely() {
        let a = toks("please show 1\n");
        let b = toks("            please show 1\n");
        assert_eq!(a, b);
    }

    #[test]
    fn comments_disappear() {
        let a = toks("please show 1\n");
        let b = toks("-- a note\nplease show 1 -- another\n");
        let c = toks("by the way, ignore me\nplease show 1\n");
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn blank_lines_do_not_pile_up() {
        let t = toks("please show 1\n\n\n\nplease show 2\n");
        assert_eq!(t.iter().filter(|t| **t == Tok::Newline).count(), 2);
    }

    #[test]
    fn text_keeps_its_braces_for_later() {
        let mut w = Interner::new();
        let (l, _) = lex("\"hi {name}\"", &mut w);
        assert_eq!(l.texts[0], "hi {name}");
    }

    #[test]
    fn names_are_case_insensitive() {
        let mut w = Interner::new();
        let (l, _) = lex("Score score SCORE", &mut w);
        let syms: Vec<Tok> = l.tokens.iter().map(|t| t.tok).take(3).collect();
        assert_eq!(syms[0], syms[1]);
        assert_eq!(syms[1], syms[2]);
    }

    #[test]
    fn an_unknown_mark_is_explained_politely() {
        let mut w = Interner::new();
        let (_, problems) = lex("please show 1 @ 2", &mut w);
        assert_eq!(problems.len(), 1);
        assert!(polite_diag::find_blame_word(&problems[0].title).is_none());
    }
}
