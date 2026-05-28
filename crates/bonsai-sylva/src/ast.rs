//! Sylva AST — abstract syntax tree for the Sylva language.

use serde::{Deserialize, Serialize};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SylvaType {
    Unknown,    // gradual typing: type not yet known
    Nil,
    Bool,
    Int,
    Float,
    Str,
    List(Box<SylvaType>),
    Map(Box<SylvaType>, Box<SylvaType>),
    Fn(Vec<SylvaType>, Box<SylvaType>),
    Named(String),
    Tuple(Vec<SylvaType>),
    DataFrame,
    NDArray,
    Option(Box<SylvaType>),
    Union(Vec<SylvaType>),   // gradual union
}

impl Default for SylvaType {
    fn default() -> Self { Self::Unknown }
}

// ── Expressions ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expr {
    pub kind: ExprKind,
    pub line: usize,
}

impl Expr {
    pub fn new(kind: ExprKind, line: usize) -> Self { Self { kind, line } }
    pub fn at(kind: ExprKind) -> Self { Self { kind, line: 0 } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExprKind {
    // Literals
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Tuple(Vec<Expr>),

    // Variables and access
    Var(String),
    Field(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),

    // Operators
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    UnOp(UnOp, Box<Expr>),

    // Control flow
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    While(Box<Expr>, Box<Expr>),
    For(String, Box<Expr>, Box<Expr>),
    Match(Box<Expr>, Vec<MatchArm>),

    // Functions
    Fn(FnDef),
    Call(Box<Expr>, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    Return(Option<Box<Expr>>),
    Break(Option<Box<Expr>>),
    Continue,

    // Binding
    Let { name: String, ty: Option<SylvaType>, value: Box<Expr>, mutable: bool },
    Assign(Box<Expr>, Box<Expr>),
    Block(Vec<Expr>),

    // Pipeline: expr |> fn
    Pipe(Box<Expr>, Box<Expr>),

    // Actor concurrency
    Spawn(Box<Expr>),            // spawn(actor_fn)
    Send(Box<Expr>, Box<Expr>),  // send(actor_ref, msg)
    Receive,                     // receive()
    Await(Box<Expr>),

    // Struct construction
    Struct(String, Vec<(String, Expr)>),

    // Type annotation
    TypeAscription(Box<Expr>, SylvaType),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinOp {
    Add, Sub, Mul, Div, Rem,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    Concat,     // ++
    Pipe,       // |>  (also handled as ExprKind::Pipe)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnOp { Neg, Not }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnDef {
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub ret_ty: Option<SylvaType>,
    pub body: Box<Expr>,
    pub is_async: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub ty: Option<SylvaType>,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Pattern {
    Wildcard,
    Nil,
    Bool(bool),
    Int(i64),
    Str(String),
    Bind(String),
    Tuple(Vec<Pattern>),
    List(Vec<Pattern>, Option<Box<Pattern>>),  // head patterns + optional rest bind
    Struct(String, Vec<(String, Pattern)>),
    Or(Vec<Pattern>),
}

// ── Module ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SylvaModule {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Item {
    FnDef(FnDef),
    LetDef { name: String, ty: Option<SylvaType>, value: Expr },
    Import(Vec<String>),
    Export(Vec<String>),
    TypeDef(String, SylvaType),
    StructDef { name: String, fields: Vec<(String, SylvaType)> },
    EnumDef  { name: String, variants: Vec<EnumVariant> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub payload: Option<SylvaType>,
}
