// TITAN COMPILER - Complete Implementation
// Lexer, Parser, Type Checker, and Code Generator

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// LEXER
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Keywords
    Fn, Async, Await, Let, Mut, If, Else, While, For, Return, Break, Continue,
    Struct, Enum, Trait, Impl, Use, Pub, Type, Const, Static, Unsafe,

    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),

    // Identifiers
    Ident(String),

    // Operators
    Plus, Minus, Star, Slash, Percent,
    Equal, EqualEqual, NotEqual, Less, LessEqual, Greater, GreaterEqual,
    And, Or, Not, Ampersand, Pipe, Caret,
    LeftShift, RightShift,
    Dot, DotDot, Arrow, FatArrow,

    // Delimiters
    LeftParen, RightParen,
    LeftBrace, RightBrace,
    LeftBracket, RightBracket,
    Comma, Semicolon, Colon, DoubleColon,

    // Special
    Eof,
}

pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub column: usize,
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    fn current_char(&self) -> Option<char> {
        if self.position < self.input.len() {
            Some(self.input[self.position])
        } else {
            None
        }
    }

    fn peek_char(&self, offset: usize) -> Option<char> {
        let pos = self.position + offset;
        if pos < self.input.len() {
            Some(self.input[pos])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        if self.position < self.input.len() {
            if self.input[self.position] == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.position += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.current_char() == Some('/') && self.peek_char(1) == Some('/') {
            while self.current_char().is_some() && self.current_char() != Some('\n') {
                self.advance();
            }
        }
    }

    fn read_number(&mut self) -> TokenType {
        let mut number = String::new();
        let mut is_float = false;

        while let Some(ch) = self.current_char() {
            if ch.is_ascii_digit() {
                number.push(ch);
                self.advance();
            } else if ch == '.' && self.peek_char(1).map_or(false, |c| c.is_ascii_digit()) {
                is_float = true;
                number.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if is_float {
            TokenType::Float(number.parse().unwrap())
        } else {
            TokenType::Integer(number.parse().unwrap())
        }
    }

    fn read_string(&mut self) -> TokenType {
        self.advance(); // Skip opening quote
        let mut string = String::new();

        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance();
                break;
            } else if ch == '\\' {
                self.advance();
                match self.current_char() {
                    Some('n') => string.push('\n'),
                    Some('t') => string.push('\t'),
                    Some('\\') => string.push('\\'),
                    Some('"') => string.push('"'),
                    Some(ch) => string.push(ch),
                    None => break,
                }
                self.advance();
            } else {
                string.push(ch);
                self.advance();
            }
        }

        TokenType::String(string)
    }

    fn read_ident(&mut self) -> TokenType {
        let mut ident = String::new();

        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        match ident.as_str() {
            "fn" => TokenType::Fn,
            "async" => TokenType::Async,
            "await" => TokenType::Await,
            "let" => TokenType::Let,
            "mut" => TokenType::Mut,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "while" => TokenType::While,
            "for" => TokenType::For,
            "return" => TokenType::Return,
            "true" => TokenType::Bool(true),
            "false" => TokenType::Bool(false),
            "struct" => TokenType::Struct,
            "enum" => TokenType::Enum,
            "trait" => TokenType::Trait,
            "impl" => TokenType::Impl,
            "break" => TokenType::Break,
            "continue" => TokenType::Continue,
            _ => TokenType::Ident(ident),
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        // Skip comments
        while self.current_char() == Some('/') && self.peek_char(1) == Some('/') {
            self.skip_comment();
            self.skip_whitespace();
        }

        let line = self.line;
        let column = self.column;

        let token_type = match self.current_char() {
            None => TokenType::Eof,
            Some('+') => {
                self.advance();
                TokenType::Plus
            }
            Some('-') => {
                self.advance();
                if self.current_char() == Some('>') {
                    self.advance();
                    TokenType::Arrow
                } else {
                    TokenType::Minus
                }
            }
            Some('*') => {
                self.advance();
                TokenType::Star
            }
            Some('/') => {
                self.advance();
                TokenType::Slash
            }
            Some('%') => {
                self.advance();
                TokenType::Percent
            }
            Some('=') => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    TokenType::EqualEqual
                } else if self.current_char() == Some('>') {
                    self.advance();
                    TokenType::FatArrow
                } else {
                    TokenType::Equal
                }
            }
            Some('!') => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    TokenType::NotEqual
                } else {
                    TokenType::Not
                }
            }
            Some('<') => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    TokenType::LessEqual
                } else if self.current_char() == Some('<') {
                    self.advance();
                    TokenType::LeftShift
                } else {
                    TokenType::Less
                }
            }
            Some('>') => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    TokenType::GreaterEqual
                } else if self.current_char() == Some('>') {
                    self.advance();
                    TokenType::RightShift
                } else {
                    TokenType::Greater
                }
            }
            Some('&') => {
                self.advance();
                if self.current_char() == Some('&') {
                    self.advance();
                    TokenType::And
                } else {
                    TokenType::Ampersand
                }
            }
            Some('|') => {
                self.advance();
                if self.current_char() == Some('|') {
                    self.advance();
                    TokenType::Or
                } else {
                    TokenType::Pipe
                }
            }
            Some('^') => {
                self.advance();
                TokenType::Caret
            }
            Some('(') => {
                self.advance();
                TokenType::LeftParen
            }
            Some(')') => {
                self.advance();
                TokenType::RightParen
            }
            Some('{') => {
                self.advance();
                TokenType::LeftBrace
            }
            Some('}') => {
                self.advance();
                TokenType::RightBrace
            }
            Some('[') => {
                self.advance();
                TokenType::LeftBracket
            }
            Some(']') => {
                self.advance();
                TokenType::RightBracket
            }
            Some(',') => {
                self.advance();
                TokenType::Comma
            }
            Some(';') => {
                self.advance();
                TokenType::Semicolon
            }
            Some(':') => {
                self.advance();
                if self.current_char() == Some(':') {
                    self.advance();
                    TokenType::DoubleColon
                } else {
                    TokenType::Colon
                }
            }
            Some('.') => {
                self.advance();
                if self.current_char() == Some('.') {
                    self.advance();
                    TokenType::DotDot
                } else {
                    TokenType::Dot
                }
            }
            Some('"') => self.read_string(),
            Some(ch) if ch.is_ascii_digit() => self.read_number(),
            Some(ch) if ch.is_alphabetic() || ch == '_' => self.read_ident(),
            Some(ch) => {
                self.advance();
                TokenType::Ident(ch.to_string())
            }
        };

        Token { token_type, line, column }
    }
}

