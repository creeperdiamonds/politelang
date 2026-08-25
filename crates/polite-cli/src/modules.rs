//! Borrowing between files (spec section 5).
//!
//! A file that borrows another is gathered together with it before anything else happens: the
//! borrowed files come first, the borrowing file last. What each file may see of another is not
//! decided here — that is the checker's job, which knows which file every name came from and
//! whether it was shared.
//!
//! Nothing is rewritten along the way: every file is laid down exactly as it was written, so a
//! message about a `use` line points at the line somebody actually typed. The parser knows that
//! borrowing was settled here and reads those lines as the no-ops they have become.

use polite_diag::{Diagnostic, Span};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Where one file sits inside the gathered whole.
#[derive(Clone, Debug)]
pub struct FilePart {
    pub name: String,
    /// Byte offset of this file's first character within the gathered text.
    pub start: u32,
    pub len: u32,
}

impl FilePart {
    pub fn end(&self) -> u32 {
        self.start + self.len
    }
}

pub struct Bundle {
    pub text: String,
    pub parts: Vec<FilePart>,
    pub problems: Vec<Diagnostic>,
}

impl Bundle {
    /// The ranges the checker needs to tell one file from another.
    pub fn ranges(&self) -> Vec<(u32, u32)> {
        self.parts.iter().map(|p| (p.start, p.end())).collect()
    }
}

/// Gather a file and everything it borrows.
pub fn gather(path: &Path) -> Result<Bundle, String> {
    let mut state = Gatherer {
        text: String::new(),
        parts: Vec::new(),
        problems: Vec::new(),
        visiting: Vec::new(),
        done: HashSet::new(),
    };
    let leftover = state.load(path, None)?;
    state.problems.extend(leftover);
    Ok(Bundle {
        text: state.text,
        parts: state.parts,
        problems: state.problems,
    })
}

/// Gather from text already in hand, borrowing nothing. Used by the test suites.
pub fn single(name: &str, text: &str) -> Bundle {
    Bundle {
        text: text.to_string(),
        parts: vec![FilePart {
            name: name.to_string(),
            start: 0,
            len: text.len() as u32,
        }],
        problems: Vec::new(),
    }
}

struct Gatherer {
    text: String,
    parts: Vec<FilePart>,
    /// Messages already placed, with offsets into the gathered text.
    problems: Vec<Diagnostic>,
    /// The chain of files currently being followed, for spotting a circle.
    visiting: Vec<PathBuf>,
    done: HashSet<PathBuf>,
}

impl Gatherer {
    /// Load a file and everything it borrows.
    ///
    /// Anything returned is a message *about the line in the borrowing file*, which cannot be
    /// placed until that file is, so it is handed back for the caller to position.
    fn load(&mut self, path: &Path, borrowed_by: Option<&str>) -> Result<Vec<Diagnostic>, String> {
        let full = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let name = short_name(&full);

        // Borrowing the same file twice is harmless; it is already here.
        if self.done.contains(&full) {
            return Ok(Vec::new());
        }

        if self.visiting.contains(&full) {
            let circle: Vec<String> = self
                .visiting
                .iter()
                .chain(std::iter::once(&full))
                .map(|p| short_name(p))
                .collect();
            return Ok(vec![Diagnostic::problem(
                Span::default(),
                "These files borrow each other in a circle.",
            )
            .because(format!(
                "The circle runs {}. Each one is waiting for the next, so none of them can go \
                 first.",
                circle.join(" then ")
            ))
            .suggest(
                "Move what they share into a file of its own, and let both borrow that:",
                "please use shared",
            )]);
        }

        let source = match std::fs::read_to_string(&full) {
            Ok(t) => t,
            Err(e) => match borrowed_by {
                Some(from) => {
                    return Ok(vec![Diagnostic::problem(
                        Span::default(),
                        format!("I could not borrow `{name}`, because {}.", why(&e)),
                    )
                    .because(format!(
                        "`{from}` asks for it, and I looked for it beside that file."
                    ))
                    .suggest(
                        "A file is borrowed by its name, without the ending:",
                        "please use helpers",
                    )])
                }
                None => return Err(format!("I could not read `{name}`, because {}.", why(&e))),
            },
        };

        self.visiting.push(full.clone());

        // Follow what this file borrows first, so anything it needs is already in place.
        let dir = full.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let mut mine: Vec<Diagnostic> = Vec::new();

        for found in find_borrows(&source) {
            let here = Span::new(found.start as u32, (found.start + found.len) as u32);
            let target = resolve(&dir, &found.reference);
            let mut raised = self.load(&target, Some(&name))?;
            for d in &mut raised {
                d.span = here;
            }
            mine.append(&mut raised);
        }

        self.visiting.pop();
        self.done.insert(full.clone());

        // Lay the file down.
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
        }
        let start = self.text.len() as u32;
        self.text.push_str(&source);
        if !self.text.ends_with('\n') {
            self.text.push('\n');
        }
        let len = self.text.len() as u32 - start;
        self.parts.push(FilePart { name, start, len });

