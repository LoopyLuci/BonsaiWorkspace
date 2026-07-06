//! Titan abstract syntax tree.

use crate::diag::Span;

#[derive(Debug, Clone)]
pub struct TypeRef {
    pub name: String,
    pub args: Vec<TypeRef>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Use,
    Struct(StructItem),
    Enum(EnumItem),
    Impl(ImplItem),
    Trait(TraitItem),
    Fn(FnItem),
    Const(ConstItem),
    Mod(ModItem),
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub is_self: bool,
}

#[derive(Debug, Clone)]
pub struct FnItem {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Option<Block>,
    /// Declared return type, when present (`-> T`). Used by `?` to determine
    /// the target error type for `From`-based conversion when the function
    /// returns `Result<_, E>` and the propagated error is a different type.
    pub ret: Option<TypeRef>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct StructItem {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub arity: usize,
    pub struct_fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EnumItem {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImplItem {
    pub target: String,
    pub methods: Vec<FnItem>,
    pub consts: Vec<ConstItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TraitItem {
    pub name: String,
    pub methods: Vec<FnItem>,
}

#[derive(Debug, Clone)]
pub struct ConstItem {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ModItem {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { pat: Pattern, init: Option<Expr>, span: Span },
    Expr { expr: Expr, semi: bool },
    Item(Box<Item>),
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Bind { name: String, span: Span },
    Wild,
    Lit(Box<Expr>),
    /// Range pattern: `1..=5` (inclusive) or `1..5` (exclusive). Either bound
    /// may be omitted (`..=5`, `1..`). Matches ints, floats, and chars.
    Range { lo: Option<Box<Expr>>, hi: Option<Box<Expr>>, inclusive: bool, span: Span },
    Tuple(Vec<Pattern>),
    Path { path: Vec<String>, span: Span },
    Enum { path: Vec<String>, elems: Vec<Pattern>, span: Span },
    Struct { path: Vec<String>, fields: Vec<(String, Pattern)>, span: Span },
    Ref(Box<Pattern>),
    Or(Vec<Pattern>),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pat: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int { v: i64, span: Span },
    Float { v: f64, span: Span },
    Str { v: String, span: Span },
    Char { v: char, span: Span },
    Bool { v: bool, span: Span },
    Path { segs: Vec<String>, span: Span },
    Field { obj: Box<Expr>, name: String, span: Span },
    Index { obj: Box<Expr>, index: Box<Expr>, span: Span },
    Call { callee: Box<Expr>, args: Vec<Expr>, span: Span },
    Method { recv: Box<Expr>, name: String, args: Vec<Expr>, span: Span },
    Unary { op: String, operand: Box<Expr>, span: Span },
    Binary { op: String, left: Box<Expr>, right: Box<Expr>, span: Span },
    Assign { op: String, target: Box<Expr>, value: Box<Expr>, span: Span },
    Range { from: Option<Box<Expr>>, to: Option<Box<Expr>>, inclusive: bool, span: Span },
    If { let_pat: Option<Box<Pattern>>, cond: Box<Expr>, then: Block, els: Option<Box<Expr>>, span: Span },
    Match { scrut: Box<Expr>, arms: Vec<MatchArm>, span: Span },
    While { let_pat: Option<Box<Pattern>>, cond: Box<Expr>, body: Block, label: Option<String>, span: Span },
    For { pat: Pattern, iter: Box<Expr>, body: Block, label: Option<String>, span: Span },
    Loop { body: Block, label: Option<String>, span: Span },
    BlockE { block: Block },
    Return { value: Option<Box<Expr>>, span: Span },
    Break { value: Option<Box<Expr>>, label: Option<String>, span: Span },
    Continue { label: Option<String>, span: Span },
    StructLit { path: Vec<String>, fields: Vec<(String, Expr)>, spread: Option<Box<Expr>>, span: Span },
    Array { elems: Vec<Expr>, repeat: Option<Box<Expr>>, span: Span },
    Tuple { elems: Vec<Expr>, span: Span },
    Closure { params: Vec<String>, body: Box<Expr>, span: Span },
    Try { expr: Box<Expr>, span: Span },
    Cast { expr: Box<Expr>, ty: TypeRef, span: Span },
    Macro { name: String, args: Vec<Expr>, repeat: Option<Box<Expr>>, span: Span },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int { span, .. }
            | Expr::Float { span, .. }
            | Expr::Str { span, .. }
            | Expr::Char { span, .. }
            | Expr::Bool { span, .. }
            | Expr::Path { span, .. }
            | Expr::Field { span, .. }
            | Expr::Index { span, .. }
            | Expr::Call { span, .. }
            | Expr::Method { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Assign { span, .. }
            | Expr::Range { span, .. }
            | Expr::If { span, .. }
            | Expr::Match { span, .. }
            | Expr::While { span, .. }
            | Expr::For { span, .. }
            | Expr::Loop { span, .. }
            | Expr::Return { span, .. }
            | Expr::Break { span, .. }
            | Expr::Continue { span, .. }
            | Expr::StructLit { span, .. }
            | Expr::Array { span, .. }
            | Expr::Tuple { span, .. }
            | Expr::Closure { span, .. }
            | Expr::Try { span, .. }
            | Expr::Cast { span, .. }
            | Expr::Macro { span, .. } => *span,
            Expr::BlockE { block } => block.span,
        }
    }
}
