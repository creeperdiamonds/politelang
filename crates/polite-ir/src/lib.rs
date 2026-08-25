//! PoliteIR — the middle language.
//!
//! Spec 9.3: small, boring, regular. Basic blocks with numbered slots, types attached to every
//! value, and about thirty instructions. Every one of the six ways to write a loop produces the
//! same shape in here, which is what makes a large vocabulary cheap: the front of the language
//! can grow forever while the back stays this size.
//!
//! Spec 9.4: backends consume PoliteIR and never the sentence tree.

#![forbid(unsafe_code)]

pub mod lower;
pub mod optimise;

use polite_diag::Diagnostic;

pub type Slot = u32;
pub type BlockId = u32;
pub type FuncId = u32;

/// What a slot holds. Decided once, at lowering, so the runtime never asks (spec 10.2).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Ty {
    Whole,
    Fraction,
    Decimal,
    Complex,
    Text,
    YesNo,
    List,
    Lookup,
    Nothing,
}

/// Standard-library operations. Keeping these behind one instruction is what holds the
/// instruction set to a size a backend author can hold in their head.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Builtin {
    Show,
    AskWhole,
    AskDecimal,
    AskText,
    AskYesNo,

    NewList,
    ListItem,
    ListCount,
    ListFirst,
    ListLast,
    ListAppend,
    ListPutAt,
    ListRemoveAt,
    ListSum,
    ListBiggest,
    ListSmallest,
    ListSorted,
    ListReversed,
    ListJoin,
    ListContains,
    ListPosition,

    NewLookup,
    LookupGet,
    LookupPut,
    LookupForget,
    LookupKeys,
    LookupHas,

    TextLength,
    TextCapitals,
    TextSmall,
    TextTrimmed,
    TextWords,
    TextSplit,
    TextNumber,
    TextOf,
    TextStartsWith,
    TextEndsWith,
    TextContains,

    RandomRange,
    Rounded,
    Absolute,
    SquareRoot,
    DividesEvenly,
    DivideNumbers,

    FileContents,
    FileWrite,
    FileAppend,
    FileExists,
    TimeNow,
    WaitFor,

    TextSlice,
    TextReplace,
    TextLetter,
    TextLetters,
    TextRepeated,
    IsEmpty,

    Remainder,
    Smaller,
    Larger,
    Power,
    RoundedDown,
    RoundedUp,

    SaveCanvas,
    PutInWindow,
    DotSize,

    OpenCanvas,
    ClearCanvas,
    PaintPoint,
    DrawLine,
    DrawBox,
    FillBox,
    DrawCircle,
    RevealCanvas,
    RevealLetters,
    MakeColour,
    NamedColour,
    CanvasWidth,
    CanvasHeight,
    ColourAt,

    IsPrime,
    PrimeFactors,
    Divisors,
    PowerWithin,
    InverseWithin,
    WaysToChoose,
    WaysToArrange,

    InBinary,
    InHexadecimal,
    InBase,
    ValueOfInBase,

    BitwiseAnd,
    BitwiseOr,
    BitwiseExclusiveOr,
    BitwiseNot,
    ShiftedLeft,
    ShiftedRight,

    Mode,
    Variance,
    Correlation,

    PairwiseSum,
    PairwiseProduct,
    DotProduct,
    CrossProduct,
    Magnitude,
    ScaledBy,

    MatrixProduct,
    Transpose,
    Determinant,
    MatrixInverse,
    IdentityMatrix,

    MakeFraction,
    FractionTop,
    FractionBottom,
    AsFraction,
    AsDecimal,
    AsWholeNumber,
    WholeNumberIn,

    ImaginaryNumber,
    RealPart,
    ImaginaryPart,
    Conjugate,
    Direction,
    ComplexSquareRoot,

    Pi,
    EulerE,

    Sine,
    Cosine,
    Tangent,
    ArcSine,
    ArcCosine,
    ArcTangent,
    AngleOver,
    ToDegrees,
    ToRadians,

    HyperbolicSine,
    HyperbolicCosine,
    HyperbolicTangent,

    NaturalLogarithm,
    CommonLogarithm,
    LogarithmInBase,
    Exponential,

    CubeRoot,
    Squared,
    Cubed,

    WholePart,
    FractionPart,
    Sign,
    RoundedTo,
    KeptBetween,

    GreatestCommonFactor,
    SmallestCommonMultiple,
    Factorial,

    Median,
    Spread,
    AsPercentageOf,
    PercentOf,

    ListRest,
    ListFirstFew,
    ListAverage,
    ListCountIn,
    LookupCount,
}

