//! Sylva AST — dynamically-typed, Python/JS-shaped: `class` (not struct+impl),
//! `def` functions with default/varargs/kwargs, comprehensions, f-strings,
//! try/except, lambda. No type annotations are required anywhere (Sylva is
//! dynamically typed by design — that's the point of absorbing Python/JS
//! capabilities rather than Rust/C's).

use crate::diag::Span;

#[derive(Debug, Clone)]
pub struct Module {
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default: Option<Expr>,
    pub is_vararg: bool,   // *args
    pub is_kwarg: bool,    // **kwargs
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub decorators: Vec<Expr>,
    pub is_async: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: String,
    pub bases: Vec<String>,
    pub methods: Vec<FnDef>,
    /// Class-body-level assignments (class variables), evaluated once at
    /// class-definition time and shared across instances unless shadowed.
    pub class_vars: Vec<(String, Expr)>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExceptClause {
    pub exc_type: Option<String>,
    pub bind: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    /// `name = expr` / `target.attr = expr` / `target[idx] = expr` (also
    /// covers augmented assignment `+=` etc., desugared by the parser).
    Assign { target: Expr, value: Expr, span: Span },
    Let { name: String, value: Option<Expr>, span: Span },
    If { branches: Vec<(Expr, Vec<Stmt>)>, orelse: Vec<Stmt>, span: Span },
    While { cond: Expr, body: Vec<Stmt>, orelse: Vec<Stmt>, span: Span },
    /// `target` holds one name (`for x in ...`) or several for tuple
    /// unpacking (`for k, v in d.items():`).
    For { target: Vec<String>, iter: Expr, body: Vec<Stmt>, orelse: Vec<Stmt>, span: Span },
    FnDef(FnDef),
    ClassDef(ClassDef),
    Return { value: Option<Expr>, span: Span },
    Break(Span),
    Continue(Span),
    Pass,
    Raise { exc: Option<Expr>, span: Span },
    Try { body: Vec<Stmt>, handlers: Vec<ExceptClause>, orelse: Vec<Stmt>, finally: Vec<Stmt>, span: Span },
    Import { path: String, alias: Option<String>, span: Span },
    Assert { cond: Expr, msg: Option<Expr>, span: Span },
    Global { names: Vec<String> },
    Del { target: Expr, span: Span },
    /// `layer`/`model`/`pipeline NAME { field: expr, ... }` — a declarative
    /// ML-pipeline config-block DSL used by the omni-integration specs
    /// (distinct from `class`: flat field/value pairs, no methods).
    ConfigBlock { kind: String, name: String, fields: Vec<(String, Expr)>, span: Span },
    /// `[pub] mod NAME { items }` — a Rust module; `body` holds its nested
    /// items exactly like `Module.body` (structs/fns/impls/nested mods).
    Mod { name: String, body: Vec<Stmt>, span: Span },
}

#[derive(Debug, Clone)]
pub enum FStrPart {
    Lit(String),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int { v: i64, span: Span },
    Float { v: f64, span: Span },
    Str { v: String, span: Span },
    FStr { parts: Vec<FStrPart>, span: Span },
    Bool { v: bool, span: Span },
    None_ { span: Span },
    Ident { name: String, span: Span },
    List { elems: Vec<Expr>, span: Span },
    Tuple { elems: Vec<Expr>, span: Span },
    Dict { entries: Vec<(Expr, Expr)>, span: Span },
    /// `[expr for target in iter if cond]` — `cond: None` if no filter.
    ListComp { expr: Box<Expr>, target: Vec<String>, iter: Box<Expr>, cond: Option<Box<Expr>>, span: Span },
    DictComp { key: Box<Expr>, value: Box<Expr>, target: Vec<String>, iter: Box<Expr>, cond: Option<Box<Expr>>, span: Span },
    BinOp { op: String, left: Box<Expr>, right: Box<Expr>, span: Span },
    UnaryOp { op: String, expr: Box<Expr>, span: Span },
    BoolOp { op: String, left: Box<Expr>, right: Box<Expr>, span: Span }, // and/or (short-circuit)
    Not { expr: Box<Expr>, span: Span },
    Compare { left: Box<Expr>, ops: Vec<String>, comparators: Vec<Expr>, span: Span },
    Call { func: Box<Expr>, args: Vec<Expr>, kwargs: Vec<(String, Expr)>, span: Span },
    Attr { obj: Box<Expr>, name: String, span: Span },
    Index { obj: Box<Expr>, index: Box<Expr>, span: Span },
    Slice { obj: Box<Expr>, lo: Option<Box<Expr>>, hi: Option<Box<Expr>>, step: Option<Box<Expr>>, span: Span },
    Lambda { params: Vec<Param>, body: Box<Expr>, span: Span },
    Ternary { body: Box<Expr>, cond: Box<Expr>, orelse: Box<Expr>, span: Span }, // `a if cond else b`
    Await { expr: Box<Expr>, span: Span },
    Yield { value: Option<Box<Expr>>, span: Span },
    /// `vec![value; count]` — a Rust-macro repeat-element list literal
    /// (`name!(...)` and `name![a, b]` reuse `Call`/`List` instead; only the
    /// `; count` shape needs a dedicated node).
    Repeat { value: Box<Expr>, count: Box<Expr>, span: Span },
    /// `match subject { Pat(bindings) => arm, .. }`. Patterns are reduced to
    /// (name, bound-identifiers) — `Some(e)`, `None`, `_` — with no real
    /// enum/variant matching underneath (this interpreter has no such
    /// value); an arm whose body is a statement rather than an expression
    /// (`None => return Vec::new()`) is parsed but its body replaced with
    /// `None_` (this is a parse-level-only construct, same tradeoff as
    /// `ast::TheoremBody::Structured` in the Axiom bootstrap).
    Match { subject: Box<Expr>, arms: Vec<(String, Vec<String>, Expr)>, span: Span },
    /// `expr?` — Rust's try-operator (propagate on error), and also used as
    /// a bare "optional slot" marker in some omni-integration list literals
    /// (e.g. `[system?, user(prompt)]`). This interpreter doesn't model
    /// Result/Option deeply, so it evaluates `inner` and passes its value
    /// through unchanged in both readings — a parse-level simplification.
    Try { inner: Box<Expr>, span: Span },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int { span, .. }
            | Expr::Float { span, .. }
            | Expr::Str { span, .. }
            | Expr::FStr { span, .. }
            | Expr::Bool { span, .. }
            | Expr::None_ { span }
            | Expr::Ident { span, .. }
            | Expr::List { span, .. }
            | Expr::Tuple { span, .. }
            | Expr::Dict { span, .. }
            | Expr::ListComp { span, .. }
            | Expr::DictComp { span, .. }
            | Expr::BinOp { span, .. }
            | Expr::UnaryOp { span, .. }
            | Expr::BoolOp { span, .. }
            | Expr::Not { span, .. }
            | Expr::Compare { span, .. }
            | Expr::Call { span, .. }
            | Expr::Attr { span, .. }
            | Expr::Index { span, .. }
            | Expr::Slice { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Ternary { span, .. }
            | Expr::Await { span, .. }
            | Expr::Yield { span, .. }
            | Expr::Repeat { span, .. }
            | Expr::Try { span, .. }
            | Expr::Match { span, .. } => *span,
        }
    }
}
