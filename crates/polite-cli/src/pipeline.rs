//! Source in, middle language out — the whole front of the compiler in one place.

use polite_diag::{render_all, Bag, Source};
use polite_ir::Program;
use polite_vocab::Vocabulary;

pub struct Built {
    pub program: Option<Program>,
    /// Everything worth saying, already rendered.
    pub messages: String,
    pub had_problems: bool,
    /// Numbers for `polite bench`.
    pub lines: usize,
    pub tree_bytes: usize,
}

pub fn build(name: &str, text: &str, vocab: &Vocabulary, optimise: bool) -> Built {
    let parsed = polite_syntax::parse(text, vocab);

    let mut bag = Bag::new();
    bag.extend(parsed.problems);
    let parse_failed = bag.has_problems();

    let (mut checked, check_bag) = polite_check::check(&parsed.ast, vocab);
    bag.extend(check_bag.into_vec());

    let source = Source::new(name, text);
    let had_problems = bag.has_problems();
    let messages = render_all(&source, bag);

    // A tree that could not be read is not worth lowering; the messages already say why.
    let program = if had_problems || parse_failed {
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
        had_problems,
        lines: source.line_count(),
        tree_bytes: parsed.ast.footprint_bytes(),
    }
}
