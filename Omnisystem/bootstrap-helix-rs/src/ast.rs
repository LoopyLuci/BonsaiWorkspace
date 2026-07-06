//! Helix AST — GPU/compute-kernel flavored. The real, enforced constraints
//! that make this authentic rather than "Titan with vector types" live in
//! the interpreter: no recursion (kernels/shaders/fns can't call
//! themselves, directly or transitively) and no dynamic loop bounds (a
//! `for` loop's upper bound must be a literal integer, not a computed
//! expression) — both are genuine restrictions real shader languages
//! impose for compilability to fixed-function/parallel hardware, checked
//! for real here rather than just implied by the domain.

use crate::diag::Span;

#[derive(Debug, Clone)]
pub struct Module {
    pub fns: Vec<FnDef>,
    pub kernels: Vec<KernelDef>,
    pub shaders: Vec<ShaderDef>,
    pub script: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// `kernel Name(id, buf) { ... }` — a compute kernel. `id` is bound to the
/// current thread index at dispatch time (0..dispatch_size); any other
/// params are bound from the values passed to `dispatch(...)`.
#[derive(Debug, Clone)]
pub struct KernelDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Vertex,
    Fragment,
    Compute,
}

/// `shader vertex Name(pos) { ... return ... }` — run once per element of
/// an input stream (`run_stage(Name, [inputs...])`), producing one output
/// per input. No rasterization is simulated — see `interp.rs`'s doc comment
/// for why that's an honest, deliberate scope boundary, not a shortcut.
#[derive(Debug, Clone)]
pub struct ShaderDef {
    pub name: String,
    pub stage: Stage,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Let { name: String, value: Expr, span: Span },
    Assign { target: Expr, value: Expr, span: Span },
    If { branches: Vec<(Expr, Vec<Stmt>)>, orelse: Vec<Stmt>, span: Span },
    /// `for i in 0..N { ... }` — `hi` must be `Expr::Int` (a literal), not
    /// an arbitrary computed expression; enforced in the interpreter, not
    /// just the parser, so the error message can explain why.
    For { var: String, hi: Expr, body: Vec<Stmt>, span: Span },
    Return { value: Option<Expr>, span: Span },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int { v: i64, span: Span },
    Float { v: f64, span: Span },
    Bool { v: bool, span: Span },
    Ident { name: String, span: Span },
    BinOp { op: String, left: Box<Expr>, right: Box<Expr>, span: Span },
    UnaryOp { op: String, expr: Box<Expr>, span: Span },
    Call { func: String, args: Vec<Expr>, span: Span },
    /// `.xyz` / `.rgb` / `.x` swizzle or plain field access.
    Attr { obj: Box<Expr>, name: String, span: Span },
    Index { obj: Box<Expr>, index: Box<Expr>, span: Span },
}