impl Builtin {
    /// Whether this one might not work out (spec 7.5).
    pub fn might_not_work_out(self) -> bool {
        matches!(
            self,
            Builtin::ListItem
                | Builtin::ListFirst
                | Builtin::ListLast
                | Builtin::ListBiggest
                | Builtin::ListSmallest
                | Builtin::ListPosition
                | Builtin::ListPutAt
                | Builtin::ListRemoveAt
                | Builtin::LookupGet
                | Builtin::TextNumber
                | Builtin::SquareRoot
                | Builtin::DivideNumbers
                | Builtin::FileContents
                | Builtin::FileWrite
                | Builtin::FileAppend
                | Builtin::TextLetter
                | Builtin::Remainder
                | Builtin::ListAverage
                | Builtin::ArcSine
                | Builtin::ArcCosine
                | Builtin::NaturalLogarithm
                | Builtin::CommonLogarithm
                | Builtin::LogarithmInBase
                | Builtin::SmallestCommonMultiple
                | Builtin::Factorial
                | Builtin::Median
                | Builtin::Spread
                | Builtin::AsPercentageOf
                | Builtin::MakeFraction
                | Builtin::WholeNumberIn
                | Builtin::PowerWithin
                | Builtin::InverseWithin
                | Builtin::WaysToChoose
                | Builtin::WaysToArrange
                | Builtin::InBase
                | Builtin::ValueOfInBase
                | Builtin::Mode
                | Builtin::Variance
                | Builtin::Correlation
                | Builtin::PairwiseSum
                | Builtin::PairwiseProduct
                | Builtin::DotProduct
                | Builtin::CrossProduct
                | Builtin::MatrixProduct
                | Builtin::Transpose
                | Builtin::Determinant
                | Builtin::MatrixInverse
                | Builtin::IdentityMatrix
                | Builtin::NamedColour
                | Builtin::SaveCanvas
                | Builtin::PutInWindow
        )
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Compare {
    Equal,
    NotEqual,
    Over,
    Under,
    AtLeast,
    AtMost,
}

/// Which specialised comparison to use. Chosen at lowering, never at runtime.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CmpKind {
    /// Numbers, compared exactly across the whole tower: a whole number of any size, a fraction,
    /// a decimal. Small whole numbers take a fast path and never leave it.
    Number,
    Decimal,
    Text,
    YesNo,
    /// Lists and lookups, compared item by item.
    Value,
}

#[derive(Clone, Debug)]
pub enum Instr {
    ConstWhole {
        dst: Slot,
        value: i64,
    },
    ConstDecimal {
        dst: Slot,
        value: f64,
    },
    ConstText {
        dst: Slot,
        text: u32,
    },
    ConstYesNo {
        dst: Slot,
        value: bool,
    },
    ConstNothing {
        dst: Slot,
    },
    Move {
        dst: Slot,
        src: Slot,
    },

