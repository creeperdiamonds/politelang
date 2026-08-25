//! Suites one and two of spec section 11: the vocabulary round-trip and the ambiguity check.
//!
//! Spec 4.1: tests are free. Every row in the table gets a generated test without anybody writing
//! it, which is the only way a four-hundred-word language stays trustworthy.

mod common;

use polite_syntax::ast::{ExprKind, StmtKind};
use polite_vocab::{Kind, Piece, Vocabulary};

/// Build a sentence that uses a phrase, by filling its holes with something simple.
fn sentence_for(phrase: &polite_vocab::Phrase) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut names = 0;
    for piece in &phrase.pieces {
        match piece {
            Piece::Word(w) => words.push(w.to_string()),
            Piece::Hole { takes_name, .. } => {
                if *takes_name {
                    names += 1;
                    words.push(format!("thing{names}"));
                } else {
                    words.push("1".to_string());
                }
            }
        }
    }
    let body = words.join(" ");
    match phrase.kind {
        Kind::Stmt => format!("please {body}\n"),
        Kind::Block => format!("please {body}:\n    show 1\nthanks\n"),
        Kind::Expr => format!("please show {body}\n"),
    }
}

#[test]
fn every_phrase_in_the_table_parses_to_the_form_it_claims() {
    let vocab = Vocabulary::embedded();
    let mut checked = 0;
    let mut trouble: Vec<String> = Vec::new();

    for phrase in vocab.phrases() {
        let src = sentence_for(phrase);
        let parsed = polite_syntax::parse(&src, &vocab);

        // Only something that actually stops the program counts. A notice means it parsed
        // perfectly well and the language had something to say about it, which is its job.
        let stopped: Vec<_> = parsed
            .problems
            .iter()
            .filter(|d| d.severity == polite_diag::Severity::Problem)
            .collect();
        if !stopped.is_empty() {
            trouble.push(format!(
                "`{}` (vocabulary line {}) did not parse: {}",
                phrase.pattern, phrase.line, stopped[0].title
            ));
            continue;
        }

        let ast = &parsed.ast;
        let first = match ast.block(ast.top).first() {
            Some(s) => *s,
            None => {
                trouble.push(format!("`{}` produced nothing at all", phrase.pattern));
                continue;
            }
        };

        let found = match ast.stmt(first).kind {
            StmtKind::Form { form, args, .. } => {
                if phrase.kind == Kind::Expr {
                    // It was wrapped in `please show ...`, so look inside.
                    match ast.arg_slice(args).first() {
                        Some(a) => match ast.expr(*a).kind {
                            ExprKind::Phrase { form, .. } => Some(form),
                            _ => None,
                        },
                        None => None,
                    }
                } else {
                    Some(form)
                }
            }
            StmtKind::Check { .. } => Some(polite_vocab::Form::Check),
            _ => None,
        };

        match found {
            Some(f) if f == phrase.form => checked += 1,
            Some(f) => trouble.push(format!(
                "`{}` reached `{}` instead of `{}`",
                phrase.pattern,
                f.name(),
                phrase.form.name()
            )),
            None => trouble.push(format!(
                "`{}` did not reach a form at all",
                phrase.pattern
            )),
        }
    }

    assert!(
        trouble.is_empty(),
        "{} of {} phrases did not round-trip:\n{}",
        trouble.len(),
        vocab.phrases().len(),
        trouble.join("\n")
    );
    assert!(checked > 60, "only {checked} phrases were exercised");
}

/// Spec 4.3. If this fails the language would have to guess which phrase was meant, and it must
/// never guess.
#[test]
fn no_two_phrases_could_match_the_same_sentence() {
    let vocab = Vocabulary::embedded();
    let conflicts = vocab.conflicts();
    assert!(
        conflicts.is_empty(),
        "the vocabulary has collisions:\n{}",
        conflicts
            .iter()
            .map(|c| format!("  {}\n", c.why))
            .collect::<String>()
    );
}

/// Spec 4.1: the documentation is generated from the table, so nothing may be undocumented.
#[test]
fn every_form_the_table_uses_is_explained_in_english() {
    let vocab = Vocabulary::embedded();
    let mut missing: Vec<&str> = vocab
        .phrases()
        .iter()
        .filter(|p| vocab.explanation(p.form).is_none())
        .map(|p| p.form.name())
        .collect();
    missing.sort_unstable();
    missing.dedup();
    assert!(missing.is_empty(), "forms with no explanation: {missing:?}");
}

/// The explanations are for people, so they have to read like English.
#[test]
fn every_explanation_is_a_finished_sentence() {
    let vocab = Vocabulary::embedded();
    for form in polite_vocab::Form::ALL {
        if let Some(text) = vocab.explanation(*form) {
            assert!(
                text.ends_with('.'),
                "the explanation of `{}` does not finish: {text}",
                form.name()
            );
            assert!(
                text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false),
                "the explanation of `{}` does not start with a capital: {text}",
                form.name()
            );
        }
    }
}

/// Spec 12: the editor grammar is generated from the table, never hand-edited, so it cannot drift.
#[test]
fn the_editor_grammar_matches_the_table() {
    let vocab = Vocabulary::embedded();
    let generated = polite_cli::grammar::build(&vocab);
    let shipped = common::read(
        &common::root()
            .join("vscode-politelang")
            .join("syntaxes")
            .join("politelang.tmLanguage.json"),
    );
    assert_eq!(
        generated.trim(),
        shipped.trim(),
        "the shipped grammar is out of date. Run `polite grammar` to write it again."
    );
}
