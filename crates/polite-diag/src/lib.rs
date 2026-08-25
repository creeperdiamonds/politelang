//! Gentle messages.
//!
//! Spec section 8: error messages are the product, not an afterthought. Five rules, enforced on
//! every message:
//!
//! 1. Plain English sentences; no jargon without explaining it.
//! 2. Never blame. "I cannot...", never "you did X wrong", never illegal / invalid / fatal.
//! 3. Show the actual line.
//! 4. Suggest a fix.
//! 5. Explain why, so the reader learns something.
//!
//! Spec section 10.1: diagnostics are lazy. A `Diagnostic` is a small structured record, and
//! nothing is rendered into a string unless it is genuinely being shown to somebody. Code that
//! compiles pays nothing for any of this.

#![forbid(unsafe_code)]

use std::fmt::Write as _;

/// A position in a source file, as byte offsets.
///
/// Spec 10.1: positions are byte offsets, not line/column. Tracking line and column through
/// lexing is pure waste since almost none of it is ever used. The line is computed only when a
/// message is actually printed.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const fn new(start: u32, end: u32) -> Self {
        Span { start, end }
    }

    /// A span covering both of these and everything between.
    pub fn join(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn is_empty(self) -> bool {
        self.end <= self.start
    }
}

/// How serious a message is.
///
/// Deliberately not called "error" and "warning". A `Problem` stops the program from running; a
/// `Notice` is the language mentioning something useful, as in spec 7.4 where it tells you an
/// action of yours might not work out.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Severity {
    Problem,
    Notice,
}

/// A suggested replacement, shown as code the reader can copy.
#[derive(Clone, Debug)]
pub struct Suggestion {
    /// The sentence introducing it, e.g. "Did you mean to show them together instead?"
    pub lead: String,
    /// The code itself.
    pub code: String,
}

/// One message.
///
/// Built as structured data. Rendering happens in [`render`], and only then.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    pub span: Span,
    /// The first line: what happened, in plain English, never blaming.
    pub title: String,
    /// Rule 5: why, so the reader learns something. Wrapped when rendered.
    pub because: Option<String>,
    /// Rule 4: a fix.
    pub suggestion: Option<Suggestion>,
    /// Extra spans worth pointing at, each with a short label.
    pub also: Vec<(Span, String)>,
}

impl Diagnostic {
    pub fn problem(span: Span, title: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Problem,
            span,
            title: title.into(),
            because: None,
            suggestion: None,
            also: Vec::new(),
        }
    }

    pub fn notice(span: Span, title: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Notice,
            span,
            title: title.into(),
            because: None,
            suggestion: None,
            also: Vec::new(),
        }
    }

    pub fn because(mut self, why: impl Into<String>) -> Self {
        self.because = Some(why.into());
        self
    }

    pub fn suggest(mut self, lead: impl Into<String>, code: impl Into<String>) -> Self {
        self.suggestion = Some(Suggestion {
            lead: lead.into(),
            code: code.into(),
        });
        self
    }

    pub fn also(mut self, span: Span, label: impl Into<String>) -> Self {
        self.also.push((span, label.into()));
        self
    }
}

/// A source file, able to turn byte offsets into line numbers when finally asked.
pub struct Source<'a> {
    pub name: &'a str,
    pub text: &'a str,
    /// Byte offset of the start of each line. Built once, on demand.
    line_starts: Vec<u32>,
}

impl<'a> Source<'a> {
    pub fn new(name: &'a str, text: &'a str) -> Self {
        let mut line_starts = Vec::with_capacity(text.len() / 24 + 1);
        line_starts.push(0);
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i as u32 + 1);
            }
        }
        Source {
            name,
            text,
            line_starts,
        }
    }

    /// 1-based line number containing this byte offset.
    pub fn line_of(&self, offset: u32) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(i) => i + 1,
            Err(i) => i,
        }
    }

    /// The text of a 1-based line, without its newline.
    pub fn line_text(&self, line: usize) -> &'a str {
        if line == 0 || line > self.line_starts.len() {
            return "";
        }
        let start = self.line_starts[line - 1] as usize;
        let end = self
            .line_starts
            .get(line)
            .map(|e| *e as usize)
            .unwrap_or(self.text.len());
        self.text[start..end].trim_end_matches(['\n', '\r'])
    }

    /// Byte offset where a 1-based line begins.
    pub fn line_start(&self, line: usize) -> u32 {
        self.line_starts.get(line - 1).copied().unwrap_or(0)
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}