    AddWhole {
        dst: Slot,
        a: Slot,
        b: Slot,
    },
    SubWhole {
        dst: Slot,
        a: Slot,
        b: Slot,
    },
    MulWhole {
        dst: Slot,
        a: Slot,
        b: Slot,
    },
    AddDecimal {
        dst: Slot,
        a: Slot,
        b: Slot,
    },
    SubDecimal {
        dst: Slot,
        a: Slot,
        b: Slot,
    },
    MulDecimal {
        dst: Slot,
        a: Slot,
        b: Slot,
    },
    /// Widening a whole number to a decimal one, decided at lowering.
    WholeToDecimal {
        dst: Slot,
        src: Slot,
    },
    /// Arithmetic where the kinds of number are not both plain, so the tower decides. Fractions
    /// and complex numbers come through here; whole numbers and decimals have their own.
    AddNumber {
        dst: Slot,
        a: Slot,
        b: Slot,
    },
    SubNumber {
        dst: Slot,
        a: Slot,
        b: Slot,
    },
    MulNumber {
        dst: Slot,
        a: Slot,
        b: Slot,
    },
    NegateNumber {
        dst: Slot,
        src: Slot,
    },
    NegateWhole {
        dst: Slot,
        src: Slot,
    },
    NegateDecimal {
        dst: Slot,
        src: Slot,
    },
    ConcatText {
        dst: Slot,
        a: Slot,
        b: Slot,
    },
    Cmp {
        dst: Slot,
        op: Compare,
        kind: CmpKind,
        a: Slot,
        b: Slot,
    },
    Not {
        dst: Slot,
        src: Slot,
    },

    /// A builtin that cannot fail.
    Call {
        dst: Option<Slot>,
        which: Builtin,
        args: Vec<Slot>,
    },
    /// A builtin that might not work out.
    ///
    /// On failure the reason goes into `reason` and control moves to `fail`. A `fail` of `None`
    /// means the failure leaves this action altogether, which is how spec 7.4 makes an action
    /// risky without anybody writing an annotation.
    TryCall {
        dst: Option<Slot>,
        which: Builtin,
        args: Vec<Slot>,
        reason: Slot,
        fail: Option<BlockId>,
    },
    /// Calling one of your own actions. Same failure rule as `TryCall`.
    CallAction {
        dst: Option<Slot>,
        func: FuncId,
        args: Vec<Slot>,
        reason: Slot,
        fail: Option<BlockId>,
    },

    /// `..., I am sure` turned out to be wrong. Stops, politely, saying so.
    StopBecauseSure {
        reason: Slot,
        what: u32,
    },

    /// `stop everything`. Ends the whole program, and no `try` can catch it — that is the point.
    StopEverything,

