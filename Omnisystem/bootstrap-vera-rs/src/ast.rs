//! Vera AST — component-oriented, UI-flavored: `component`s with reactive
//! `state`/`computed` declarations and a `render` block containing an
//! **embedded markup tag tree** (`<Tag attr={expr}>...</Tag>`), not a
//! string template and not a builder-pattern API. This is the real,
//! structural differentiator from Titan/Sylva/Aether.

use crate::diag::Span;

#[derive(Debug, Clone)]
pub struct Module {
    pub components: Vec<ComponentDef>,
    /// Top-level script statements after the component definitions — how
    /// this bootstrap "runs" a UI without an actual browser/GPU: mount a
    /// component, render it to a text tree, fire event-handler methods,
    /// re-render, and print — proving reactivity via real, verifiable
    /// before/after output rather than an unobservable visual claim.
    pub script: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct ComponentDef {
    pub name: String,
    pub props: Vec<String>,
    pub state: Vec<(String, Expr)>,
    pub computed: Vec<(String, Expr)>,
    pub methods: Vec<FnDef>,
    pub render: Vec<Node>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// One node in a `render` block's markup tree.
#[derive(Debug, Clone)]
pub enum Node {
    /// `<div class={cls}>...</div>` or a component reference `<Counter .../>`
    /// — both parse identically; the interpreter decides at render time
    /// whether the tag names a built-in element or a defined component.
    Element { tag: String, attrs: Vec<(String, Expr)>, children: Vec<Node>, span: Span },
    Text(String),
    /// `{expr}` — interpolated value, rendered as text.
    Expr(Expr),
    If { cond: Expr, then_branch: Vec<Node>, else_branch: Vec<Node> },
    For { var: String, iter: Expr, body: Vec<Node> },
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Assign { name: String, value: Expr, span: Span },
    If { branches: Vec<(Expr, Vec<Stmt>)>, orelse: Vec<Stmt>, span: Span },
    Return { value: Option<Expr>, span: Span },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int { v: i64, span: Span },
    Float { v: f64, span: Span },
    Str { v: String, span: Span },
    Bool { v: bool, span: Span },
    Ident { name: String, span: Span },
    BinOp { op: String, left: Box<Expr>, right: Box<Expr>, span: Span },
    UnaryOp { op: String, expr: Box<Expr>, span: Span },
    Call { func: Box<Expr>, args: Vec<Expr>, span: Span },
    Attr { obj: Box<Expr>, name: String, span: Span },
    /// `fn() { stmt* }` or `|| expr` — event-handler / callback closures.
    Lambda { params: Vec<String>, body: Vec<Stmt>, span: Span },
    List { elems: Vec<Expr>, span: Span },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int { span, .. }
            | Expr::Float { span, .. }
            | Expr::Str { span, .. }
            | Expr::Bool { span, .. }
            | Expr::Ident { span, .. }
            | Expr::BinOp { span, .. }
            | Expr::UnaryOp { span, .. }
            | Expr::Call { span, .. }
            | Expr::Attr { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::List { span, .. } => *span,
        }
    }
}
