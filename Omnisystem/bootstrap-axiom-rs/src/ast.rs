//! Axiom AST — a formal-verification checker's input language, not a
//! general-purpose program: `axiom` (assumed ground facts), `theorem`
//! (checked propositions, optionally quantified over explicit bounded
//! ranges), and `invariant` (checked across an explicit, enumerated state
//! space — TLA+ heritage). See `interp.rs` for what "checked" really means
//! here (bounded-exhaustive verification, not example-based testing and
//! not a full dependent type theory).

use crate::diag::Span;

#[derive(Debug, Clone)]
pub struct Module {
    pub axioms: Vec<AxiomDef>,
    pub theorems: Vec<TheoremDef>,
    pub invariants: Vec<InvariantDef>,
}

/// An assumed, unchecked ground fact — evaluated once at load time, and
/// must have no free variables (there's nothing to quantify it over).
#[derive(Debug, Clone)]
pub struct AxiomDef {
    pub name: String,
    pub body: Expr,
    pub span: Span,
}

/// `forall v in lo..hi` — both bounds are literal integers (Axiom requires
/// an explicit, finite domain to verify over; an unbounded free variable is
/// a real, reportable error, not silently assumed universal).
#[derive(Debug, Clone)]
pub struct QuantBinding {
    pub var: String,
    pub lo: i64,
    pub hi: i64, // exclusive, matching Helix's range convention
}

#[derive(Debug, Clone)]
pub struct TheoremDef {
    pub name: String,
    pub foralls: Vec<QuantBinding>,
    pub body: TheoremBody,
    pub span: Span,
}

/// Two theorem shapes coexist: the original single quantified-expression
/// form (`theorem N forall v in 0..5 { expr }`, this bootstrap's own test
/// corpus), and the structured form used by the omni-integration specs
/// (`theorem N { preconditions {..} postconditions {..} invariants {..}
/// assertions {..} }`). The structured form is parsed for lex/parse-level
/// verification (`check`) only — it is not executed by `interp.rs`'s
/// bounded-exhaustive checker, which still only understands `Simple`.
#[derive(Debug, Clone)]
pub enum TheoremBody {
    Simple(Expr),
    Structured {
        preconditions: Vec<(String, String)>,
        postconditions: Vec<Stmt>,
        named_invariants: Vec<(String, Expr)>,
        assertions: Vec<Stmt>,
    },
}

/// A statement inside a structured theorem's `postconditions`/`assertions`
/// block. Parse-level only (see `TheoremBody`).
#[derive(Debug, Clone)]
pub enum Stmt {
    Assert(Expr),
    Let { name: String, value: Expr },
    If { cond: Expr, body: Vec<Stmt> },
    /// `forall v1, v2, .. [in collection] [where guard] { stmts }` — the
    /// statement-context sibling of `Expr::ForallIn` (used inside
    /// `postconditions`/`assertions` blocks, where the forall's body is a
    /// list of statements, not one expression).
    ForallStmt { vars: Vec<String>, collection: Option<Expr>, guard: Option<Expr>, body: Vec<Stmt> },
    Expr(Expr),
}

/// One explicit state: a flat set of `name: value` integer bindings.
pub type StateLit = Vec<(String, i64)>;

#[derive(Debug, Clone)]
pub struct InvariantDef {
    pub name: String,
    pub states: Vec<StateLit>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int { v: i64, span: Span },
    Bool { v: bool, span: Span },
    Str { v: String, span: Span },
    Ident { name: String, span: Span },
    BinOp { op: String, left: Box<Expr>, right: Box<Expr>, span: Span },
    UnaryOp { op: String, expr: Box<Expr>, span: Span },
    /// `f(a, b)` — a bare function call.
    Call { func: String, args: Vec<Expr>, span: Span },
    /// `obj.name(a, b)` (or `obj.name` with no parens, `args` empty and
    /// `has_parens: false`) — Axiom doesn't distinguish field access from
    /// method calls at parse time; `interp.rs` (untouched here) already
    /// doesn't model either, so this is parse-level only.
    MethodCall { obj: Box<Expr>, name: String, args: Vec<Expr>, has_parens: bool, span: Span },
    /// `obj[index]`.
    Index { obj: Box<Expr>, index: Box<Expr>, span: Span },
    /// `|params| body` — a closure literal (e.g. `.all(|c| c.is_alphanumeric())`).
    /// Parsed, never invoked (see `TheoremBody::Structured` doc comment).
    Closure { params: Vec<String>, body: Box<Expr>, span: Span },
    /// `forall v1, v2, .. [in collection] [where guard] => body`, generalizing
    /// `QuantBinding`'s literal-integer-range-only, single-variable
    /// quantifier: `collection` may be an arbitrary expression (a named set
    /// like `SENSITIVE_OPS`, or an `Expr::Range`), and may be absent
    /// (`forall i, j where j < i => ...`).
    ForallIn { vars: Vec<String>, collection: Option<Box<Expr>>, guard: Option<Box<Expr>>, body: Box<Expr>, span: Span },
    /// `exists v in collection where cond`.
    ExistsIn { var: String, collection: Box<Expr>, cond: Box<Expr>, span: Span },
    /// `lo..hi` with arbitrary (not necessarily literal-integer) bounds —
    /// used as a `forall`/`exists` collection, e.g. `0..events.len()`.
    Range { lo: Box<Expr>, hi: Box<Expr>, span: Span },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int { span, .. }
            | Expr::Bool { span, .. }
            | Expr::Str { span, .. }
            | Expr::Ident { span, .. }
            | Expr::BinOp { span, .. }
            | Expr::UnaryOp { span, .. }
            | Expr::Call { span, .. }
            | Expr::MethodCall { span, .. }
            | Expr::Index { span, .. }
            | Expr::Closure { span, .. }
            | Expr::ForallIn { span, .. }
            | Expr::ExistsIn { span, .. }
            | Expr::Range { span, .. } => *span,
        }
    }
}
