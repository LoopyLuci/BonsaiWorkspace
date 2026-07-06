//! Nexus AST — a **declarative constraint/layout description**, not an
//! imperative program: `box` properties are equations (solved via lazy
//! memoized evaluation + cycle detection, spreadsheet-style), `layout`
//! blocks describe real flow-layout (row/column flex-style positioning),
//! and top-level `constrain` statements are checked (not just declared) —
//! solving can genuinely fail if a constraint is violated.

use crate::diag::Span;

#[derive(Debug, Clone)]
pub struct Module {
    pub boxes: Vec<BoxDef>,
    pub layouts: Vec<LayoutDef>,
    pub constraints: Vec<ConstraintStmt>,
}

#[derive(Debug, Clone)]
pub struct BoxDef {
    pub name: String,
    pub props: Vec<(String, Expr)>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Row,
    Column,
}

#[derive(Debug, Clone)]
pub struct LayoutDef {
    pub name: String,
    pub props: Vec<(String, Expr)>, // typically width/height
    pub direction: Direction,
    pub children: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ConstraintStmt {
    pub left: Expr,
    pub op: String, // "==" | ">=" | "<="
    pub right: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Num { v: f64, span: Span },
    /// `Box.prop` / `parent.prop` — a reference to another box/layout's
    /// (possibly not-yet-solved) property.
    PropRef { obj: String, prop: String, span: Span },
    BinOp { op: String, left: Box<Expr>, right: Box<Expr>, span: Span },
    UnaryOp { op: String, expr: Box<Expr>, span: Span },
}
