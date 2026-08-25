//! The phrase table.
//!
//! Spec section 4.1: the vocabulary is data, not code. Nobody hand-writes parser code per phrase.
//! Adding a word to PoliteLang is adding a row to `vocabulary/core.vocab`.
//!
//! Spec section 10.1: matching a phrase is a lookup, not a search. Phrases are indexed by their
//! first word, so the cost of matching does not grow with the size of the vocabulary. That claim
//! is load-bearing for the whole project and is benchmarked (`polite bench`).

#![forbid(unsafe_code)]

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Forms
// ---------------------------------------------------------------------------

macro_rules! forms {
    ($($variant:ident => $name:literal),* $(,)?) => {
        /// The meaning a phrase lowers to.
        ///
        /// Many phrases share one form. Six ways to loop, one loop in the middle language: that
        /// is what makes a large vocabulary cheap (spec 9.1, V + N rather than V * N).
        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        pub enum Form { $($variant),* }

        impl Form {
            pub fn from_name(s: &str) -> Option<Form> {
                match s { $($name => Some(Form::$variant),)* _ => None }
            }
            pub fn name(self) -> &'static str {
                match self { $(Form::$variant => $name,)* }
            }
            pub const ALL: &'static [Form] = &[$(Form::$variant),*];
        }
    };
}

forms! {
    Show => "show",
    AskAs => "ask_as",
    Remember => "remember",
    Assign => "assign",
    AddTo => "add_to",
    TakeFrom => "take_from",
    MultiplyBy => "multiply_by",
    DivideBy => "divide_by",

    LoopCount => "loop_count",
    LoopWhile => "loop_while",
    LoopUntil => "loop_until",
    LoopForever => "loop_forever",
    LoopEach => "loop_each",
    LoopRange => "loop_range",
    StopRepeating => "stop_repeating",
    SkipOne => "skip_one",
    Check => "check",
    GiveBack => "give_back",

    EmptyList => "empty_list",
    ItemOf => "item_of",
    CountOf => "count_of",
    FirstItem => "first_item",
    LastItem => "last_item",
    SumOf => "sum_of",
    BiggestOf => "biggest_of",
    SmallestOf => "smallest_of",
    SortedOf => "sorted_of",
    ReverseOf => "reverse_of",
    JoinOf => "join_of",
    ContainsItem => "contains_item",
    PositionOf => "position_of",
    PutAt => "put_at",
    RemoveAt => "remove_at",

    EmptyLookup => "empty_lookup",
    ValueFor => "value_for",
    KeysOf => "keys_of",
    HasKey => "has_key",
    PutFor => "put_for",
    ForgetKey => "forget_key",

    LengthOf => "length_of",
    CapitalsOf => "capitals_of",
    SmallOf => "small_of",
    TrimmedOf => "trimmed_of",
    WordsIn => "words_in",
    SplitOf => "split_of",
    NumberIn => "number_in",
    TextOf => "text_of",
    StartsWith => "starts_with",
    EndsWith => "ends_with",
    ContainsText => "contains_text",

    RandomRange => "random_range",
    RoundedOf => "rounded_of",
    AbsoluteOf => "absolute_of",
    SquareRootOf => "square_root_of",
    DividesEvenly => "divides_evenly",

    ContentsOf => "contents_of",
    WriteFile => "write_file",
    FileExists => "file_exists",
    TimeNow => "time_now",

    UseModule => "use_module",
    Share => "share",
}

// ---------------------------------------------------------------------------
// Rows
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Kind {
    /// A request that does something.
    Stmt,
    /// A request that opens a block, closed by `thank you` / `thanks`.
    Block,
    /// A phrase that produces a value.
    Expr,
}

impl Kind {
    fn from_name(s: &str) -> Option<Kind> {
        match s {
            "stmt" => Some(Kind::Stmt),
            "block" => Some(Kind::Block),
            "expr" => Some(Kind::Expr),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Kind::Stmt => "stmt",
            Kind::Block => "block",
            Kind::Expr => "expr",
        }
    }
}

/// Spec 4.2: nobody learns 400 words. They learn 40, and the rest arrives when they go looking.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Tier {
    Everyday,
    Working,
    Full,
}