/// Turn a diagnostic into the text a person reads.
///
/// This is the only place a message becomes a string.
pub fn render(source: &Source<'_>, d: &Diagnostic) -> String {
    let mut out = String::with_capacity(320);
    let line = source.line_of(d.span.start).max(1);

    let opener = match d.severity {
        Severity::Problem => "In",
        Severity::Notice => "Just so you know, in",
    };
    let _ = writeln!(out, "{} {}, line {}:", opener, source.name, line);
    out.push('\n');

    write_snippet(&mut out, source, line, d.span);
    out.push('\n');

    wrap_into(&mut out, &d.title, 68, "");

    for (span, label) in &d.also {
        let other_line = source.line_of(span.start).max(1);
        out.push('\n');
        let _ = writeln!(out, "  line {}: {}", other_line, label);
        write_snippet(&mut out, source, other_line, *span);
    }

    if let Some(why) = &d.because {
        out.push('\n');
        wrap_into(&mut out, why, 68, "");
    }

    if let Some(s) = &d.suggestion {
        out.push('\n');
        wrap_into(&mut out, &s.lead, 68, "");
        out.push('\n');
        for l in s.code.lines() {
            if l.trim().is_empty() {
                out.push('\n');
            } else {
                let _ = writeln!(out, "    {}", l);
            }
        }
    }

    out
}

fn write_snippet(out: &mut String, source: &Source<'_>, line: usize, span: Span) {
    let text = source.line_text(line);
    let _ = writeln!(out, "    {}", text);

    // The caret row, aligned by counting characters rather than bytes so that non-ASCII
    // source still lines up.
    let line_start = source.line_start(line);
    if span.start >= line_start {
        let before = &source.text[line_start as usize..span.start as usize];
        if !before.contains('\n') {
            let pad = before.chars().count();
            let width = source.text[span.start as usize..(span.end as usize).min(source.text.len())]
                .split('\n')
                .next()
                .unwrap_or("")
                .chars()
                .count()
                .max(1);
            let mut caret = String::with_capacity(4 + pad + width);
            caret.push_str("    ");
            for _ in 0..pad {
                caret.push(' ');
            }
            for _ in 0..width {
                caret.push('^');
            }
            let _ = writeln!(out, "{}", caret);
        }
    }
}

/// Wrap prose to a width, so long explanations stay readable in a terminal.
fn wrap_into(out: &mut String, text: &str, width: usize, indent: &str) {
    for paragraph in text.split('\n') {
        let mut col = 0usize;
        let mut first = true;
        out.push_str(indent);
        for word in paragraph.split_whitespace() {
            let w = word.chars().count();
            if !first && col + 1 + w > width {
                out.push('\n');
                out.push_str(indent);
                col = 0;
                first = true;
            }
            if !first {
                out.push(' ');
                col += 1;
            }
            out.push_str(word);
            col += w;
            first = false;
        }
        out.push('\n');
    }
}

/// Words a PoliteLang message may never contain (spec principle 3).
///
/// Checked by a test over every message the compiler can produce, so the tone cannot rot.
pub const BLAME_WORDS: &[&str] = &[
    "illegal",
    "invalid",
    "fatal",
    "forbidden",
    "you failed",
    "your fault",
    "bad input",
    "malformed",
    "unexpected token",
    "panic",
    "abort",
];

/// Returns the first blame word found in a message, if any.
pub fn find_blame_word(message: &str) -> Option<&'static str> {
    let lower = message.to_ascii_lowercase();
    BLAME_WORDS.iter().copied().find(|w| lower.contains(w))
}

/// A collected set of messages.
#[derive(Default)]
pub struct Bag {
    items: Vec<Diagnostic>,
}

impl Bag {
    pub fn new() -> Self {
        Bag::default()
    }

    pub fn push(&mut self, d: Diagnostic) {
        self.items.push(d);
    }

