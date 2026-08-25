//! Source in, middle language out — the whole front of the compiler in one place.

use crate::modules::{self, Bundle};
use polite_diag::{render_all_across, Bag, Files};
use polite_ir::Program;
use polite_vocab::Vocabulary;
use std::path::Path;

pub struct Built {
    pub program: Option<Program>,
    /// Everything worth saying, already rendered against the file it belongs to.
    pub messages: String,
    /// The same messages, still structured, for anything that wants to look at them.
    pub problems: Vec<polite_diag::Diagnostic>,
    pub had_problems: bool,
    /// Numbers for `polite bench`.
    pub lines: usize,
    pub tree_bytes: usize,
    /// How many pieces of text this program asked to keep hidden. `polite run` will not start
    /// while this is above nought unless somebody agrees to it first.
    pub hidden: usize,
}

/// Build one piece of text, borrowing nothing.
pub fn build(name: &str, text: &str, vocab: &Vocabulary, optimise: bool) -> Built {
    build_bundle(modules::single(name, text), vocab, optimise)
}

/// Build a file from disk, together with everything it borrows (spec section 5).
pub fn build_path(path: &Path, vocab: &Vocabulary, optimise: bool) -> Result<Built, String> {
    let bundle = modules::gather(path)?;
    Ok(build_bundle(bundle, vocab, optimise))
}

fn build_bundle(mut bundle: Bundle, vocab: &Vocabulary, optimise: bool) -> Built {
    let gathering_problems = std::mem::take(&mut bundle.problems);
    let ranges = bundle.ranges();
    let text = bundle.text.as_str();
    let files = Files::new(
        bundle
            .parts
            .iter()
            .map(|p| {
                let start = p.start as usize;
                let end = (p.end() as usize).min(text.len());
                (p.name.as_str(), p.start, &text[start..end])
            })
            .collect(),
    );

    let mut bag = Bag::new();
    // Anything the gathering itself had to say comes first, and stops the rest: a file that
    // could not be borrowed would only cause a pile of confusing follow-on messages.
    bag.extend(gathering_problems);
    let gathering_failed = bag.has_problems();

    let parsed = polite_syntax::parse(text, vocab);
    if !gathering_failed {
        bag.extend(parsed.problems);
    }
    let parse_failed = bag.has_problems();

    let (mut checked, check_bag) = polite_check::check_across(&parsed.ast, vocab, &ranges);
    if !parse_failed {
        bag.extend(check_bag.into_vec());
    }

    let had_problems = bag.has_problems();
    let problems: Vec<polite_diag::Diagnostic> = bag.iter().cloned().collect();
    let messages = render_all_across(&files, bag);

    // A tree that could not be read is not worth lowering; the messages already say why.
    let program = if had_problems {
        None
    } else {
        let mut p = polite_ir::lower::lower(&parsed.ast, &mut checked, vocab);
        if optimise {
            polite_ir::optimise::run(&mut p);
        }
        Some(p)
    };

    Built {
        program,
        messages,
        problems,
        had_problems,
        lines: text.lines().count(),
        tree_bytes: parsed.ast.footprint_bytes(),
        hidden: parsed.hidden,
    }
}
