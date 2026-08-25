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

/// What a word means, in both worlds at once.
///
/// The first purpose of this language is that you come away knowing more English than you started
/// with, so every word gets explained twice: what the language does with it, and what it actually
/// means in ordinary English, with its part of speech. Neither half is decoration.
pub fn explain(args: &[String], vocab: &Vocabulary) -> ExitCode {
    let wanted = args.join(" ").trim().trim_matches('"').to_ascii_lowercase();
    if wanted.is_empty() {
        eprintln!("Which word would you like explained?");
        return ExitCode::FAILURE;
    }

    let (phrase, role) = resolve(vocab, &wanted);
    let has_english = !vocab.english(&wanted).is_empty();

    if phrase.is_none() && !has_english && role.is_none() {
        match vocab.nearest_word(&wanted) {
            Some(near) => println!(
                "\nI do not know `{wanted}`. Did you mean `{near}`?\n\n  polite explain {near}\n"
            ),
            None => println!(
                "\nI do not know `{wanted}`. `polite words` lists everything I do.\n"
            ),
        }
        return ExitCode::FAILURE;
    }

    // ---- the heading -------------------------------------------------------
    println!();
    match phrase {
        Some(p) if role.is_none() => println!("  {}", p.pattern),
        Some(p) => println!("  {wanted}   (and: {})", p.pattern),
        None => println!("  {wanted}"),
    }

    // ---- what the language does with it ------------------------------------
    rule("IN POLITELANG");

    // What the word does for the shape of a sentence, if anything.
    if let Some(text) = role {
        for line in wrap(text, 66) {
            println!("  {line}");
        }
        if phrase.is_some() {
            println!();
        }
    }

    match phrase {
        Some(p) => {
            if let Some(text) = vocab.explanation(p.form) {
                for line in wrap(text, 66) {
                    println!("  {line}");
                }
                println!();
            }
            println!(
                "  {}, from the {} vocabulary.",
                match p.kind {
                    Kind::Stmt => "A request",
                    Kind::Block => "A request that opens a block",
                    Kind::Expr => "A phrase that gives back a value",
                },
                p.tier.name()
            );
            if p.risky {
                println!();
                for line in wrap(
                    "This one might not work out, so you have to say what happens if it does not \
                     — with `or something`, or inside a `try to`.",
                    66,
                ) {
                    println!("  {line}");
                }
            }
            if p.tight {
                println!();
                for line in wrap(
                    "This follows a value and belongs to the value right beside it, so `2 plus 3 \
                     squared` is eleven rather than twenty five.",
                    66,
                ) {
                    println!("  {line}");
                }
            }
            let ways = vocab.ways_to_say(p.form);
            if ways.len() > 1 {
                println!("\n  Other ways to say the very same thing:");
                for w in ways {
                    if w.pattern != p.pattern {
                        println!("    {}", w.pattern);
                    }
                }
            }
        }
        None => {
            if role.is_none() {
                for line in wrap(
                    "This word is not one the language acts on by itself. It reads naturally \
                     inside a phrase and means nothing on its own.",
                    66,
                ) {
                    println!("  {line}");
                }
            }
        }
    }

    // ---- and what it means in English --------------------------------------
    rule("IN ENGLISH");
    let mut shown: Vec<String> = Vec::new();
    let words: Vec<String> = match phrase {
        // Every real word of the phrase, in the order it is written.
        Some(p) => p.literals.iter().map(|w| w.to_string()).collect(),
        None => vec![wanted.clone()],
    };
    for word in words {
        if shown.contains(&word) {
            continue;
        }
        let senses = vocab.english(&word);
        if senses.is_empty() {
            continue;
        }
        shown.push(word.clone());
        println!("  {word}");
        for sense in senses {
            let lines = wrap(&sense.meaning, 52);
            for (i, line) in lines.iter().enumerate() {
                if i == 0 {
                    println!("    {:<14}{}", sense.part.name(), line);
                } else {
                    println!("    {:<14}{}", "", line);
                }
            }
        }
        println!();
    }
    if shown.is_empty() {
        println!("  Nothing written down for this one yet.\n");
    }

    ExitCode::SUCCESS
}

fn rule(title: &str) {
    let width = 66usize;
    let dashes = width.saturating_sub(title.chars().count() + 6);
    println!();
    println!("  ── {title} {}", "─".repeat(dashes));
    println!();
}