impl Tier {
    fn from_name(s: &str) -> Option<Tier> {
        match s {
            "everyday" => Some(Tier::Everyday),
            "working" => Some(Tier::Working),
            "full" => Some(Tier::Full),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Tier::Everyday => "everyday",
            Tier::Working => "working",
            Tier::Full => "full",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    /// A literal word that must appear.
    Word(Box<str>),
    /// A gap the parser fills in.
    Hole {
        label: Box<str>,
        /// `{name}` and `{var}` holes take a plain name; every other hole takes an expression.
        takes_name: bool,
    },
}

#[derive(Clone, Debug)]
pub struct Phrase {
    pub kind: Kind,
    pub tier: Tier,
    pub form: Form,
    pub pieces: Vec<Piece>,
    /// Spec 7.5: whether this operation might not work out.
    pub risky: bool,
    /// Line in the vocabulary file, so problems point at the right row.
    pub line: u32,
    /// The pattern exactly as written, for documentation and messages.
    pub pattern: Box<str>,
    /// Just the literal words, in order. Used by the ambiguity check.
    pub literals: Vec<Box<str>>,
}

impl Phrase {
    /// An infix phrase begins with a hole: `{list} contains {value}`.
    pub fn is_infix(&self) -> bool {
        matches!(self.pieces.first(), Some(Piece::Hole { .. }))
    }

    /// The word this phrase is indexed under.
    ///
    /// For a normal phrase that is its first word. For an infix phrase it is the first literal
    /// word after the leading hole, which is what the parser sees once it has read the left side.
    pub fn key_word(&self) -> Option<&str> {
        self.pieces.iter().find_map(|p| match p {
            Piece::Word(w) => Some(&**w),
            _ => None,
        })
    }

    pub fn literal_count(&self) -> usize {
        self.literals.len()
    }

    pub fn hole_count(&self) -> usize {
        self.pieces
            .iter()
            .filter(|p| matches!(p, Piece::Hole { .. }))
            .count()
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LoadProblem {
    pub line: u32,
    pub message: String,
}

impl std::fmt::Display for LoadProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "vocabulary line {}: {}", self.line, self.message)
    }
}

pub struct Vocabulary {
    phrases: Vec<Phrase>,
    /// Phrases that begin with a literal word, indexed by that word.
    by_word: HashMap<Box<str>, Vec<u32>>,
    /// Phrases that begin with a hole, indexed by their first literal word.
    infix_by_word: HashMap<Box<str>, Vec<u32>>,
    explanations: HashMap<Form, Box<str>>,
    /// Every literal word in the language, for "did you mean" suggestions.
    words: Vec<Box<str>>,
}

impl Vocabulary {
    /// The vocabulary shipped with the compiler.
    pub fn embedded() -> Vocabulary {
        let text = include_str!("../../../vocabulary/core.vocab");
        match Vocabulary::load(text) {
            Ok(v) => v,
            Err(problems) => {
                // The shipped table is checked by a test, so reaching here means the binary was
                // built from a broken table. Say so plainly rather than limping on.
                let mut msg = String::from("the built-in vocabulary could not be read:\n");
                for p in problems {
                    msg.push_str(&format!("  {p}\n"));
                }
                panic!("{msg}");
            }
        }
    }

