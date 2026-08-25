//! `polite grammar` — the editor grammar, generated from the vocabulary table.
//!
//! Spec 12: highlighting is generated, always in sync, never hand-edited. Adding a word to
//! `core.vocab` colours it in the editor without anybody touching a grammar file.

use polite_vocab::Vocabulary;
use std::process::ExitCode;

pub fn write(args: &[String], vocab: &Vocabulary) -> ExitCode {
    let path = args
        .first()
        .cloned()
        .unwrap_or_else(|| "vscode-politelang/syntaxes/politelang.tmLanguage.json".to_string());

    let json = build(vocab);
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&path, json) {
        Ok(()) => {
            println!("Grammar written to {path}, from {} phrases.", vocab.phrases().len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("I could not write {path}: {e}");
            ExitCode::FAILURE
        }
    }
}

pub fn build(vocab: &Vocabulary) -> String {
    // Every literal word in the language, longest first so that multi-word phrases win.
    let mut words: Vec<&str> = Vec::new();
    for p in vocab.phrases() {
        for w in &p.literals {
            if !words.contains(&&**w) {
                words.push(w);
            }
        }
    }
    words.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));

    let vocabulary = words.join("|");

    // The words that carry the structure of the language get their own colour.
    let courtesy = "please|kindly|thanks|thank you|would you|if you would|be so kind|for repeating|for checking|for defining|for trying|for that";
    let structure = "define|otherwise|check if|see whether|when|try to|give back|hand back|answer with|share|use|borrow|with|and|or|not|is|is not|is over|is under|is at least|is at most|is between";
    let literals = "yes|no|nothing";

    format!(
        r##"{{
  "$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
  "name": "PoliteLang",
  "scopeName": "source.polite",
  "_generated": "by `polite grammar` from vocabulary/core.vocab — do not edit by hand",
  "patterns": [
    {{ "include": "#comments" }},
    {{ "include": "#text" }},
    {{ "include": "#courtesy" }},
    {{ "include": "#structure" }},
    {{ "include": "#literals" }},
    {{ "include": "#numbers" }},
    {{ "include": "#vocabulary" }}
  ],
  "repository": {{
    "comments": {{
      "patterns": [
        {{ "name": "comment.line.double-dash.polite", "match": "--.*$" }},
        {{ "name": "comment.line.by-the-way.polite", "match": "(?i)^\\s*by the way.*$" }}
      ]
    }},
    "text": {{
      "name": "string.quoted.double.polite",
      "begin": "\"",
      "end": "\"",
      "patterns": [
        {{ "name": "constant.character.escape.polite", "match": "\\\\." }},
        {{ "name": "variable.other.interpolated.polite", "match": "\\{{[^}}]*\\}}" }}
      ]
    }},
    "courtesy": {{
      "name": "keyword.control.courtesy.polite",
      "match": "(?i)\\b({courtesy})\\b"
    }},
    "structure": {{
      "name": "keyword.control.polite",
      "match": "(?i)\\b({structure})\\b"
    }},
    "literals": {{
      "name": "constant.language.polite",
      "match": "(?i)\\b({literals})\\b"
    }},
    "numbers": {{
      "name": "constant.numeric.polite",
      "match": "\\b[0-9]+(\\.[0-9]+)?\\b"
    }},
    "vocabulary": {{
      "name": "support.function.polite",
      "match": "(?i)\\b({vocabulary})\\b"
    }}
  }}
}}
"##
    )
}
