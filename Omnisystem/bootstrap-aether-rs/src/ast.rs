//! Aether AST — Erlang/OTP + Elixir-flavored: multi-clause pattern-matched
//! function definitions (grouped by name, clauses tried in order — a real
//! capability neither Titan's single-body-with-internal-match nor Sylva's
//! single-body-dynamic-dispatch has), atoms, cons-cell list patterns
//! (`[h | t]`), the pipe operator, and actor `spawn`/`send`/`receive`.

use crate::diag::Span;

#[derive(Debug, Clone)]
pub struct Module {
    pub name: Option<String>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    /// One function *clause*. Clauses sharing (name, arity) are grouped at
    /// registration time into a `FnClauses` value and tried in order —
    /// that grouping happens in the interpreter, not the parser, so the
    /// parser stays a straightforward single-pass recursive descent.
    FnClause(FnClause),
    ActorDef(ActorDef),
    /// A top-level statement (bare call, assignment, etc.) — a real
    /// `Stmt`, not just an `Expr`, so top-level scripts can assign
    /// variables (`c = spawn Counter(10)`) exactly like inside a function.
    TopStmt(Stmt),
}

#[derive(Debug, Clone)]
pub struct FnClause {
    pub name: String,
    pub params: Vec<Pattern>,
    pub guard: Option<Expr>,
    pub body: Vec<Stmt>,
    pub is_private: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ActorDef {
    pub name: String,
    pub fns: Vec<FnClause>,
    /// `receive <pattern>, <state_binding> do ... end` clauses — each
    /// evaluates to the actor's *new* state (Erlang gen_server-style
    /// fold-over-state, simplified for this bootstrap).
    pub receives: Vec<ReceiveClause>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReceiveClause {
    pub msg_pattern: Pattern,
    pub state_binding: String,
    pub guard: Option<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wild,
    Bind(String),
    Lit(Expr),
    Atom(String),
    Tuple(Vec<Pattern>),
    /// `[a, b, c]` — fixed-length list pattern.
    List(Vec<Pattern>),
    /// `[h | t]` — classic cons pattern: `h` binds the first element, `t`
    /// binds the rest as a list. The single most iconic Erlang-family
    /// pattern-matching construct; deliberately included for real fidelity.
    Cons(Box<Pattern>, Box<Pattern>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    /// `x = expr` — Aether variables are single-assignment-by-convention but
    /// this bootstrap allows rebinding (documented simplification; real
    /// Erlang would reject rebinding a bound variable outside pattern match).
    Assign { name: String, value: Expr, span: Span },
    If { branches: Vec<(Expr, Vec<Stmt>)>, orelse: Vec<Stmt>, span: Span },
    Case { scrut: Expr, arms: Vec<CaseArm>, span: Span },
    For { var: String, iter: Expr, body: Vec<Stmt>, span: Span },
    Return { value: Option<Expr>, span: Span },
}

#[derive(Debug, Clone)]
pub struct CaseArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum IStrPart {
    Lit(String),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int { v: i64, span: Span },
    Float { v: f64, span: Span },
    Str { v: String, span: Span },
    IStr { parts: Vec<IStrPart>, span: Span },
    Bool { v: bool, span: Span },
    Nil { span: Span },
    Atom { name: String, span: Span },
    Ident { name: String, span: Span },
    Tuple { elems: Vec<Expr>, span: Span },
    List { elems: Vec<Expr>, span: Span },
    /// `%{key: value, ...}` — Elixir-style map literal.
    Map { entries: Vec<(Expr, Expr)>, span: Span },
    BinOp { op: String, left: Box<Expr>, right: Box<Expr>, span: Span },
    UnaryOp { op: String, expr: Box<Expr>, span: Span },
    /// `lhs |> rhs(args)` desugars at parse time into `rhs(lhs, args)` —
    /// the pipe operator, Elixir's signature idiom.
    Call { func: Box<Expr>, args: Vec<Expr>, span: Span },
    Attr { obj: Box<Expr>, name: String, span: Span },
    Index { obj: Box<Expr>, index: Box<Expr>, span: Span },
    Spawn { actor: String, args: Vec<Expr>, span: Span },
    Lambda { params: Vec<Pattern>, body: Box<Expr>, span: Span },
    /// `case`/`if` used as expressions (`x = case ... end`) — real Elixir
    /// idiom (everything is an expression there). Shares evaluation logic
    /// with `Stmt::Case`/`Stmt::If` in the interpreter rather than
    /// duplicating it.
    Case { scrut: Box<Expr>, arms: Vec<CaseArm>, span: Span },
    If { branches: Vec<(Expr, Vec<Stmt>)>, orelse: Vec<Stmt>, span: Span },
}
