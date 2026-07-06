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
    pub body: Expr,
    pub span: Span,
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
    Ident { name: String, span: Span },
    BinOp { op: String, left: Box<Expr>, right: Box<Expr>, span: Span },
    UnaryOp { op: String, expr: Box<Expr>, span: Span },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int { span, .. } | Expr::Bool { span, .. } | Expr::Ident { span, .. } | Expr::BinOp { span, .. } | Expr::UnaryOp { span, .. } => *span,
        }
    }
}
