//! The sentence tree.
//!
//! Spec 10.1: handles, not pointers. Every node lives in a flat array and is referred to by a
//! 32-bit index. There is no `Box`, no `Rc`, and no scattered object graph anywhere in here —
//! nodes sit next to each other in cache, which is worth several times both the memory and the
//! speed of the obvious design.

use crate::intern::{Interner, Sym};
use polite_diag::Span;
use polite_vocab::Form;

pub type ExprId = u32;
pub type StmtId = u32;
pub type BlockId = u32;
pub type ActionId = u32;

/// A run of items in one of the side arenas.
#[derive(Copy, Clone, Debug, Default)]
pub struct Range {
    pub start: u32,
    pub len: u32,
}

impl Range {
    pub const EMPTY: Range = Range { start: 0, len: 0 };
    pub fn is_empty(self) -> bool {
        self.len == 0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum BinOp {
    Add,
    /// `"hello, " then name then "!"` — the word form of dropping a value into text (spec 3.7).
    Then,
    Sub,
    Mul,
    Div,
    And,
    /// Either boolean `or`, or the fallback of spec 7.2. The checker decides which, because it is
    /// the only stage that knows whether the left side might not work out (spec 3.10).
    Or,
    Is,
    IsNot,
    Over,
    Under,
    AtLeast,
    AtMost,
}

impl BinOp {
    pub fn word(self) -> &'static str {
        match self {
            BinOp::Add => "plus",
            BinOp::Then => "then",
            BinOp::Sub => "minus",
            BinOp::Mul => "times",
            BinOp::Div => "divided by",
            BinOp::And => "and",
            BinOp::Or => "or",
            BinOp::Is => "is",
            BinOp::IsNot => "is not",
            BinOp::Over => "is over",
            BinOp::Under => "is under",
            BinOp::AtLeast => "is at least",
            BinOp::AtMost => "is at most",
        }
    }

    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinOp::Is | BinOp::IsNot | BinOp::Over | BinOp::Under | BinOp::AtLeast | BinOp::AtMost
        )
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum UnOp {
    Not,
    Negate,
}

#[derive(Copy, Clone, Debug)]
pub enum ExprKind {
    Int(i64),
    Dec(f64),
    /// Index into [`Ast::texts`].
    Text(u32),
    Yes,
    No,
    Nothing,
    Name(Sym),
    /// Text with values dropped into it: `"Got it in {guesses} guesses!"`.
    Interp(Range),
    Unary {
        op: UnOp,
        operand: ExprId,
    },
    Binary {
        op: BinOp,
        lhs: ExprId,
        rhs: ExprId,
    },
    Between {
        value: ExprId,
        low: ExprId,
        high: ExprId,
    },
    /// A phrase from the vocabulary table that produces a value.
    Phrase {
        form: Form,
        phrase: u32,
        args: Range,
    },
    /// Calling one of your own actions.
    Call {
        name: Sym,
        args: Range,
    },
    /// `..., I am sure` — spec 7.2, way three.
    Sure {
        value: ExprId,
    },
}

#[derive(Copy, Clone, Debug)]
pub struct ExprNode {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Copy, Clone, Debug)]
pub enum InterpPart {
    /// Index into [`Ast::texts`].
    Text(u32),
    Value(ExprId),
}

#[derive(Copy, Clone, Debug)]
pub struct CheckArm {
    /// `None` on the final `otherwise:` arm.
    pub cond: Option<ExprId>,
    pub body: BlockId,
    pub span: Span,
}

#[derive(Copy, Clone, Debug)]
pub enum StmtKind {
    /// Anything driven by the vocabulary table.
    Form {
        form: Form,
        phrase: u32,
        /// `{name}` and `{var}` holes, in order.
        names: Range,
        /// Expression holes, in order.
        args: Range,
        /// Present when the phrase opened a block.
        body: Option<BlockId>,
    },
    /// `check if ...:` with its `otherwise if ...:` and `otherwise:` arms.
    Check {
        arms: Range,
    },
    Define {
        action: ActionId,
    },
    /// `please:` — one courtesy word covering a run of actions (spec rule 3).
    Courtesy {
        body: BlockId,
    },
    /// `please try to:` ... `otherwise if it does not work out:` ... (spec 7.2, way two).
    Try {
        body: BlockId,
        otherwise: BlockId,
        /// The name the reason is bound to inside the otherwise arm, normally `what went wrong`.
        reason: Sym,
    },
    /// Calling one of your own actions as a statement.
    Call {
        name: Sym,
        args: Range,
    },
}

#[derive(Copy, Clone, Debug)]
pub struct StmtNode {
    pub kind: StmtKind,
    pub span: Span,
    /// Whether this statement was written with a courtesy word of its own.
    pub asked_politely: bool,
}

