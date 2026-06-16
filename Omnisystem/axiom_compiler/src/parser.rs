// AXIOM PARSER

use crate::lexer::Token;

#[derive(Debug, Clone)]
pub enum Expr {
    Theorem { name: String, statement: String },
    Proof { theorem: String, steps: Vec<String> },
}

pub struct Program {
    pub statements: Vec<Expr>,
}

pub fn parse(_tokens: Vec<Token>) -> Result<Program, String> {
    Ok(Program {
        statements: vec![],
    })
}
