//! Types, and working them out.
//!
//! Spec 6.2: you never write a type. The language works them out and checks them before the
//! program runs. This is a small Hindley–Milner style engine: type variables in a flat arena,
//! union-find with path compression, no `Rc` anywhere.
//!
//! One deliberate v1 simplification: whole and decimal numbers mix freely, and the result of
//! mixing them is a decimal. Text and numbers still do not mix, which is what catches the
//! mistake in spec section 8.

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct TyId(pub u32);

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TyKind {
    /// Not worked out yet. The number indexes the substitution.
    Var(u32),
    Whole,
    Decimal,
    Text,
    YesNo,
    Nothing,
    List(TyId),
    Lookup(TyId),
}

pub struct Types {
    kinds: Vec<TyKind>,
    subst: Vec<Option<TyId>>,
    /// Cached ids for the types with no arguments, so they are made once.
    pub whole: TyId,
    pub decimal: TyId,
    pub text: TyId,
    pub yes_no: TyId,
    pub nothing: TyId,
}

#[derive(Debug)]
pub struct Mismatch {
    pub left: TyId,
    pub right: TyId,
}

impl Default for Types {
    fn default() -> Self {
        Self::new()
    }
}

impl Types {
    pub fn new() -> Types {
        let mut kinds = Vec::with_capacity(64);
        kinds.push(TyKind::Whole);
        kinds.push(TyKind::Decimal);
        kinds.push(TyKind::Text);
        kinds.push(TyKind::YesNo);
        kinds.push(TyKind::Nothing);
        Types {
            kinds,
            subst: Vec::new(),
            whole: TyId(0),
            decimal: TyId(1),
            text: TyId(2),
            yes_no: TyId(3),
            nothing: TyId(4),
        }
    }

    pub fn kind(&self, id: TyId) -> TyKind {
        self.kinds[id.0 as usize]
    }

    fn add(&mut self, k: TyKind) -> TyId {
        self.kinds.push(k);
        TyId(self.kinds.len() as u32 - 1)
    }

    pub fn fresh(&mut self) -> TyId {
        self.subst.push(None);
        let v = self.subst.len() as u32 - 1;
        self.add(TyKind::Var(v))
    }

    pub fn list_of(&mut self, item: TyId) -> TyId {
        self.add(TyKind::List(item))
    }

    pub fn lookup_of(&mut self, value: TyId) -> TyId {
        self.add(TyKind::Lookup(value))
    }

    /// Follow variable bindings until something concrete, or an unbound variable.
    pub fn resolve(&mut self, id: TyId) -> TyId {
        match self.kind(id) {
            TyKind::Var(v) => match self.subst[v as usize] {
                Some(next) => {
                    let root = self.resolve(next);
                    self.subst[v as usize] = Some(root); // path compression
                    root
                }
                None => id,
            },
            _ => id,
        }
    }

    pub fn is_unknown(&mut self, id: TyId) -> bool {
        let r = self.resolve(id);
        matches!(self.kind(r), TyKind::Var(_))
    }

    pub fn is_number(&mut self, id: TyId) -> bool {
        let r = self.resolve(id);
        matches!(self.kind(r), TyKind::Whole | TyKind::Decimal)
    }

    /// The wider of two number types, so mixing whole and decimal gives a decimal.
    pub fn widen(&mut self, a: TyId, b: TyId) -> TyId {
        let a = self.resolve(a);
        let b = self.resolve(b);
        match (self.kind(a), self.kind(b)) {
            (TyKind::Decimal, _) | (_, TyKind::Decimal) => self.decimal,
            _ => self.whole,
        }
    }

