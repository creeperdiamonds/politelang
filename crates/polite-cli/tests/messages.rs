//! Suite four of spec section 11: the message snapshots.
//!
//! This is the suite that matters most. Section 8 claims the messages *are* the product, and a
//! claim like that rots quietly unless something pins it down. Every file in `tests/errors` has
//! its exact expected message beside it, so a message cannot drift into jargon without a test
//! going red and a person having to read the new wording.

mod common;

use polite_cli::pipeline;
use polite_vocab::Vocabulary;

#[test]
fn every_message_reads_exactly_as_it_should() {
    let vocab = Vocabulary::embedded();
    let mut ran = 0;

    for case in common::cases("errors") {
        let name = case.file_name().unwrap().to_string_lossy().to_string();
        let built = pipeline::build_path(&case, &vocab, true)
            .unwrap_or_else(|e| panic!("{name} could not be gathered: {e}"));
        assert!(
            !built.messages.is_empty(),
            "{name} was supposed to have something to say, and said nothing"
        );
        common::compare(&case, "expected", &built.messages);
        ran += 1;
    }

    assert!(ran >= 10, "not enough messages are pinned down: {ran}");
}

/// Spec principle 3, enforced across every message the compiler can produce here.
#[test]
fn no_message_ever_blames_the_person_reading_it() {
    let vocab = Vocabulary::embedded();
    for folder in ["errors", "programs"] {
        for case in common::cases(folder) {
            let name = case.file_name().unwrap().to_string_lossy().to_string();
            let built = pipeline::build_path(&case, &vocab, true)
                .unwrap_or_else(|e| panic!("{name} could not be gathered: {e}"));
            if let Some(word) = polite_diag::find_blame_word(&built.messages) {
                panic!("{name} produced a message containing `{word}`:\n{}", built.messages);
            }
        }
    }
}

/// Every message shows the line it is about, and every message that reports a problem offers a
/// way forward (spec section 8, rules 3 and 4).
#[test]
fn every_message_shows_the_line_and_suggests_a_way_forward() {
    let vocab = Vocabulary::embedded();
    for case in common::cases("errors") {
        let name = case.file_name().unwrap().to_string_lossy().to_string();
        let built = pipeline::build_path(&case, &vocab, true)
            .unwrap_or_else(|e| panic!("{name} could not be gathered: {e}"));

        for d in &built.problems {
            if d.severity == polite_diag::Severity::Problem {
                assert!(
                    d.suggestion.is_some(),
                    "a problem in {name} offers no way forward: {}",
                    d.title
                );
            }
            assert!(
                d.title.ends_with('.') || d.title.ends_with('?'),
                "a message in {name} is not a finished sentence: {}",
                d.title
            );
        }

        for chunk in built.messages.split("\n\n") {
            if chunk.trim().is_empty() {
                continue;
            }
            if chunk.starts_with("In ") || chunk.starts_with("Just so you know, in ") {
                assert!(
                    chunk.contains(", line "),
                    "a message in {name} does not say which line it is about:\n{chunk}"
                );
            }
        }
    }
}
