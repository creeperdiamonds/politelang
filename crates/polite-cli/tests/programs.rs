//! Suite three of spec section 11: the program corpus.
//!
//! Every `.polite` file in `tests/programs` is run and its output compared with the `.expected`
//! file beside it. A `.replies` file, if present, is what the program is told when it asks.
//!
//! This suite doubles as the backend conformance suite (spec 11): when the native, JVM and
//! WebAssembly backends arrive, each must produce exactly these outputs or it is not finished.

mod common;

use polite_cli::pipeline;
use polite_run::{Limits, Scripted};
use polite_vocab::Vocabulary;

#[test]
fn every_program_in_the_corpus_does_what_it_says() {
    let vocab = Vocabulary::embedded();
    let mut ran = 0;

    for case in common::cases("programs") {
        let name = case.file_name().unwrap().to_string_lossy().to_string();
        let text = common::read(&case);

        let built = pipeline::build(&name, &text, &vocab, true);
        assert!(
            !built.had_problems,
            "{name} did not check out:\n{}",
            built.messages
        );
        let program = built.program.expect("a program that checks out should build");

        let replies: Vec<String> = {
            let path = case.with_extension("replies");
            if path.exists() {
                common::read(&path).lines().map(|l| l.to_string()).collect()
            } else {
                Vec::new()
            }
        };

        let mut world = Scripted::with_replies(replies);
        // Every seed is the same one, so anything using chance is repeatable.
        let outcome = polite_run::run_with(&program, &mut world, Limits::steps(50_000_000), Some(7));

        let mut got = world.output();
        if let Err(reason) = outcome {
            got.push_str(&format!("\n{reason}\n"));
        }

        common::compare(&case, "expected", &got);
        ran += 1;
    }

    assert!(ran >= 8, "the corpus is thinner than it should be: {ran}");
}

/// The same programs, with the optimisation passes turned off, must behave identically.
///
/// Spec 10.3 claims those passes are invisible to the person writing the program. This is the
/// test that keeps the claim honest.
#[test]
fn optimising_never_changes_what_a_program_does() {
    let vocab = Vocabulary::embedded();

    for case in common::cases("programs") {
        let name = case.file_name().unwrap().to_string_lossy().to_string();
        let text = common::read(&case);

        let replies: Vec<String> = {
            let path = case.with_extension("replies");
            if path.exists() {
                common::read(&path).lines().map(|l| l.to_string()).collect()
            } else {
                Vec::new()
            }
        };

        let mut outputs = Vec::new();
        for optimise in [true, false] {
            let built = pipeline::build(&name, &text, &vocab, optimise);
            let program = built.program.expect("should build");
            let mut world = Scripted::with_replies(replies.clone());
            let outcome =
                polite_run::run_with(&program, &mut world, Limits::steps(50_000_000), Some(7));
            let mut got = world.output();
            if let Err(reason) = outcome {
                got.push_str(&format!("\n{reason}\n"));
            }
            outputs.push(got);
        }

        assert_eq!(
            outputs[0], outputs[1],
            "{name} behaves differently once it has been optimised"
        );
    }
}
