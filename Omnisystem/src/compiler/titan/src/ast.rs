// TITAN AST - Abstract Syntax Tree Definitions

#[derive(Debug, Clone)]
pub enum Expr {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Identifier(String),
    BinaryOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnOp,
        operand: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    Array(Vec<Expr>),
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    If {
        condition: Box<Expr>,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },
    Match {
        value: Box<Expr>,
        arms: Vec<(Pattern, Vec<Stmt>)>,
    },
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        mutable: bool,
        name: String,
        type_hint: Option<String>,
        value: Expr,
    },
    FnDef {
        name: String,
        params: Vec<(String, Option<String>)>,
        return_type: Option<String>,
        body: Vec<Stmt>,
    },
    Expression(Expr),
    Return(Option<Expr>),
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    For {
        var: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Negate,
    Not,
    BitwiseNot,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(i64),
    Identifier(String),
    Wildcard,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Stmt>,
}
