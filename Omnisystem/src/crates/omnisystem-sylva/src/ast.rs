//! Sylva Abstract Syntax Tree

#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Unit,

    // Variables and identifiers
    Var(String),

    // Binary operations
    BinOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },

    // Unary operations
    UnOp {
        op: UnOp,
        expr: Box<Expr>,
    },

    // Function application
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },

    // Control flow
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },

    // Lists
    List(Vec<Expr>),

    // Tuples
    Tuple(Vec<Expr>),

    // Dictionary/record
    Dict(Vec<(String, Expr)>),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Let {
        name: String,
        type_annotation: Option<String>,
        value: Expr,
        mutable: bool,
    },
    Expr(Expr),
    Return(Option<Expr>),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Stmt>,
}
