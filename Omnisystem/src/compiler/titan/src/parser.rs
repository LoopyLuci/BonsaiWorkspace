// TITAN PARSER - Converts tokens to AST

use crate::lexer::{Token, TokenType};
use crate::ast::*;

pub fn parse(tokens: Vec<Token>) -> Result<Program, String> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    fn parse_program(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            if let Ok(stmt) = self.parse_statement() {
                statements.push(stmt);
            } else {
                self.advance();
            }
        }

        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        match &self.current().token_type {
            TokenType::Let => self.parse_let(),
            TokenType::Fn => self.parse_fn(),
            TokenType::While => self.parse_while(),
            TokenType::For => self.parse_for(),
            TokenType::Return => self.parse_return(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, String> {
        self.consume(TokenType::Let)?;

        let mutable = if matches!(self.current().token_type, TokenType::Mut) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.parse_identifier()?;

        let type_hint = if matches!(self.current().token_type, TokenType::Colon) {
            self.advance();
            Some(self.parse_identifier()?)
        } else {
            None
        };

        self.consume(TokenType::Equal)?;
        let value = self.parse_expression()?;
        self.consume_optional(TokenType::Semicolon);

        Ok(Stmt::Let {
            mutable,
            name,
            type_hint,
            value,
        })
    }

    fn parse_fn(&mut self) -> Result<Stmt, String> {
        self.consume(TokenType::Fn)?;

        let name = self.parse_identifier()?;
        self.consume(TokenType::LeftParen)?;

        let mut params = Vec::new();
        if !matches!(self.current().token_type, TokenType::RightParen) {
            loop {
                let param_name = self.parse_identifier()?;
                let param_type = if matches!(self.current().token_type, TokenType::Colon) {
                    self.advance();
                    Some(self.parse_identifier()?)
                } else {
                    None
                };
                params.push((param_name, param_type));

                if !matches!(self.current().token_type, TokenType::Comma) {
                    break;
                }
                self.advance();
            }
        }

        self.consume(TokenType::RightParen)?;

        let return_type = if matches!(self.current().token_type, TokenType::Arrow) {
            self.advance();
            Some(self.parse_identifier()?)
        } else {
            None
        };

        self.consume(TokenType::LeftBrace)?;
        let mut body = Vec::new();
        while !matches!(self.current().token_type, TokenType::RightBrace) && !self.is_at_end() {
            if let Ok(stmt) = self.parse_statement() {
                body.push(stmt);
            } else {
                self.advance();
            }
        }
        self.consume(TokenType::RightBrace)?;

        Ok(Stmt::FnDef {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.consume(TokenType::While)?;
        let condition = self.parse_expression()?;
        self.consume(TokenType::LeftBrace)?;

        let mut body = Vec::new();
        while !matches!(self.current().token_type, TokenType::RightBrace) && !self.is_at_end() {
            if let Ok(stmt) = self.parse_statement() {
                body.push(stmt);
            } else {
                self.advance();
            }
        }
        self.consume(TokenType::RightBrace)?;

        Ok(Stmt::While { condition, body })
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.consume(TokenType::For)?;
        let var = self.parse_identifier()?;
        self.consume(TokenType::In)?;
        let iterable = self.parse_expression()?;
        self.consume(TokenType::LeftBrace)?;

        let mut body = Vec::new();
        while !matches!(self.current().token_type, TokenType::RightBrace) && !self.is_at_end() {
            if let Ok(stmt) = self.parse_statement() {
                body.push(stmt);
            } else {
                self.advance();
            }
        }
        self.consume(TokenType::RightBrace)?;

        Ok(Stmt::For {
            var,
            iterable,
            body,
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.consume(TokenType::Return)?;
        let value = if matches!(self.current().token_type, TokenType::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.consume_optional(TokenType::Semicolon);
        Ok(Stmt::Return(value))
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt, String> {
        let expr = self.parse_expression()?;
        self.consume_optional(TokenType::Semicolon);
        Ok(Stmt::Expression(expr))
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;

        while matches!(self.current().token_type, TokenType::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;

        while matches!(self.current().token_type, TokenType::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;

        while let Some(op) = match &self.current().token_type {
            TokenType::EqualEqual => Some(BinOp::Equal),
            TokenType::NotEqual => Some(BinOp::NotEqual),
            _ => None,
        } {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_addition()?;

        while let Some(op) = match &self.current().token_type {
            TokenType::Less => Some(BinOp::Less),
            TokenType::Greater => Some(BinOp::Greater),
            TokenType::LessEqual => Some(BinOp::LessEqual),
            TokenType::GreaterEqual => Some(BinOp::GreaterEqual),
            _ => None,
        } {
            self.advance();
            let right = self.parse_addition()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplication()?;

        while let Some(op) = match &self.current().token_type {
            TokenType::Plus => Some(BinOp::Add),
            TokenType::Minus => Some(BinOp::Subtract),
            _ => None,
        } {
            self.advance();
            let right = self.parse_multiplication()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;

        while let Some(op) = match &self.current().token_type {
            TokenType::Star => Some(BinOp::Multiply),
            TokenType::Slash => Some(BinOp::Divide),
            TokenType::Percent => Some(BinOp::Modulo),
            _ => None,
        } {
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match &self.current().token_type {
            TokenType::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnOp::Negate,
                    operand: Box::new(operand),
                })
            }
            TokenType::Not => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnOp::Not,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match &self.current().token_type {
                TokenType::LeftParen => {
                    self.advance();
                    let mut args = Vec::new();
                    if !matches!(self.current().token_type, TokenType::RightParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if !matches!(self.current().token_type, TokenType::Comma) {
                                break;
                            }
                            self.advance();
                        }
                    }
                    self.consume(TokenType::RightParen)?;
                    expr = Expr::Call {
                        func: Box::new(expr),
                        args,
                    };
                }
                TokenType::LeftBracket => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.consume(TokenType::RightBracket)?;
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match &self.current().token_type {
            TokenType::Integer(n) => {
                let value = *n;
                self.advance();
                Ok(Expr::Integer(value))
            }
            TokenType::Float(f) => {
                let value = *f;
                self.advance();
                Ok(Expr::Float(value))
            }
            TokenType::String(s) => {
                let value = s.clone();
                self.advance();
                Ok(Expr::String(value))
            }
            TokenType::Boolean(b) => {
                let value = *b;
                self.advance();
                Ok(Expr::Boolean(value))
            }
            TokenType::Identifier(name) => {
                let value = name.clone();
                self.advance();
                Ok(Expr::Identifier(value))
            }
            TokenType::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume(TokenType::RightParen)?;
                Ok(expr)
            }
            TokenType::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();
                if !matches!(self.current().token_type, TokenType::RightBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !matches!(self.current().token_type, TokenType::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                self.consume(TokenType::RightBracket)?;
                Ok(Expr::Array(elements))
            }
            TokenType::If => self.parse_if_expr(),
            _ => Err(format!("Unexpected token: {:?}", self.current().token_type)),
        }
    }

    fn parse_if_expr(&mut self) -> Result<Expr, String> {
        self.consume(TokenType::If)?;
        let condition = self.parse_expression()?;
        self.consume(TokenType::LeftBrace)?;

        let mut then_branch = Vec::new();
        while !matches!(self.current().token_type, TokenType::RightBrace) && !self.is_at_end() {
            if let Ok(stmt) = self.parse_statement() {
                then_branch.push(stmt);
            } else {
                self.advance();
            }
        }
        self.consume(TokenType::RightBrace)?;

        let else_branch = if matches!(self.current().token_type, TokenType::Else) {
            self.advance();
            self.consume(TokenType::LeftBrace)?;
            let mut branch = Vec::new();
            while !matches!(self.current().token_type, TokenType::RightBrace) && !self.is_at_end() {
                if let Ok(stmt) = self.parse_statement() {
                    branch.push(stmt);
                } else {
                    self.advance();
                }
            }
            self.consume(TokenType::RightBrace)?;
            Some(branch)
        } else {
            None
        };

        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
        })
    }

    fn parse_identifier(&mut self) -> Result<String, String> {
        match &self.current().token_type {
            TokenType::Identifier(name) => {
                let result = name.clone();
                self.advance();
                Ok(result)
            }
            _ => Err("Expected identifier".to_string()),
        }
    }

    fn consume(&mut self, token_type: TokenType) -> Result<(), String> {
        if std::mem::discriminant(&self.current().token_type) == std::mem::discriminant(&token_type) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", token_type, self.current().token_type))
        }
    }

    fn consume_optional(&mut self, token_type: TokenType) {
        if std::mem::discriminant(&self.current().token_type) == std::mem::discriminant(&token_type) {
            self.advance();
        }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token {
            token_type: TokenType::Eof,
            line: 0,
            column: 0,
        })
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        matches!(self.current().token_type, TokenType::Eof)
    }
}
