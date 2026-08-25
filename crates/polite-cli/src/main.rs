//! The `polite` command.

#![forbid(unsafe_code)]

use polite_cli::{bench, grammar, pipeline, words};

use polite_vocab::Vocabulary;
use std::process::ExitCode;

const USAGE: &str = "\
polite — the PoliteLang toolchain

  polite run <file.polite>        run a program
      --allow-hidden              run it even though part of it is kept unreadable
      --seed <number>             make anything it leaves to chance repeatable
  polite check <file.polite>      look it over without running it
      --show-middle               also print the middle language
      --plain                     no optimisation passes

  polite words [about <topic>]    every word the language knows
      --tier everyday|working|full
  polite explain <phrase>         what a phrase means, in English

  polite check-vocabulary         make sure no two phrases could collide
  polite bench                    measure against the budgets in the spec
  polite grammar <file.json>      write the editor grammar

Written by Creeperdiamonds Studios.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    let vocab = Vocabulary::embedded();

    match args[0].as_str() {
        "run" => command_run(&args[1..], &vocab),
        "check" => command_check(&args[1..], &vocab),
        "words" => words::list(&args[1..], &vocab),
        "explain" => words::explain(&args[1..], &vocab),
        "check-vocabulary" => command_check_vocabulary(&vocab),
        "bench" => bench::run(&args[1..], &vocab),
        "grammar" => grammar::write(&args[1..], &vocab),
        "help" | "--help" | "-h" => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        other if other.ends_with(".polite") => command_run(&args[0..], &vocab),
        other => {
            eprintln!("I do not know the command `{other}`.\n");
            print!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn command_run(args: &[String], vocab: &Vocabulary) -> ExitCode {
    // A seed makes a run repeatable, which is what you want when a program uses chance and you
    // are trying to work out what it did.
    let mut seed: Option<u64> = None;
    // Agreeing to hidden text on the command line, for when there is nobody to ask.
    let mut allow_hidden = false;
    let mut rest: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--allow-hidden" {
            allow_hidden = true;
            i += 1;
        } else if args[i] == "--seed" {
            match args.get(i + 1).and_then(|v| v.parse::<u64>().ok()) {
                Some(v) => seed = Some(v),
                None => {
                    eprintln!("`--seed` wants a whole number after it.");
                    return ExitCode::FAILURE;
                }
            }
            i += 2;
        } else {
            rest.push(args[i].clone());
            i += 1;
        }
    }

    let path = match rest.first() {
        Some(p) => p.clone(),
        None => {
            eprintln!("Which file would you like me to run?");
            return ExitCode::FAILURE;
        }
    };
    let built = match pipeline::build_path(std::path::Path::new(&path), vocab, true) {
        Ok(b) => b,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::FAILURE;
        }
    };
    if !built.messages.is_empty() {
        print!("{}", built.messages);
    }
    let program = match built.program {
        Some(p) => p,
        None => return ExitCode::FAILURE,
    };

    // Text the program asked to keep hidden has already been pointed at, above. Nothing runs
    // until somebody who can see it says so.
    if built.hidden > 0 && !agreed_to_hidden(built.hidden, &path, allow_hidden) {
        return ExitCode::FAILURE;
    }

    let mut world = polite_run::Terminal;
    match polite_run::run_with(&program, &mut world, polite_run::Limits::none(), seed) {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            println!("\n{reason}");
            ExitCode::FAILURE
        }
    }
}

