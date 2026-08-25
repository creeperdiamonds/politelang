//! The `polite` command.

#![forbid(unsafe_code)]

use polite_cli::{bench, grammar, pipeline, words};

use polite_vocab::Vocabulary;
use std::process::ExitCode;

const USAGE: &str = "\
polite — the PoliteLang toolchain

  polite run <file.polite>        run a program
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
    let path = match args.first() {
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

    let mut world = polite_run::Terminal;
    match polite_run::run(&program, &mut world) {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            println!("\n{reason}");
            ExitCode::FAILURE
        }
    }
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