    Jump {
        to: BlockId,
    },
    Branch {
        cond: Slot,
        then_block: BlockId,
        else_block: BlockId,
    },
    Return {
        src: Option<Slot>,
    },
}

impl Instr {
    pub fn is_terminator(&self) -> bool {
        matches!(
            self,
            Instr::Jump { .. } | Instr::Branch { .. } | Instr::Return { .. }
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct Block {
    pub instrs: Vec<Instr>,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub param_slots: Vec<Slot>,
    pub slot_count: u32,
    pub blocks: Vec<Block>,
    pub entry: BlockId,
    /// Spec 7.4: whether a failure can leave this action.
    pub risky: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Program {
    pub funcs: Vec<Function>,
    pub texts: Vec<String>,
    pub main: FuncId,
}

impl Program {
    pub fn instruction_count(&self) -> usize {
        self.funcs
            .iter()
            .flat_map(|f| f.blocks.iter())
            .map(|b| b.instrs.len())
            .sum()
    }
}

/// Spec 9.4: the socket every future backend plugs into.
///
/// Nothing above this crate knows a backend exists. The reference runner in `polite-run` is the
/// first implementation; native, JVM and WebAssembly are later ones, and none of them will ever
/// see a `please` or a `thank you`.
pub trait Backend {
    fn name(&self) -> &str;
    fn emit(&mut self, program: &Program) -> Result<(), Diagnostic>;
}

/// A readable dump of the middle language, for `polite check --show-middle` and for tests.
pub fn print(program: &Program) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for (i, f) in program.funcs.iter().enumerate() {
        let _ = writeln!(
            out,
            "action {} ({} slots{})",
            if f.name.is_empty() {
                "<the file>".to_string()
            } else {
                f.name.clone()
            },
            f.slot_count,
            if f.risky { ", might not work out" } else { "" }
        );
        let _ = i;
        for (b, block) in f.blocks.iter().enumerate() {
            let _ = writeln!(out, "  block {b}:");
            for instr in &block.instrs {
                let _ = writeln!(out, "    {}", show_instr(instr, program));
            }
        }
        out.push('\n');
    }
    out
}

fn show_instr(instr: &Instr, p: &Program) -> String {
    let slots = |v: &Vec<Slot>| {
        v.iter()
            .map(|s| format!("%{s}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    match instr {
        Instr::ConstWhole { dst, value } => format!("%{dst} = {value}"),
        Instr::ConstDecimal { dst, value } => format!("%{dst} = {value}"),
        Instr::ConstText { dst, text } => format!("%{dst} = {:?}", p.texts[*text as usize]),
        Instr::ConstYesNo { dst, value } => {
            format!("%{dst} = {}", if *value { "yes" } else { "no" })
        }
        Instr::ConstNothing { dst } => format!("%{dst} = nothing"),
        Instr::Move { dst, src } => format!("%{dst} = %{src}"),
        Instr::AddWhole { dst, a, b } => format!("%{dst} = add.whole %{a}, %{b}"),
        Instr::SubWhole { dst, a, b } => format!("%{dst} = sub.whole %{a}, %{b}"),
        Instr::MulWhole { dst, a, b } => format!("%{dst} = mul.whole %{a}, %{b}"),
        Instr::AddDecimal { dst, a, b } => format!("%{dst} = add.decimal %{a}, %{b}"),
        Instr::SubDecimal { dst, a, b } => format!("%{dst} = sub.decimal %{a}, %{b}"),
        Instr::MulDecimal { dst, a, b } => format!("%{dst} = mul.decimal %{a}, %{b}"),
        Instr::WholeToDecimal { dst, src } => format!("%{dst} = widen %{src}"),
        Instr::AddNumber { dst, a, b } => format!("%{dst} = add.number %{a}, %{b}"),
        Instr::SubNumber { dst, a, b } => format!("%{dst} = sub.number %{a}, %{b}"),
        Instr::MulNumber { dst, a, b } => format!("%{dst} = mul.number %{a}, %{b}"),
        Instr::NegateNumber { dst, src } => format!("%{dst} = negate.number %{src}"),
        Instr::NegateWhole { dst, src } => format!("%{dst} = negate.whole %{src}"),
        Instr::NegateDecimal { dst, src } => format!("%{dst} = negate.decimal %{src}"),
        Instr::ConcatText { dst, a, b } => format!("%{dst} = concat.text %{a}, %{b}"),
        Instr::Cmp {
            dst,
            op,
            kind,
            a,
            b,
        } => format!("%{dst} = {op:?}.{kind:?} %{a}, %{b}"),
        Instr::Not { dst, src } => format!("%{dst} = not %{src}"),
        Instr::Call { dst, which, args } => match dst {
            Some(d) => format!("%{d} = {which:?}({})", slots(args)),
            None => format!("{which:?}({})", slots(args)),
        },
        Instr::TryCall {
            dst,
            which,
            args,
            fail,
            ..
        } => {
            let target = match fail {
                Some(b) => format!("block {b}"),
                None => "out of this action".to_string(),
            };
            match dst {
                Some(d) => format!("%{d} = try {which:?}({}) else {target}", slots(args)),
                None => format!("try {which:?}({}) else {target}", slots(args)),
            }
        }
        Instr::CallAction {
            dst,
            func,
            args,
            fail,
            ..
        } => {
            let name = &p.funcs[*func as usize].name;
            let target = match fail {
                Some(b) => format!(" else block {b}"),
                None => String::new(),
            };
            match dst {
                Some(d) => format!("%{d} = call {name}({}){target}", slots(args)),
                None => format!("call {name}({}){target}", slots(args)),
            }
        }
        Instr::StopBecauseSure { reason, what } => {
            format!("stop.sure %{reason}, {:?}", p.texts[*what as usize])
        }
        Instr::StopEverything => "stop everything".to_string(),
        Instr::Jump { to } => format!("jump block {to}"),
        Instr::Branch {
            cond,
            then_block,
            else_block,
        } => format!("branch %{cond}, block {then_block}, block {else_block}"),
        Instr::Return { src } => match src {
            Some(s) => format!("return %{s}"),
            None => "return".to_string(),
        },
    }
}
