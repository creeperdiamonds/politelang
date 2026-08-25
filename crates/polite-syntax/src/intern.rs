//! Every name becomes a number, once.
//!
//! Spec 10.1: comparing words is the most common thing a compiler does, so words are interned on
//! first sight and compared as `u32` afterwards.
//!
//! Names in PoliteLang are case-insensitive. `Score` and `score` are the same name, which removes
//! a whole class of beginner confusion for no cost.

use std::collections::HashMap;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Sym(pub u32);

#[derive(Default)]
pub struct Interner {
    map: HashMap<Box<str>, Sym>,
    names: Vec<Box<str>>,
}

impl Interner {
    pub fn new() -> Self {
        Interner::default()
    }

    pub fn intern(&mut self, word: &str) -> Sym {
        let lower = word.to_ascii_lowercase();
        if let Some(s) = self.map.get(lower.as_str()) {
            return *s;
        }
        let sym = Sym(self.names.len() as u32);
        let boxed: Box<str> = lower.as_str().into();
        self.names.push(boxed.clone());
        self.map.insert(boxed, sym);
        sym
    }

    pub fn get(&self, word: &str) -> Option<Sym> {
        self.map.get(word.to_ascii_lowercase().as_str()).copied()
    }

    pub fn text(&self, sym: Sym) -> &str {
        &self.names[sym.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}
