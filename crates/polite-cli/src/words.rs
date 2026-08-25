//! `polite words` and `polite explain`.
//!
//! Spec 4.4: discovery is a command, not a manual. A four-hundred-word language documents itself
//! or it does not get documented, and both of these read straight off the vocabulary table.

use polite_vocab::{Kind, Tier, Vocabulary};
use std::process::ExitCode;

pub fn list(args: &[String], vocab: &Vocabulary) -> ExitCode {
    let mut topic: Option<String> = None;
    let mut tier: Option<Tier> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "about" | "--about" => {
                topic = args.get(i + 1).cloned();
                i += 2;
            }
            "--tier" => {
                tier = match args.get(i + 1).map(|s| s.as_str()) {
                    Some("everyday") => Some(Tier::Everyday),
                    Some("working") => Some(Tier::Working),
                    Some("full") => Some(Tier::Full),
                    other => {
                        eprintln!(
                            "I do not know the tier `{}`. Try everyday, working or full.",
                            other.unwrap_or("")
                        );
                        return ExitCode::FAILURE;
                    }
                };
                i += 2;
            }
            other => {
                topic = Some(other.to_string());
                i += 1;
            }
        }
    }

    let needle = topic.as_ref().map(|t| t.to_ascii_lowercase());
    let mut shown = 0usize;

    for want in [Tier::Everyday, Tier::Working, Tier::Full] {
        if let Some(only) = tier {
            if only != want {
                continue;
            }
        }
        let mut rows: Vec<&polite_vocab::Phrase> = vocab
            .phrases()
            .iter()
            .filter(|p| p.tier == want)
            .filter(|p| match &needle {
                None => true,
                Some(n) => {
                    p.pattern.to_ascii_lowercase().contains(n)
                        || vocab
                            .explanation(p.form)
                            .map(|e| e.to_ascii_lowercase().contains(n))
                            .unwrap_or(false)
                        || p.form.name().contains(n)
                }
            })
            .collect();
        if rows.is_empty() {
            continue;
        }
        rows.sort_by(|a, b| a.line.cmp(&b.line));

        println!("\n{}", heading(want));
        for p in rows {
            shown += 1;
            let mark = match p.kind {
                Kind::Stmt => " ",
                Kind::Block => ":",
                Kind::Expr => "=",
            };
            let risky = if p.risky { "   (might not work out)" } else { "" };
            println!("  {mark} {}{}", p.pattern, risky);
        }
    }

    if shown == 0 {
        match &topic {
            Some(t) => println!("I do not know any words about `{t}` yet."),
            None => println!("The vocabulary is empty, which should not happen."),
        }
        return ExitCode::FAILURE;
    }

    println!(
        "\n  {} phrases shown.   `:` opens a block   `=` gives back a value",
        shown
    );
    println!("  `polite explain \"<phrase>\"` says what one means.");
    ExitCode::SUCCESS
}

fn heading(tier: Tier) -> &'static str {
    match tier {
        Tier::Everyday => "EVERYDAY — the words to start with",
        Tier::Working => "WORKING — for real programs",
        Tier::Full => "FULL — the specialist end",
    }
}

pub fn explain(args: &[String], vocab: &Vocabulary) -> ExitCode {
    let wanted = args.join(" ").trim().trim_matches('"').to_ascii_lowercase();
    if wanted.is_empty() {
        eprintln!("Which phrase would you like explained?");
        return ExitCode::FAILURE;
    }

    // Look for a phrase whose words contain what was asked about.
    let mut best: Option<&polite_vocab::Phrase> = None;
    for p in vocab.phrases() {
        let pattern = p.pattern.to_ascii_lowercase();
        if pattern == wanted {
            best = Some(p);
            break;
        }
        if pattern.contains(&wanted) && best.is_none() {
            best = Some(p);
        }
    }
    if best.is_none() {
        for p in vocab.phrases() {
            if p.literals.iter().any(|w| **w == *wanted) {
                best = Some(p);
                break;
            }
        }
    }

    let phrase = match best {
        Some(p) => p,
        None => {
            match vocab.nearest_word(&wanted) {
                Some(near) => println!(
                    "I do not know `{wanted}`. Did you mean `{near}`?\n\nTry: polite explain \"{near}\""
                ),
                None => println!("I do not know `{wanted}`. `polite words` lists everything I do."),
            }
            return ExitCode::FAILURE;
        }
    };

    println!("\n  {}\n", phrase.pattern);
    if let Some(text) = vocab.explanation(phrase.form) {
        for line in wrap(text, 70) {
            println!("  {line}");
        }
    }

    let ways = vocab.ways_to_say(phrase.form);
    if ways.len() > 1 {
        println!("\n  Other ways to say the same thing:");
        for w in ways {
            if w.pattern != phrase.pattern {
                println!("    {}", w.pattern);
            }
        }
    }

    if phrase.risky {
        println!(
            "\n  This one might not work out, so you have to say what happens if it does not:"
        );
        println!("    ... or <something to fall back on>");
    }

    println!(
        "\n  Tier: {}   ({})",
        phrase.tier.name(),
        match phrase.kind {
            Kind::Stmt => "a request",
            Kind::Block => "opens a block",
            Kind::Expr => "gives back a value",
        }
    );
    ExitCode::SUCCESS
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