/// Everything there is to say about a word: what it does to a sentence, and which phrase it
/// belongs to.
///
/// A word can be both. `to` holds a sentence together *and* leads `to the power of`, and where
/// both are true both are worth saying, so neither is made to win. A word the parser uses itself
/// is never explained as whatever phrase happens to contain it, because `and` is far better
/// explained as what it does than as the greatest common factor.
fn resolve<'a>(
    vocab: &'a Vocabulary,
    wanted: &str,
) -> (Option<&'a polite_vocab::Phrase>, Option<&'static str>) {
    let role = structural_role(wanted);
    let phrase = find_phrase(vocab, wanted).or_else(|| {
        if role.is_some() {
            None
        } else {
            find_phrase_mentioning(vocab, wanted)
        }
    });
    (phrase, role)
}

/// The phrase somebody most likely meant.
///
/// Matched on whole words rather than on letters, so looking up `to` does not land you in
/// `the total of {list}`.
fn find_phrase<'a>(vocab: &'a Vocabulary, wanted: &str) -> Option<&'a polite_vocab::Phrase> {
    let asked: Vec<&str> = wanted.split_whitespace().collect();
    if asked.is_empty() {
        return None;
    }

    // Written out in full.
    for p in vocab.phrases() {
        if p.pattern.to_ascii_lowercase() == wanted {
            return Some(p);
        }
    }

    // The phrase opens with exactly these words.
    for p in vocab.phrases() {
        if p.literals.len() >= asked.len()
            && p.literals
                .iter()
                .zip(asked.iter())
                .all(|(have, want)| &**have == *want)
        {
            return Some(p);
        }
    }

    None
}

/// Any phrase that holds these words somewhere, in order. The last resort.
///
/// Only reached for a word the parser does not use itself, because `and` is far better explained
/// as what it does to a sentence than as whichever phrase happens to contain one.
fn find_phrase_mentioning<'a>(
    vocab: &'a Vocabulary,
    wanted: &str,
) -> Option<&'a polite_vocab::Phrase> {
    let asked: Vec<&str> = wanted.split_whitespace().collect();
    vocab.phrases().iter().find(|p| {
        let mut left = asked.iter().peekable();
        for word in &p.literals {
            if let Some(next) = left.peek() {
                if &**word == **next {
                    left.next();
                }
            }
        }
        left.peek().is_none()
    })
}