    pub fn extend(&mut self, other: impl IntoIterator<Item = Diagnostic>) {
        self.items.extend(other);
    }

    pub fn has_problems(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Problem)
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items.iter()
    }

    /// Sort by position so messages arrive in reading order.
    pub fn sorted(mut self) -> Vec<Diagnostic> {
        self.items.sort_by_key(|d| d.span.start);
        self.items
    }

    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.items
    }
}

/// Render every message in a bag, in reading order.
pub fn render_all(source: &Source<'_>, bag: Bag) -> String {
    let items = bag.sorted();
    let mut out = String::new();
    for (i, d) in items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&render(source, d));
    }
    out
}


/// Several files gathered into one piece of text (spec section 5).
///
/// Every span in the compiler is an offset into the gathered whole, and this is what turns one
/// back into a place in the file somebody actually wrote.
pub struct Files<'a> {
    parts: Vec<(u32, u32, Source<'a>)>,
}

impl<'a> Files<'a> {
    /// Build from `(name, start, text)` for each file, where `start` is its offset in the whole.
    pub fn new(parts: Vec<(&'a str, u32, &'a str)>) -> Files<'a> {
        Files {
            parts: parts
                .into_iter()
                .map(|(name, start, text)| {
                    (start, start + text.len() as u32, Source::new(name, text))
                })
                .collect(),
        }
    }

    /// The file an offset belongs to, and where that file begins.
    pub fn locate(&self, offset: u32) -> Option<(&Source<'a>, u32)> {
        self.parts
            .iter()
            .find(|(start, end, _)| offset >= *start && offset <= *end)
            .map(|(start, _, source)| (source, *start))
            .or_else(|| self.parts.last().map(|(start, _, s)| (s, *start)))
    }

    pub fn is_single(&self) -> bool {
        self.parts.len() <= 1
    }
}

/// Render every message, each against the file it was actually written in.
pub fn render_all_across(files: &Files<'_>, bag: Bag) -> String {
    let items = bag.sorted();
    let mut out = String::new();
    for (i, d) in items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match files.locate(d.span.start) {
            Some((source, base)) => {
                let mut local = d.clone();
                local.span = Span::new(
                    d.span.start.saturating_sub(base),
                    d.span.end.saturating_sub(base),
                );
                for (span, _) in &mut local.also {
                    *span = Span::new(
                        span.start.saturating_sub(base),
                        span.end.saturating_sub(base),
                    );
                }
                out.push_str(&render(source, &local));
            }
            None => out.push_str(&d.title),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_found_from_byte_offsets() {
        let text = "one\ntwo\nthree\n";
        let s = Source::new("t.polite", text);
        assert_eq!(s.line_of(0), 1);
        assert_eq!(s.line_of(4), 2);
        assert_eq!(s.line_of(8), 3);
        assert_eq!(s.line_text(2), "two");
        assert_eq!(s.line_count(), 4);
    }

    #[test]
    fn a_rendered_message_shows_the_line_and_a_caret() {
        let text = "please add \"hello\" to score\n";
        let s = Source::new("game.polite", text);
        let d = Diagnostic::problem(Span::new(11, 18), "I cannot add text to a number.")
            .because("`score` is a whole number and \"hello\" is text.")
            .suggest(
                "Did you mean to show them together instead?",
                "please show \"hello\" and score",
            );
        let out = render(&s, &d);
        assert!(out.contains("In game.polite, line 1:"));
        assert!(out.contains("please add \"hello\" to score"));
        assert!(out.contains("^^^^^^^"));
        assert!(out.contains("I cannot add text to a number."));
        assert!(out.contains("please show \"hello\" and score"));
    }

    #[test]
    fn our_own_messages_never_blame() {
        let d = Diagnostic::problem(Span::default(), "I cannot work out what this holds.");
        assert!(find_blame_word(&d.title).is_none());
        assert_eq!(find_blame_word("that is an invalid token"), Some("invalid"));
    }

    #[test]
    fn prose_wraps_without_breaking_words() {
        let mut out = String::new();
        wrap_into(&mut out, &"word ".repeat(40), 20, "");
        for line in out.lines() {
            assert!(line.chars().count() <= 20, "line too long: {line:?}");
        }
    }
}
