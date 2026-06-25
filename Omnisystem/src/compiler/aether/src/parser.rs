// AETHER PARSER

use crate::lexer::Token;

#[derive(Debug, Clone)]
pub enum Expr {
    ActorDef { name: String },
    Spawn { actor: String },
    Send { target: String, message: String },
    Replicate,
}

#[derive(Debug)]
pub enum Stmt {
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
            Token::Actor => {
                pos += 1;
                if let Token::Identifier(name) = &tokens.get(pos).cloned().unwrap_or(Token::Eof) {
                    stmts.push(Stmt::Expression(Expr::ActorDef {
                        name: name.clone(),
                    }));
                }
                pos += 1;
            },
            Token::Spawn => {
                pos += 1;
                if let Token::Identifier(actor) = &tokens.get(pos).cloned().unwrap_or(Token::Eof) {
                    stmts.push(Stmt::Expression(Expr::Spawn {
                        actor: actor.clone(),
                    }));
                }
                pos += 1;
            },
            Token::Replicate => {
                stmts.push(Stmt::Expression(Expr::Replicate));
                pos += 1;
            },
            _ => { pos += 1; }
        }
    }

    Ok(Program { statements: stmts })
}
