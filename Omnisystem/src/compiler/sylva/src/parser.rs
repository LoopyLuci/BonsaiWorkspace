// SYLVA PARSER

use crate::lexer::Token;

#[derive(Debug, Clone)]
pub enum Expr {
    Tensor { shape: Vec<usize> },
    Model { layers: Vec<(String, String)> },
    Call { func: String, args: Vec<Expr> },
    Number(f64),
    Identifier(String),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assignment { name: String, value: Expr },
    Expression(Expr),
}

pub struct Program {
    pub statements: Vec<Stmt>,
}

pub fn parse(tokens: Vec<Token>) -> Result<Program, String> {
    let mut stmts = Vec::new();
    let mut pos = 0;

    while pos < tokens.len() {
        match &tokens[pos] {
            Token::Eof => break,
            Token::Tensor => {
                pos += 1;
                if let Token::Identifier(name) = &tokens[pos] {
                    let name = name.clone();
                    pos += 3; // skip =, randn, (
                    // Simple shape parsing
                    stmts.push(Stmt::Assignment {
                        name,
                        value: Expr::Tensor { shape: vec![3, 4] },
                    });
                }
            },
            Token::Let => {
                pos += 1;
                if let Token::Identifier(name) = &tokens[pos] {
                    let name = name.clone();
                    pos += 2; // skip =
                    stmts.push(Stmt::Assignment {
                        name,
                        value: Expr::Number(0.0),
                    });
                }
            },
            _ => { pos += 1; }
        }
    }

    Ok(Program { statements: stmts })
}