/// Ask before running a program that keeps part of itself unreadable.
///
/// The phrase `force not to decode` is allowed, and this is the price of it. Nobody has read that
/// text — not the language, not whoever is about to run it — so the only safe default is to stop
/// and say so. If there is nobody at the keyboard to ask, it stops anyway: a question nobody can
/// answer is not permission.
fn agreed_to_hidden(count: usize, path: &str, allowed_already: bool) -> bool {
    use std::io::{IsTerminal, Write};

    let pieces = if count == 1 {
        "one piece of text".to_string()
    } else {
        format!("{count} pieces of text")
    };
    let them = if count == 1 { "it" } else { "them" };

    if allowed_already {
        println!("\n  Running with {pieces} kept hidden, because you said so on the command line.");
        return true;
    }

    let rule = "-".repeat(74);
    println!("\n  {rule}");
    println!("   MAY CONTAIN MALICIOUS CODE");
    println!();
    println!("   This program keeps {pieces} hidden from me, in the place marked above.");
    println!("   I could not read {them}, so I cannot tell you what this program will do.");
    println!();
    println!("   Only carry on if you trust whoever wrote this file.");
    println!("  {rule}");

    if !std::io::stdin().is_terminal() {
        println!();
        println!("  There is nobody at the keyboard for me to ask, so I have not run it.");
        println!();
        println!("  Run it yourself and answer the question, or, if you already know what that");
        println!("  text says:");
        println!();
        println!("      polite run {path} --allow-hidden");
        println!();
        return false;
    }

    print!("\n  Run it anyway? Type yes, or anything else to stop.\n  > ");
    let _ = std::io::stdout().flush();

    let mut said = String::new();
    if std::io::stdin().read_line(&mut said).is_err() {
        println!("\n  I did not catch that, so I have not run it.");
        return false;
    }
    if is_yes(&said) {
        println!();
        true
    } else {
        println!("\n  Stopped. Nothing was run.");
        false
    }
}


/// Whether an answer to the hidden-text question was a yes.
///
/// The whole word, and nothing else. This is the one question in the language where a stray
/// keypress must not be able to agree to something on your behalf.
fn is_yes(said: &str) -> bool {
    said.trim().eq_ignore_ascii_case("yes")
}

fn command_check(args: &[String], vocab: &Vocabulary) -> ExitCode {
    let show_middle = args.iter().any(|a| a == "--show-middle");
    let optimise = !args.iter().any(|a| a == "--plain");
    let path = match args.iter().find(|a| !a.starts_with("--")) {
        Some(p) => p.clone(),
        None => {
            eprintln!("Which file would you like me to look over?");
            return ExitCode::FAILURE;
        }
    };
    let built = match pipeline::build_path(std::path::Path::new(&path), vocab, optimise) {
        Ok(b) => b,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::FAILURE;
        }
    };
    if !built.messages.is_empty() {
        print!("{}", built.messages);
    }

    match &built.program {
        Some(p) => {
            if show_middle {
                print!("{}", polite_ir::print(p));
            }
            if built.messages.is_empty() {
                println!("All good — {} looks right to me.", path);
            }
            ExitCode::SUCCESS
        }
        None => ExitCode::FAILURE,
    }
}

fn command_check_vocabulary(vocab: &Vocabulary) -> ExitCode {
    let conflicts = vocab.conflicts();
    let (everyday, working, full) = vocab.tier_counts();
    if conflicts.is_empty() {
        println!(
            "The vocabulary is clear: {} phrases, and no two of them could ever match the same \
             sentence.",
            vocab.phrases().len()
        );
        println!("  everyday {everyday} · working {working} · full {full}");
        ExitCode::SUCCESS
    } else {
        println!(
            "There {} {} place{} where I would have to guess:\n",
            if conflicts.len() == 1 { "is" } else { "are" },
            conflicts.len(),
            if conflicts.len() == 1 { "" } else { "s" }
        );
        for c in &conflicts {
            let a = vocab.phrase(c.a);
            let b = vocab.phrase(c.b);
            println!("  vocabulary lines {} and {}:", a.line, b.line);
            println!("    {}\n", c.why);
        }
        println!(
            "The language must never guess which phrase you meant, so one of each pair needs \
             different words."
        );
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::is_yes;

    #[test]
    fn only_the_whole_word_agrees_to_hidden_text() {
        for said in ["yes", "Yes", "YES", "  yes  ", "yes
"] {
            assert!(is_yes(said), "{said:?} should have been taken as a yes");
        }
        // Everything else is a no, including the things people type without reading.
        for said in ["y", "Y", "", "
", "ok", "sure", "yeah", "yes please", "no", "n"] {
            assert!(!is_yes(said), "{said:?} should not have agreed to anything");
        }
    }
}