    pub fn unify(&mut self, a: TyId, b: TyId) -> Result<(), Mismatch> {
        let ra = self.resolve(a);
        let rb = self.resolve(b);
        if ra == rb {
            return Ok(());
        }
        match (self.kind(ra), self.kind(rb)) {
            (TyKind::Var(v), _) => {
                if self.occurs(v, rb) {
                    return Err(Mismatch {
                        left: ra,
                        right: rb,
                    });
                }
                self.subst[v as usize] = Some(rb);
                Ok(())
            }
            (_, TyKind::Var(v)) => {
                if self.occurs(v, ra) {
                    return Err(Mismatch {
                        left: ra,
                        right: rb,
                    });
                }
                self.subst[v as usize] = Some(ra);
                Ok(())
            }
            // Numbers mix.
            (TyKind::Whole, TyKind::Decimal) | (TyKind::Decimal, TyKind::Whole) => Ok(()),
            (TyKind::List(x), TyKind::List(y)) => self.unify(x, y),
            (TyKind::Lookup(x), TyKind::Lookup(y)) => self.unify(x, y),
            (x, y) if x == y => Ok(()),
            _ => Err(Mismatch {
                left: ra,
                right: rb,
            }),
        }
    }

    fn occurs(&mut self, var: u32, inside: TyId) -> bool {
        let r = self.resolve(inside);
        match self.kind(r) {
            TyKind::Var(v) => v == var,
            TyKind::List(x) | TyKind::Lookup(x) => self.occurs(var, x),
            _ => false,
        }
    }

    /// How a type is named to a person. Always a noun phrase that fits in a sentence.
    pub fn describe(&mut self, id: TyId) -> String {
        let r = self.resolve(id);
        match self.kind(r) {
            TyKind::Var(_) => "something I have not worked out yet".to_string(),
            TyKind::Whole => "a whole number".to_string(),
            TyKind::Decimal => "a decimal number".to_string(),
            TyKind::Text => "text".to_string(),
            TyKind::YesNo => "a yes or no".to_string(),
            TyKind::Nothing => "nothing".to_string(),
            TyKind::List(x) => {
                let inner = self.describe(x);
                if inner.starts_with("something") {
                    "a list".to_string()
                } else {
                    format!("a list of {inner}")
                }
            }
            TyKind::Lookup(x) => {
                let inner = self.describe(x);
                if inner.starts_with("something") {
                    "a lookup".to_string()
                } else {
                    format!("a lookup holding {inner}")
                }
            }
        }
    }

    /// A settled type for the middle language: every remaining unknown becomes `nothing`.
    pub fn settle(&mut self, id: TyId) -> TyId {
        let r = self.resolve(id);
        match self.kind(r) {
            TyKind::Var(_) => self.nothing,
            _ => r,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_variable_takes_the_shape_of_what_it_meets() {
        let mut t = Types::new();
        let v = t.fresh();
        t.unify(v, t.text).unwrap();
        let r = t.resolve(v);
        assert_eq!(t.kind(r), TyKind::Text);
    }

    #[test]
    fn text_and_numbers_do_not_mix() {
        let mut t = Types::new();
        assert!(t.unify(t.text, t.whole).is_err());
    }

    #[test]
    fn whole_and_decimal_numbers_do_mix() {
        let mut t = Types::new();
        assert!(t.unify(t.whole, t.decimal).is_ok());
        let w = t.whole;
        let d = t.decimal;
        let wd = t.widen(w, d);
        assert_eq!(t.kind(wd), TyKind::Decimal);
    }

    #[test]
    fn lists_unify_through_their_items() {
        let mut t = Types::new();
        let v = t.fresh();
        let a = t.list_of(v);
        let b = t.list_of(t.text);
        t.unify(a, b).unwrap();
        let r = t.resolve(v);
        assert_eq!(t.kind(r), TyKind::Text);
    }

    #[test]
    fn a_list_cannot_hold_itself() {
        let mut t = Types::new();
        let v = t.fresh();
        let l = t.list_of(v);
        assert!(t.unify(v, l).is_err());
    }

    #[test]
    fn types_are_named_in_plain_english() {
        let mut t = Types::new();
        let l = t.list_of(t.text);
        assert_eq!(t.describe(l), "a list of text");
        assert_eq!(t.describe(t.yes_no), "a yes or no");
    }
}