        // Now that it has a place, its own messages have one too.
        for mut d in mine {
            d.span = Span::new(d.span.start + start, d.span.end + start);
            self.problems.push(d);
        }

        Ok(Vec::new())
    }
}

struct Borrow {
    reference: String,
    start: usize,
    len: usize,
}

/// Find the `use` and `borrow` lines, without needing the whole parser.
///
/// Only a line whose first words are a courtesy opener followed by `use` or `borrow` counts, so
/// the word is free to appear anywhere else without being mistaken for a request.
fn find_borrows(source: &str) -> Vec<Borrow> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    for raw in source.split_inclusive('\n') {
        let line_start = offset;
        offset += raw.len();

        let line = raw.trim_end_matches(['\n', '\r']);
        let trimmed = line.trim_start();
        if trimmed.starts_with("--") || trimmed.is_empty() {
            continue;
        }
        let indent = line.len() - trimmed.len();

        let mut words: Vec<&str> = trimmed.split_whitespace().collect();
        strip_opener(&mut words);
        if words.len() < 2 {
            continue;
        }
        if !matches!(words[0].to_ascii_lowercase().as_str(), "use" | "borrow") {
            continue;
        }
        let reference = words[1].trim_matches('"').to_string();
        if reference.is_empty() {
            continue;
        }
        out.push(Borrow {
            reference,
            start: line_start + indent,
            len: line.len() - indent,
        });
    }
    out
}

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

fn strip_opener(words: &mut Vec<&str>) {
    for opener in OPENERS {
        if words.len() >= opener.len()
            && words[..opener.len()]
                .iter()
                .zip(*opener)
                .all(|(a, b)| a.to_ascii_lowercase() == *b)
        {
            words.drain(..opener.len());
            return;
        }
    }
}

fn resolve(dir: &Path, reference: &str) -> PathBuf {
    let direct = dir.join(reference);
    if direct.extension().is_some() && direct.exists() {
        return direct;
    }
    let with_ending = dir.join(format!("{reference}.polite"));
    if with_ending.exists() {
        return with_ending;
    }
    direct
}

fn short_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn why(e: &std::io::Error) -> String {
    use std::io::ErrorKind::*;
    match e.kind() {
        NotFound => "there is no file there".to_string(),
        PermissionDenied => "it is not mine to read".to_string(),
        _ => "it could not be reached".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_borrow_line_is_recognised_however_politely_it_is_asked() {
        for line in [
            "please use helpers\n",
            "kindly borrow helpers\n",
            "would you please use helpers\n",
            "    please use helpers\n",
        ] {
            let found = find_borrows(line);
            assert_eq!(found.len(), 1, "did not see a borrow in {line:?}");
            assert_eq!(found[0].reference, "helpers");
        }
    }

    #[test]
    fn a_borrow_inside_a_comment_is_not_one() {
        assert!(find_borrows("-- please use helpers\n").is_empty());
    }

}