// ============================================================================
// AST DEFINITIONS
// ============================================================================

#[derive(Debug, Clone)]
pub enum Expr {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Ident(String),
    BinOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    IfExpr {
        cond: Box<Expr>,
        then_body: Box<Expr>,
        else_body: Option<Box<Expr>>,
    },
    Block(Vec<Stmt>),
    Return(Option<Box<Expr>>),
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<String>,
        value: Expr,
    },
    Expr(Expr),
    FnDecl {
        name: String,
        params: Vec<(String, String)>,
        return_type: String,
        body: Vec<Stmt>,
    },
}

// ============================================================================
// PARSER
// ============================================================================

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();

        loop {
            let token = lexer.next_token();
            let is_eof = matches!(token.token_type, TokenType::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Parser { tokens, position: 0 }
    }

    fn current_token(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn peek_token(&self, offset: usize) -> &Token {
        let pos = (self.position + offset).min(self.tokens.len() - 1);
        &self.tokens[pos]
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() - 1 {
            self.position += 1;
        }
    }

    fn expect(&mut self, expected: TokenType) -> Result<(), String> {
        if std::mem::discriminant(&self.current_token().token_type) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", expected, self.current_token().token_type))
        }
    }

    pub fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements = Vec::new();

        while !matches!(self.current_token().token_type, TokenType::Eof) {
            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        match &self.current_token().token_type {
            TokenType::Let => self.parse_let(),
            TokenType::Fn => self.parse_fn(),
            _ => {
                let expr = self.parse_expression()?;
                if matches!(self.current_token().token_type, TokenType::Semicolon) {
                    self.advance();
                }
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, String> {
        self.expect(TokenType::Let)?;

        let name = match &self.current_token().token_type {
            TokenType::Ident(n) => {
                self.advance();
                n.clone()
            }
            _ => return Err("Expected identifier".to_string()),
        };

        let ty = if matches!(self.current_token().token_type, TokenType::Colon) {
            self.advance();
            match &self.current_token().token_type {
                TokenType::Ident(t) => {
                    self.advance();
                    Some(t.clone())
                }
                _ => return Err("Expected type".to_string()),
            }
        } else {
            None
        };

        self.expect(TokenType::Equal)?;
        let value = self.parse_expression()?;

        if matches!(self.current_token().token_type, TokenType::Semicolon) {
            self.advance();
        }

        Ok(Stmt::Let { name, ty, value })
    }

    fn parse_fn(&mut self) -> Result<Stmt, String> {
        self.expect(TokenType::Fn)?;

        let name = match &self.current_token().token_type {
            TokenType::Ident(n) => {
                self.advance();
                n.clone()
            }
            _ => return Err("Expected function name".to_string()),
        };

        self.expect(TokenType::LeftParen)?;
        let mut params = Vec::new();

        while !matches!(self.current_token().token_type, TokenType::RightParen) {
            let param_name = match &self.current_token().token_type {
                TokenType::Ident(n) => {
                    self.advance();
                    n.clone()
                }
                _ => return Err("Expected parameter name".to_string()),
            };

            self.expect(TokenType::Colon)?;

            let param_type = match &self.current_token().token_type {
                TokenType::Ident(t) => {
                    self.advance();
                    t.clone()
                }
                _ => return Err("Expected parameter type".to_string()),
            };

            params.push((param_name, param_type));

            if matches!(self.current_token().token_type, TokenType::Comma) {
                self.advance();
            }
        }

        self.expect(TokenType::RightParen)?;

        let return_type = if matches!(self.current_token().token_type, TokenType::Arrow) {
            self.advance();
            match &self.current_token().token_type {
                TokenType::Ident(t) => {
                    self.advance();
                    t.clone()
                }
                _ => return Err("Expected return type".to_string()),
            }
        } else {
            "void".to_string()
        };

        self.expect(TokenType::LeftBrace)?;
        let mut body = Vec::new();

        while !matches!(self.current_token().token_type, TokenType::RightBrace) {
            body.push(self.parse_statement()?);
        }

        self.expect(TokenType::RightBrace)?;

        Ok(Stmt::FnDecl { name, params, return_type, body })
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;

        while matches!(self.current_token().token_type, TokenType::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinOp {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;

        while matches!(self.current_token().token_type, TokenType::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinOp {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = match &self.current_token().token_type {
                TokenType::EqualEqual => BinOp::Eq,
                TokenType::NotEqual => BinOp::Ne,
                _ => break,
            };

            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match &self.current_token().token_type {
                TokenType::Less => BinOp::Lt,
                TokenType::LessEqual => BinOp::Le,
                TokenType::Greater => BinOp::Gt,
                TokenType::GreaterEqual => BinOp::Ge,
                _ => break,
            };

            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match &self.current_token().token_type {
                TokenType::Plus => BinOp::Add,
                TokenType::Minus => BinOp::Sub,
                _ => break,
            };

            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match &self.current_token().token_type {
                TokenType::Star => BinOp::Mul,
                TokenType::Slash => BinOp::Div,
                TokenType::Percent => BinOp::Mod,
                _ => break,
            };

            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match &self.current_token().token_type {
            TokenType::Not => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            TokenType::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match &self.current_token().token_type {
                TokenType::LeftParen => {
                    self.advance();
                    let mut args = Vec::new();

                    while !matches!(self.current_token().token_type, TokenType::RightParen) {
                        args.push(self.parse_expression()?);
                        if matches!(self.current_token().token_type, TokenType::Comma) {
                            self.advance();
                        }
                    }

                    self.expect(TokenType::RightParen)?;
                    expr = Expr::Call {
                        func: Box::new(expr),
                        args,
                    };
                }
                TokenType::Dot => {
                    self.advance();
                    let method = match &self.current_token().token_type {
                        TokenType::Ident(m) => {
                            self.advance();
                            m.clone()
                        }
                        _ => return Err("Expected method name".to_string()),
                    };

                    self.expect(TokenType::LeftParen)?;
                    let mut args = Vec::new();

                    while !matches!(self.current_token().token_type, TokenType::RightParen) {
                        args.push(self.parse_expression()?);
                        if matches!(self.current_token().token_type, TokenType::Comma) {
                            self.advance();
                        }
                    }

                    self.expect(TokenType::RightParen)?;
                    expr = Expr::MethodCall {
                        object: Box::new(expr),
                        method,
                        args,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match &self.current_token().token_type.clone() {
            TokenType::Integer(n) => {
                let n = *n;
                self.advance();
                Ok(Expr::Integer(n))
            }
            TokenType::Float(f) => {
                let f = *f;
                self.advance();
                Ok(Expr::Float(f))
            }
            TokenType::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::String(s))
            }
            TokenType::Bool(b) => {
                let b = *b;
                self.advance();
                Ok(Expr::Bool(b))
            }
            TokenType::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Ident(name))
            }
            TokenType::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(TokenType::RightParen)?;
                Ok(expr)
            }
            TokenType::If => {
                self.advance();
                self.expect(TokenType::LeftParen)?;
                let cond = self.parse_expression()?;
                self.expect(TokenType::RightParen)?;
                self.expect(TokenType::LeftBrace)?;

                let mut then_body_stmts = Vec::new();
                while !matches!(self.current_token().token_type, TokenType::RightBrace) {
                    then_body_stmts.push(self.parse_statement()?);
                }
                self.expect(TokenType::RightBrace)?;

                let then_body = Box::new(Expr::Block(then_body_stmts));

                let else_body = if matches!(self.current_token().token_type, TokenType::Else) {
                    self.advance();
                    self.expect(TokenType::LeftBrace)?;

                    let mut else_body_stmts = Vec::new();
                    while !matches!(self.current_token().token_type, TokenType::RightBrace) {
                        else_body_stmts.push(self.parse_statement()?);
                    }
                    self.expect(TokenType::RightBrace)?;

                    Some(Box::new(Expr::Block(else_body_stmts)))
                } else {
                    None
                };

                Ok(Expr::IfExpr {
                    cond: Box::new(cond),
                    then_body,
                    else_body,
                })
            }
            _ => Err(format!("Unexpected token: {:?}", self.current_token().token_type)),
        }
    }
}

// ============================================================================
// TYPE CHECKER
// ============================================================================

pub struct TypeChecker {
    types: HashMap<String, String>,
    functions: HashMap<String, (Vec<String>, String)>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            types: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn check(&mut self, statements: &[Stmt]) -> Result<(), String> {
        for stmt in statements {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Let { name, ty, value } => {
                let inferred_type = self.infer_type(value)?;
                if let Some(declared_type) = ty {
                    if declared_type != &inferred_type {
                        return Err(format!(
                            "Type mismatch for {}: expected {}, got {}",
                            name, declared_type, inferred_type
                        ));
                    }
                }
                self.types.insert(name.clone(), inferred_type);
                Ok(())
            }
            Stmt::FnDecl { name, params, return_type, body } => {
                let param_types: Vec<String> = params.iter().map(|(_, t)| t.clone()).collect();
                self.functions.insert(name.clone(), (param_types, return_type.clone()));

                for stmt in body {
                    self.check_stmt(stmt)?;
                }
                Ok(())
            }
            Stmt::Expr(expr) => {
                self.infer_type(expr)?;
                Ok(())
            }
        }
    }

    fn infer_type(&self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Integer(_) => Ok("i64".to_string()),
            Expr::Float(_) => Ok("f64".to_string()),
            Expr::String(_) => Ok("String".to_string()),
            Expr::Bool(_) => Ok("bool".to_string()),
            Expr::Ident(name) => {
                self.types
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable: {}", name))
            }
            Expr::BinOp { left, op, right } => {
                let left_type = self.infer_type(left)?;
                let right_type = self.infer_type(right)?;

                if left_type != right_type {
                    return Err(format!(
                        "Type mismatch in binary operation: {} {:?} {}",
                        left_type, op, right_type
                    ));
                }

                match op {
                    BinOp::And | BinOp::Or | BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        Ok("bool".to_string())
                    }
                    _ => Ok(left_type),
                }
            }
            Expr::UnaryOp { op, expr } => {
                let expr_type = self.infer_type(expr)?;
                match op {
                    UnaryOp::Not => Ok("bool".to_string()),
                    UnaryOp::Neg => Ok(expr_type),
                }
            }
            Expr::Call { func, args } => {
                if let Expr::Ident(name) = &**func {
                    if let Some((param_types, return_type)) = self.functions.get(name) {
                        if args.len() != param_types.len() {
                            return Err(format!(
                                "Function {} expects {} arguments, got {}",
                                name,
                                param_types.len(),
                                args.len()
                            ));
                        }
                        return Ok(return_type.clone());
                    }
                }
                Err("Unknown function".to_string())
            }
            Expr::Block(_) => Ok("void".to_string()),
            Expr::Return(Some(expr)) => self.infer_type(expr),
            Expr::Return(None) => Ok("void".to_string()),
            Expr::IfExpr { then_body, else_body, .. } => {
                let then_type = self.infer_type(then_body)?;
                if let Some(else_body) = else_body {
                    let else_type = self.infer_type(else_body)?;
                    if then_type != else_type {
                        return Err(format!(
                            "If-else type mismatch: {} vs {}",
                            then_type, else_type
                        ));
                    }
                }
                Ok(then_type)
            }
            _ => Ok("unknown".to_string()),
        }
    }
}

// ============================================================================
// CODE GENERATOR (to OCPF-IR)
// ============================================================================

pub struct CodeGenerator {
    ir_code: Vec<String>,
    var_counter: usize,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            ir_code: Vec::new(),
            var_counter: 0,
        }
    }

    pub fn generate(&mut self, statements: &[Stmt]) -> Vec<String> {
        self.ir_code.push("@module".to_string());

        for stmt in statements {
            self.generate_stmt(stmt);
        }

        self.ir_code.clone()
    }

    fn generate_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, ty, value } => {
                let ty = ty.as_deref().unwrap_or("i64");
                let var = self.generate_expr(value);
                self.ir_code.push(format!("  let {} {} = {}", name, ty, var));
            }
            Stmt::FnDecl { name, params, return_type, body } => {
                let param_str = params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.ir_code.push(format!(
                    "  fn {}({}) -> {} {{",
                    name, param_str, return_type
                ));

                for stmt in body {
                    self.generate_stmt(stmt);
                }

                self.ir_code.push("  }".to_string());
            }
            Stmt::Expr(expr) => {
                self.generate_expr(expr);
            }
        }
    }

    fn generate_expr(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Integer(n) => format!("{}", n),
            Expr::Float(f) => format!("{}", f),
            Expr::String(s) => format!("\"{}\"", s),
            Expr::Bool(b) => format!("{}", b),
            Expr::Ident(name) => name.clone(),
            Expr::BinOp { left, op, right } => {
                let left = self.generate_expr(left);
                let right = self.generate_expr(right);
                let op_str = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Mod => "%",
                    BinOp::Eq => "==",
                    BinOp::Ne => "!=",
                    BinOp::Lt => "<",
                    BinOp::Le => "<=",
                    BinOp::Gt => ">",
                    BinOp::Ge => ">=",
                    BinOp::And => "&&",
                    BinOp::Or => "||",
                };
                format!("({} {} {})", left, op_str, right)
            }
            Expr::Call { func, args } => {
                let func = self.generate_expr(func);
                let args_str = args
                    .iter()
                    .map(|a| self.generate_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", func, args_str)
            }
            Expr::Block(_) => "block".to_string(),
            _ => "unknown".to_string(),
        }
    }
}