    pub fn load(text: &str) -> Result<Vocabulary, Vec<LoadProblem>> {
        let mut phrases: Vec<Phrase> = Vec::with_capacity(256);
        let mut explanations: HashMap<Form, Box<str>> = HashMap::new();
        let mut problems: Vec<LoadProblem> = Vec::new();

        for (i, raw) in text.lines().enumerate() {
            let line = i as u32 + 1;
            let row = raw.trim();
            if row.is_empty() || row.starts_with('#') {
                continue;
            }

            let mut parts = row.split("::").map(str::trim);
            let head = parts.next().unwrap_or("");
            let pattern = parts.next().unwrap_or("");
            let extra = parts.next();
            if parts.next().is_some() {
                problems.push(LoadProblem {
                    line,
                    message: "this row has more than three parts separated by `::`".into(),
                });
                continue;
            }

            let head_words: Vec<&str> = head.split_whitespace().collect();

            // An explanation row: `explain <form> :: <English>`
            if head_words.first() == Some(&"explain") {
                if head_words.len() != 2 {
                    problems.push(LoadProblem {
                        line,
                        message: "an explain row reads `explain <form> :: <English>`".into(),
                    });
                    continue;
                }
                match Form::from_name(head_words[1]) {
                    Some(f) => {
                        if explanations.insert(f, pattern.into()).is_some() {
                            problems.push(LoadProblem {
                                line,
                                message: format!("`{}` is explained twice", head_words[1]),
                            });
                        }
                    }
                    None => problems.push(LoadProblem {
                        line,
                        message: format!("there is no form called `{}`", head_words[1]),
                    }),
                }
                continue;
            }

            if head_words.len() != 3 {
                problems.push(LoadProblem {
                    line,
                    message: "a phrase row reads `<kind> <tier> <form> :: <pattern>`".into(),
                });
                continue;
            }

            let kind = match Kind::from_name(head_words[0]) {
                Some(k) => k,
                None => {
                    problems.push(LoadProblem {
                        line,
                        message: format!(
                            "`{}` is not a kind. Use stmt, block or expr.",
                            head_words[0]
                        ),
                    });
                    continue;
                }
            };
            let tier = match Tier::from_name(head_words[1]) {
                Some(t) => t,
                None => {
                    problems.push(LoadProblem {
                        line,
                        message: format!(
                            "`{}` is not a tier. Use everyday, working or full.",
                            head_words[1]
                        ),
                    });
                    continue;
                }
            };
            let form = match Form::from_name(head_words[2]) {
                Some(f) => f,
                None => {
                    problems.push(LoadProblem {
                        line,
                        message: format!(
                            "there is no form called `{}`. Forms live in polite-vocab.",
                            head_words[2]
                        ),
                    });
                    continue;
                }
            };

            let risky = match extra {
                None => false,
                Some("risky") => true,
                Some(other) => {
                    problems.push(LoadProblem {
                        line,
                        message: format!("`{other}` is not something a row can say. Only `risky`."),
                    });
                    continue;
                }
            };

            let (pieces, literals) = match parse_pattern(pattern) {
                Ok(v) => v,
                Err(m) => {
                    problems.push(LoadProblem { line, message: m });
                    continue;
                }
            };

            if literals.is_empty() {
                problems.push(LoadProblem {
                    line,
                    message: "a pattern needs at least one real word in it".into(),
                });
                continue;
            }

            phrases.push(Phrase {
                kind,
                tier,
                form,
                pieces,
                risky,
                line,
                pattern: pattern.into(),
                literals,
            });
        }

        if !problems.is_empty() {
            return Err(problems);
        }

        // Longest patterns first, so the most specific phrase wins a match.
        phrases.sort_by(|a, b| b.literal_count().cmp(&a.literal_count()));

        let mut by_word: HashMap<Box<str>, Vec<u32>> = HashMap::new();
        let mut infix_by_word: HashMap<Box<str>, Vec<u32>> = HashMap::new();
        let mut words: Vec<Box<str>> = Vec::new();

        for (i, p) in phrases.iter().enumerate() {
            let key: Box<str> = match p.key_word() {
                Some(w) => w.into(),
                None => continue,
            };
            if p.is_infix() {
                infix_by_word.entry(key).or_default().push(i as u32);
            } else {
                by_word.entry(key).or_default().push(i as u32);
            }
            for w in &p.literals {
                if !words.contains(w) {
                    words.push(w.clone());
                }
            }
        }

        Ok(Vocabulary {
            phrases,
            by_word,
            infix_by_word,
            explanations,
            words,
        })
    }

    pub fn phrases(&self) -> &[Phrase] {
        &self.phrases
    }

    pub fn phrase(&self, id: u32) -> &Phrase {
        &self.phrases[id as usize]
    }

    /// Candidate phrases beginning with this word. The lookup that does not grow (spec 10.1).
    pub fn starting_with(&self, word: &str) -> &[u32] {
        self.by_word.get(word).map(|v| &v[..]).unwrap_or(&[])
    }

    /// Candidate infix phrases whose first literal word is this one.
    pub fn infix_with(&self, word: &str) -> &[u32] {
        self.infix_by_word.get(word).map(|v| &v[..]).unwrap_or(&[])
    }

    pub fn explanation(&self, form: Form) -> Option<&str> {
        self.explanations.get(&form).map(|s| &**s)
    }

    pub fn is_risky(&self, form: Form) -> bool {
        self.phrases.iter().any(|p| p.form == form && p.risky)
    }

    /// Every phrase that uses this form, so `polite explain` can list the ways to say it.
    pub fn ways_to_say(&self, form: Form) -> Vec<&Phrase> {
        let mut v: Vec<&Phrase> = self.phrases.iter().filter(|p| p.form == form).collect();
        v.sort_by_key(|p| p.line);
        v
    }

    /// Spec 4.1: typo help is free, because the table already holds every word.
    pub fn nearest_word(&self, word: &str) -> Option<&str> {
        let lower = word.to_ascii_lowercase();
        let limit = match lower.chars().count() {
            0..=3 => 1,
            4..=7 => 2,
            _ => 3,
        };
        let mut best: Option<(usize, &str)> = None;
        for candidate in &self.words {
            let d = edit_distance(&lower, candidate);
            if d <= limit && best.map_or(true, |(bd, _)| d < bd) {
                best = Some((d, candidate));
            }
        }
        best.map(|(_, w)| w)
    }