#[derive(Clone, Debug)]
pub struct Action {
    pub name: Sym,
    /// The words making up the name, so `load the score` can be matched at a call.
    pub words: Vec<Sym>,
    pub params: Range,
    pub body: BlockId,
    pub span: Span,
    pub shared: bool,
}

#[derive(Default)]
pub struct Ast {
    pub words: Interner,
    pub texts: Vec<String>,

    pub exprs: Vec<ExprNode>,
    pub stmts: Vec<StmtNode>,

    /// Blocks, as ranges into [`Ast::stmt_ids`].
    pub blocks: Vec<Range>,
    pub stmt_ids: Vec<StmtId>,

    pub args: Vec<ExprId>,
    pub names: Vec<Sym>,
    pub interp_parts: Vec<InterpPart>,
    pub check_arms: Vec<CheckArm>,
    pub actions: Vec<Action>,

    pub top: BlockId,
}

impl Ast {
    pub fn expr(&self, id: ExprId) -> &ExprNode {
        &self.exprs[id as usize]
    }
    pub fn stmt(&self, id: StmtId) -> &StmtNode {
        &self.stmts[id as usize]
    }
    pub fn block(&self, id: BlockId) -> &[StmtId] {
        let r = self.blocks[id as usize];
        &self.stmt_ids[r.start as usize..(r.start + r.len) as usize]
    }
    pub fn arg_slice(&self, r: Range) -> &[ExprId] {
        &self.args[r.start as usize..(r.start + r.len) as usize]
    }
    pub fn name_slice(&self, r: Range) -> &[Sym] {
        &self.names[r.start as usize..(r.start + r.len) as usize]
    }
    pub fn interp_slice(&self, r: Range) -> &[InterpPart] {
        &self.interp_parts[r.start as usize..(r.start + r.len) as usize]
    }
    pub fn arm_slice(&self, r: Range) -> &[CheckArm] {
        &self.check_arms[r.start as usize..(r.start + r.len) as usize]
    }
    pub fn text(&self, id: u32) -> &str {
        &self.texts[id as usize]
    }
    pub fn name_of(&self, sym: Sym) -> &str {
        self.words.text(sym)
    }

    pub fn push_expr(&mut self, kind: ExprKind, span: Span) -> ExprId {
        self.exprs.push(ExprNode { kind, span });
        self.exprs.len() as u32 - 1
    }

    pub fn push_stmt(&mut self, kind: StmtKind, span: Span, asked_politely: bool) -> StmtId {
        self.stmts.push(StmtNode {
            kind,
            span,
            asked_politely,
        });
        self.stmts.len() as u32 - 1
    }

    pub fn push_block(&mut self, ids: &[StmtId]) -> BlockId {
        let start = self.stmt_ids.len() as u32;
        self.stmt_ids.extend_from_slice(ids);
        self.blocks.push(Range {
            start,
            len: ids.len() as u32,
        });
        self.blocks.len() as u32 - 1
    }

    pub fn push_args(&mut self, ids: &[ExprId]) -> Range {
        let start = self.args.len() as u32;
        self.args.extend_from_slice(ids);
        Range {
            start,
            len: ids.len() as u32,
        }
    }

    pub fn push_names(&mut self, ids: &[Sym]) -> Range {
        let start = self.names.len() as u32;
        self.names.extend_from_slice(ids);
        Range {
            start,
            len: ids.len() as u32,
        }
    }

    pub fn push_interp(&mut self, parts: &[InterpPart]) -> Range {
        let start = self.interp_parts.len() as u32;
        self.interp_parts.extend_from_slice(parts);
        Range {
            start,
            len: parts.len() as u32,
        }
    }

    pub fn push_arms(&mut self, arms: &[CheckArm]) -> Range {
        let start = self.check_arms.len() as u32;
        self.check_arms.extend_from_slice(arms);
        Range {
            start,
            len: arms.len() as u32,
        }
    }

    /// Roughly how much memory the tree is holding, for the budget in spec 10.4.
    pub fn footprint_bytes(&self) -> usize {
        use std::mem::size_of;
        self.exprs.capacity() * size_of::<ExprNode>()
            + self.stmts.capacity() * size_of::<StmtNode>()
            + self.blocks.capacity() * size_of::<Range>()
            + self.stmt_ids.capacity() * 4
            + self.args.capacity() * 4
            + self.names.capacity() * 4
            + self.interp_parts.capacity() * size_of::<InterpPart>()
            + self.check_arms.capacity() * size_of::<CheckArm>()
            + self.texts.iter().map(|t| t.capacity()).sum::<usize>()
    }
}