/// What the words the parser itself uses are for.
///
/// These never appear in the phrase table, because they are the structure rather than the
/// vocabulary — but somebody looking one up deserves an answer just the same.
fn structural_role(word: &str) -> Option<&'static str> {
    Some(match word {
        "please" | "kindly" => {
            "A courtesy word. It opens a request. `please`, `kindly`, `would you`, `if you would \
             be so kind` and the rest all mean exactly the same thing, and the language never \
             treats one differently from another — they are there so that anybody who finds \
             `please` too pleading has another way to be polite."
        }
        "thank" | "thanks" => {
            "Closes a block. This takes the place of brackets, of `end`, and of meaningful \
             indentation. It may say what it is closing — `thank you for repeating` — and the \
             language checks it if you do."
        }
        "otherwise" => {
            "Offers an alternative to a check, or the other half of a `try to`. On its own it is \
             the last word on the matter; followed by `if`, it asks another question first."
        }
        "define" => {
            "Teaches the language a word of your own. Whatever you define becomes a word you can \
             use anywhere afterwards, and its name may be several words long."
        }
        "with" => "Introduces the values an action needs, both where it is defined and where it is used.",
        "and" => {
            "Joins two things. Inside an argument list it separates one argument from the next, \
             which is the one rule you have to hold on to."
        }
        "or" => {
            "Offers another possibility. After something that might not work out it is not a \
             choice at all but a fallback: it says what to use instead."
        }
        "not" => "Turns a yes into a no, and a no into a yes.",
        "is" => {
            "Compares two things. On its own it asks whether they are the same; followed by \
             `not`, `over`, `under`, `at least`, `at most` or `between`, it asks something else."
        }
        "try" => {
            "Opens a block for something that might not work out, paired with `otherwise if it \
             does not work out`."
        }
        "yes" | "no" => "One of the two settled answers. Every question the language asks comes back as one of these.",
        "nothing" => "The absence of a value. Nothing at all.",
        "plus" | "minus" | "times" => "Arithmetic, in words. The symbols + - * mean exactly the same.",
        "divided" | "by" => "Part of `divided by`, which shares one number into another.",
        "then" => "Joins values together as text: `\"hello, \" then name` is the word form of putting a value inside braces.",
        "sure" | "am" => "Part of `, I am sure`, where you take responsibility for something that might not work out.",
        "what" | "went" | "wrong" => "Part of `{what went wrong}`, the plain English reason available inside the other half of a `try to`.",
        "a" | "an" | "the" => "An article. It reads naturally and means nothing to the language, which is exactly why it is allowed.",
        "would" | "you" | "if" | "be" | "so" | "kind" | "too" | "much" | "trouble" | "it" => {
            "Part of one of the longer courtesy openers, such as `would you please` or `if you \
             would be so kind`. Every one of them means the same as a plain `please`."
        }
        "i" => "Used in `, I am sure`, where you vouch for something yourself.",
        "as" => {
            "Names something. `ask \"...\" as guess` gives the answer a name, and `{value} as a              fraction` says what to turn a number into."
        }
        "to" => {
            "Points at where something is going: `change score to 5`, `add 1 to score`, `from 1              to 10`, `try to`."
        }
        "for" | "that" => "Used after `thank you` to say what is being closed, as in `thank you for repeating`.",
        "over" | "under" | "at" | "least" | "most" | "between" => {
            "Part of a comparison: `is over`, `is under`, `is at least`, `is at most`, `is \
             between one and ten`."
        }
        "does" | "work" | "out" => "Part of `otherwise if it does not work out`, the other half of a `try to`.",
        _ => return None,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every word the parser itself uses has something said about it.
    ///
    /// These never appear in the phrase table, so nothing else would catch it if one were
    /// forgotten — and a word somebody can type but not look up is a small broken promise.
    #[test]
    fn every_structural_word_has_a_role() {
        let structural = [
            "please", "kindly", "would", "you", "if", "be", "so", "kind", "as", "to", "it", "is",
            "not", "too", "much", "trouble", "thank", "thanks", "for", "that", "otherwise",
            "define", "with", "and", "or", "try", "does", "work", "out", "what", "went", "wrong",
            "yes", "no", "nothing", "plus", "minus", "times", "divided", "by", "over", "under",
            "at", "least", "most", "between", "i", "am", "sure", "then", "a", "an", "the",
        ];
        let missing: Vec<&str> = structural
            .iter()
            .copied()
            .filter(|w| structural_role(w).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "these words can be typed but not looked up: {missing:?}"
        );
    }

    /// Both halves really are there for every word in the language.
    #[test]
    fn every_word_can_be_explained_in_both_worlds() {
        let vocab = Vocabulary::embedded();
        let mut without_english: Vec<String> = Vec::new();
        let mut without_a_home: Vec<String> = Vec::new();

        for phrase in vocab.phrases() {
            for word in &phrase.literals {
                if vocab.english(word).is_empty() && !without_english.contains(&word.to_string()) {
                    without_english.push(word.to_string());
                }
                let (phrase, role) = resolve(&vocab, word);
                if phrase.is_none() && role.is_none() && !without_a_home.contains(&word.to_string())
                {
                    without_a_home.push(word.to_string());
                }
            }
        }

        assert!(
            without_english.is_empty(),
            "used but never explained in English: {without_english:?}"
        );
        assert!(
            without_a_home.is_empty(),
            "used but nothing said about what the language does with them: {without_a_home:?}"
        );
    }

    /// A word with more than one sense gives the one the language uses first.
    #[test]
    fn the_sense_the_language_uses_comes_first() {
        let vocab = Vocabulary::embedded();
        for (word, wanted) in [
            ("show", polite_vocab::PartOfSpeech::Verb),
            ("list", polite_vocab::PartOfSpeech::Noun),
            ("please", polite_vocab::PartOfSpeech::Adverb),
            ("kindly", polite_vocab::PartOfSpeech::Adverb),
            ("kind", polite_vocab::PartOfSpeech::Adjective),
            ("count", polite_vocab::PartOfSpeech::Verb),
        ] {
            let senses = vocab.english(word);
            assert!(!senses.is_empty(), "{word} has no English meaning");
            assert_eq!(senses[0].part, wanted, "the first sense of {word} is wrong");
        }
    }
}