    /// Spec 4.3: the ambiguity check. Two phrases conflict when one could swallow the other.
    ///
    /// The rule is deliberately conservative: within a kind, if two phrases are indexed under the
    /// same word and one literal skeleton is a prefix of the other, the shorter one could match
    /// wherever the longer one does. That is exactly the situation where the language would have
    /// to guess, and it must never guess.
    pub fn conflicts(&self) -> Vec<Conflict> {
        let mut out = Vec::new();
        for (a, pa) in self.phrases.iter().enumerate() {
            for (b, pb) in self.phrases.iter().enumerate().skip(a + 1) {
                if pa.kind != pb.kind || pa.is_infix() != pb.is_infix() {
                    continue;
                }
                if pa.key_word() != pb.key_word() {
                    continue;
                }
                let (short, long) = if pa.literals.len() <= pb.literals.len() {
                    (pa, pb)
                } else {
                    (pb, pa)
                };
                if long.literals.starts_with(&short.literals) {
                    out.push(Conflict {
                        a: a as u32,
                        b: b as u32,
                        why: if short.literals.len() == long.literals.len() {
                            format!(
                                "`{}` and `{}` use the same words in the same order",
                                short.pattern, long.pattern
                            )
                        } else {
                            format!(
                                "`{}` would also match anywhere `{}` does",
                                short.pattern, long.pattern
                            )
                        },
                    });
                }
            }
        }
        out
    }

    pub fn tier_counts(&self) -> (usize, usize, usize) {
        let mut c = (0, 0, 0);
        for p in &self.phrases {
            match p.tier {
                Tier::Everyday => c.0 += 1,
                Tier::Working => c.1 += 1,
                Tier::Full => c.2 += 1,
            }
        }
        c
    }
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub a: u32,
    pub b: u32,
    pub why: String,
}

fn parse_pattern(pattern: &str) -> Result<(Vec<Piece>, Vec<Box<str>>), String> {
    let mut pieces = Vec::new();
    let mut literals = Vec::new();
    for token in pattern.split_whitespace() {
        if let Some(inner) = token.strip_prefix('{') {
            let label = match inner.strip_suffix('}') {
                Some(l) => l,
                None => return Err(format!("`{token}` opens a hole but never closes it")),
            };
            if label.is_empty() {
                return Err("a hole needs a label inside its braces".into());
            }
            let takes_name = matches!(label, "name" | "var");
            pieces.push(Piece::Hole {
                label: label.into(),
                takes_name,
            });
        } else {
            if token.contains('{') || token.contains('}') {
                return Err(format!("`{token}` mixes a word and a hole"));
            }
            let lower = token.to_ascii_lowercase();
            pieces.push(Piece::Word(lower.as_str().into()));
            literals.push(lower.as_str().into());
        }
    }
    Ok((pieces, literals))
}

/// Levenshtein distance, iterative with a single row.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_vocabulary_loads() {
        let v = Vocabulary::embedded();
        assert!(v.phrases().len() > 60, "expected a real vocabulary");
    }

    /// Spec 4.3. If this ever fails, the language would have to guess, and it must not.
    #[test]
    fn the_shipped_vocabulary_is_unambiguous() {
        let v = Vocabulary::embedded();
        let conflicts = v.conflicts();
        assert!(
            conflicts.is_empty(),
            "the vocabulary has phrases that could both match the same sentence:\n{}",
            conflicts
                .iter()
                .map(|c| format!("  {}\n", c.why))
                .collect::<String>()
        );
    }

    /// Spec 4.1: every row is documented, so `polite explain` always has an answer.
    #[test]
    fn every_form_in_use_is_explained() {
        let v = Vocabulary::embedded();
        let mut missing = Vec::new();
        for p in v.phrases() {
            if v.explanation(p.form).is_none() {
                missing.push(p.form.name());
            }
        }
        missing.sort_unstable();
        missing.dedup();
        assert!(missing.is_empty(), "forms with no explanation: {missing:?}");
    }

    #[test]
    fn holes_and_words_are_told_apart() {
        let (pieces, literals) = parse_pattern("remember {name} is {value}").unwrap();
        assert_eq!(literals.len(), 2);
        assert!(matches!(&pieces[1], Piece::Hole { takes_name: true, .. }));
        assert!(matches!(&pieces[3], Piece::Hole { takes_name: false, .. }));
    }

    #[test]
    fn a_swallowing_phrase_is_reported() {
        let v = Vocabulary::load(
            "stmt everyday show :: put {value}\nstmt everyday put_for :: put {value} for {key} in {name}\n",
        )
        .unwrap();
        assert_eq!(v.conflicts().len(), 1);
    }

    #[test]
    fn a_typo_finds_its_word() {
        let v = Vocabulary::embedded();
        assert_eq!(v.nearest_word("dispay"), Some("display"));
        assert_eq!(v.nearest_word("rememer"), Some("remember"));
    }

    #[test]
    fn infix_phrases_are_indexed_on_their_first_real_word() {
        let v = Vocabulary::embedded();
        assert!(!v.infix_with("contains").is_empty());
        assert!(v.starting_with("contains").is_empty());
    }
}
