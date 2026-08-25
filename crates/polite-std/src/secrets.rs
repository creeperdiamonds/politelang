//! Secrets kept outside the program.
//!
//! A token, a password, a key: things a program needs and must not contain. `the secret called
//! "..."` reads one from the surroundings instead, which is what lets a program be shown to
//! somebody, or published, without handing them the keys to it.
//!
//! Two places are looked in, in this order:
//!
//!   1. the environment the program was started in,
//!   2. a file called `.env` in the folder it is being run from.
//!
//! The environment wins, so a secret handed over on the command line beats one written in a file —
//! which is what you want when you are trying something once, or when a real machine somewhere
//! sets it properly and the file is only there for working at home.
//!
//! Only the folder the program is run from is looked in, and no parent of it. Somewhere further up
//! the disk quietly supplying a secret is exactly the kind of thing that is very hard to work out
//! when it goes wrong.
//!
//! ## The file
//!
//! ```text
//! # Lines like this are ignored, and so are blank ones.
//! DISCORD_TOKEN=abc123
//! export ALSO_FINE=xyz
//! QUOTED="a value with spaces, and a # that stays"
//! ```
//!
//! An unquoted value has whitespace trimmed and anything after a `#` taken off. A quoted one is
//! kept exactly, `#` and all, and `\n` inside double quotes becomes a new line. There is no
//! substituting one secret into another: a value is what it says it is.

use std::collections::BTreeMap;

/// Everything a `.env` file says.
pub fn read_env_file(text: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            continue;
        }
        found.insert(name.to_string(), unquote(value.trim()));
    }
    found
}

fn unquote(value: &str) -> String {
    let mut letters = value.chars();
    match (letters.next(), value.chars().last()) {
        (Some('"'), Some('"')) if value.len() >= 2 => {
            // Inside double quotes, the handful of escapes people expect.
            let inner = &value[1..value.len() - 1];
            let mut out = String::with_capacity(inner.len());
            let mut left = inner.chars();
            while let Some(ch) = left.next() {
                if ch != '\\' {
                    out.push(ch);
                    continue;
                }
                match left.next() {
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                }
            }
            out
        }
        (Some('\''), Some('\'')) if value.len() >= 2 => value[1..value.len() - 1].to_string(),
        _ => {
            // Unquoted: a `#` starts a remark, and the rest is trimmed.
            match value.split_once('#') {
                Some((before, _)) => before.trim_end().to_string(),
                None => value.to_string(),
            }
        }
    }
}

/// The secret of this name, or a sentence saying why there is not one.
pub fn secret_called(name: &str) -> Result<String, String> {
    if let Ok(found) = std::env::var(name) {
        if !found.is_empty() {
            return Ok(found);
        }
    }
    if let Ok(text) = std::fs::read_to_string(".env") {
        if let Some(found) = read_env_file(&text).get(name) {
            return Ok(found.clone());
        }
        return Err(format!(
            "there is no secret called {name} here. It is not in the surroundings, and the .env \
             file in this folder does not mention it."
        ));
    }
    Err(format!(
        "there is no secret called {name} here. Set it before running, or put it in a file called \
         .env in this folder, and either way it stays out of the program where it belongs."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plain_shape_is_read() {
        let found = read_env_file("DISCORD_TOKEN=abc123\nOTHER=hello\n");
        assert_eq!(found.get("DISCORD_TOKEN").map(String::as_str), Some("abc123"));
        assert_eq!(found.get("OTHER").map(String::as_str), Some("hello"));
    }

    #[test]
    fn remarks_and_blank_lines_are_passed_over() {
        let found = read_env_file("# a remark\n\n  # an indented one\nA=1\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found.get("A").map(String::as_str), Some("1"));
    }

    #[test]
    fn export_in_front_is_allowed_because_people_write_it() {
        let found = read_env_file("export A=1\nexport   B=2\n");
        assert_eq!(found.get("A").map(String::as_str), Some("1"));
        assert_eq!(found.get("B").map(String::as_str), Some("2"));
    }

    #[test]
    fn a_quoted_value_keeps_everything_inside_the_quotes() {
        let found = read_env_file("A=\"spaces and a # inside\"\nB='single quoted'\n");
        assert_eq!(found.get("A").map(String::as_str), Some("spaces and a # inside"));
        assert_eq!(found.get("B").map(String::as_str), Some("single quoted"));
    }

    #[test]
    fn an_unquoted_value_stops_at_a_remark_and_is_trimmed() {
        let found = read_env_file("A=value   # what it is for\nB=  spaced  \n");
        assert_eq!(found.get("A").map(String::as_str), Some("value"));
        assert_eq!(found.get("B").map(String::as_str), Some("spaced"));
    }

    #[test]
    fn escapes_work_inside_double_quotes_and_not_outside_them() {
        let found = read_env_file("A=\"one\\ntwo\"\nB=one\\ntwo\nC='one\\ntwo'\n");
        assert_eq!(found.get("A").map(String::as_str), Some("one\ntwo"));
        // Outside double quotes a backslash is just a backslash.
        assert_eq!(found.get("B").map(String::as_str), Some("one\\ntwo"));
        assert_eq!(found.get("C").map(String::as_str), Some("one\\ntwo"));
    }

    #[test]
    fn a_value_may_itself_contain_an_equals_sign() {
        // Tokens and keys very often end in padding, and splitting on the last one would ruin them.
        let found = read_env_file("A=abc=def==\n");
        assert_eq!(found.get("A").map(String::as_str), Some("abc=def=="));
    }

    #[test]
    fn nonsense_lines_are_ignored_rather_than_guessed_at() {
        let found = read_env_file("no equals sign here\n=novalue\nBAD NAME=1\nGOOD=1\n");
        assert_eq!(found.len(), 1);
        assert!(found.contains_key("GOOD"));
    }

    #[test]
    fn an_empty_value_is_still_a_value() {
        let found = read_env_file("A=\nB=\"\"\n");
        assert_eq!(found.get("A").map(String::as_str), Some(""));
        assert_eq!(found.get("B").map(String::as_str), Some(""));
    }

    #[test]
    fn nothing_is_substituted_into_anything_else() {
        // A value is what it says it is; there is no second layer to reason about.
        let found = read_env_file("A=one\nB=$A/two\n");
        assert_eq!(found.get("B").map(String::as_str), Some("$A/two"));
    }
}