// ============================================================================
// TITAN COMPILER ENTRY POINT
// ============================================================================

pub struct TitanCompiler;

impl TitanCompiler {
    pub fn compile(source: &str) -> Result<Vec<String>, String> {
        // Lexing is implicit in Parser::new()

        // Parsing
        let mut parser = Parser::new(source);
        let ast = parser.parse_program()?;

        // Type Checking
        let mut type_checker = TypeChecker::new();
        type_checker.check(&ast)?;

        // Code Generation to OCPF-IR
        let mut codegen = CodeGenerator::new();
        let ir_code = codegen.generate(&ast);

        Ok(ir_code)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer() {
        let mut lexer = Lexer::new("let x: i64 = 42;");
        let token = lexer.next_token();
        assert!(matches!(token.token_type, TokenType::Let));
    }

    #[test]
    fn test_parser() {
        let source = "let x: i64 = 42;";
        let mut parser = Parser::new(source);
        let result = parser.parse_program();
        assert!(result.is_ok());
    }

    #[test]
    fn test_titan_compiler() {
        let source = r#"
            fn add(a: i64, b: i64) -> i64 {
                a + b
            }

            let result: i64 = add(5, 3);
        "#;
        let result = TitanCompiler::compile(source);
        assert!(result.is_ok());
    }
}
