//! Sylva parser — converts a token stream into a `SylvaModule` or `Expr`.

use crate::ast::*;
use crate::lexer::{lex, Spanned, Token};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error at line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

pub type ParseResult<T> = Result<T, ParseError>;

fn err(msg: impl Into<String>, line: usize) -> ParseError {
    ParseError {
        message: msg.into(),
        line,
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Spanned>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }
    fn line(&self) -> usize {
        self.tokens[self.pos].line
    }

    fn peek_at(&self, offset: usize) -> &Token {
        self.tokens
            .get(self.pos + offset)
            .map(|s| &s.token)
            .unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos].token;
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, tok: &Token) -> ParseResult<()> {
        if self.peek() == tok {
            self.advance();
            Ok(())
        } else {
            Err(err(
                format!("expected {:?}, got {:?}", tok, self.peek()),
                self.line(),
            ))
        }
    }

    fn expect_ident(&mut self) -> ParseResult<String> {
        match self.peek().clone() {
            Token::Ident(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(err(
                format!("expected identifier, got {:?}", other),
                self.line(),
            )),
        }
    }

    fn at(&self, tok: &Token) -> bool {
        self.peek() == tok
    }

    fn eat(&mut self, tok: &Token) -> bool {
        if self.peek() == tok {
            self.advance();
            true
        } else {
            false
        }
    }

    // ── Types ─────────────────────────────────────────────────────────────────

    fn parse_type(&mut self) -> ParseResult<SylvaType> {
        match self.peek().clone() {
            Token::Ident(n) => {
                let n = n.clone();
                self.advance();
                Ok(match n.as_str() {
                    "Bool" => SylvaType::Bool,
                    "Int" => SylvaType::Int,
                    "Float" => SylvaType::Float,
                    "Str" => SylvaType::Str,
                    "Nil" => SylvaType::Nil,
                    "DataFrame" => SylvaType::DataFrame,
                    "NDArray" => SylvaType::NDArray,
                    _ => SylvaType::Named(n),
                })
            }
            Token::LBracket => {
                self.advance();
                let inner = self.parse_type()?;
                self.expect(&Token::RBracket)?;
                Ok(SylvaType::List(Box::new(inner)))
            }
            Token::Fn => {
                self.advance();
                self.expect(&Token::LParen)?;
                let mut params = Vec::new();
                while !self.at(&Token::RParen) {
                    params.push(self.parse_type()?);
                    self.eat(&Token::Comma);
                }
                self.expect(&Token::RParen)?;
                self.expect(&Token::Arrow)?;
                let ret = self.parse_type()?;
                Ok(SylvaType::Fn(params, Box::new(ret)))
            }
            Token::Question => {
                self.advance();
                let inner = self.parse_type()?;
                Ok(SylvaType::Option(Box::new(inner)))
            }
            _ => Ok(SylvaType::Unknown),
        }
    }

    // ── Module ────────────────────────────────────────────────────────────────

    pub fn parse_module(&mut self, name: &str) -> ParseResult<SylvaModule> {
        let mut items = Vec::new();
        while !self.at(&Token::Eof) {
            items.push(self.parse_item()?);
        }
        Ok(SylvaModule {
            name: name.into(),
            items,
        })
    }

    fn parse_item(&mut self) -> ParseResult<Item> {
        match self.peek().clone() {
            Token::Fn => {
                self.advance(); // consume `fn`
                let fndef = self.parse_fn_def(false)?;
                Ok(Item::FnDef(fndef))
            }
            Token::Async => {
                self.advance();
                self.expect(&Token::Fn)?;
                // parse as async fn
                let fndef = self.parse_fn_def(true)?;
                Ok(Item::FnDef(fndef))
            }
            Token::Let => {
                self.advance();
                let name = self.expect_ident()?;
                let ty = if self.eat(&Token::Colon) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                self.expect(&Token::Assign)?;
                let value = self.parse_expr()?;
                self.eat(&Token::Semicolon);
                Ok(Item::LetDef { name, ty, value })
            }
            Token::Import => {
                self.advance();
                let mut path = vec![self.expect_ident()?];
                while self.eat(&Token::Dot) {
                    path.push(self.expect_ident()?);
                }
                self.eat(&Token::Semicolon);
                Ok(Item::Import(path))
            }
            Token::Export => {
                self.advance();
                let mut names = vec![self.expect_ident()?];
                while self.eat(&Token::Comma) {
                    names.push(self.expect_ident()?);
                }
                self.eat(&Token::Semicolon);
                Ok(Item::Export(names))
            }
            Token::Struct => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(&Token::LBrace)?;
                let mut fields = Vec::new();
                while !self.at(&Token::RBrace) {
                    let fname = self.expect_ident()?;
                    self.expect(&Token::Colon)?;
                    let fty = self.parse_type()?;
                    fields.push((fname, fty));
                    self.eat(&Token::Comma);
                }
                self.expect(&Token::RBrace)?;
                Ok(Item::StructDef { name, fields })
            }
            Token::Enum => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(&Token::LBrace)?;
                let mut variants = Vec::new();
                while !self.at(&Token::RBrace) {
                    let vname = self.expect_ident()?;
                    let payload = if self.eat(&Token::LParen) {
                        let t = self.parse_type()?;
                        self.expect(&Token::RParen)?;
                        Some(t)
                    } else {
                        None
                    };
                    variants.push(EnumVariant {
                        name: vname,
                        payload,
                    });
                    self.eat(&Token::Comma);
                }
                self.expect(&Token::RBrace)?;
                Ok(Item::EnumDef { name, variants })
            }
            _ => {
                let expr = self.parse_expr()?;
                self.eat(&Token::Semicolon);
                Ok(Item::LetDef {
                    name: "_".into(),
                    ty: None,
                    value: expr,
                })
            }
        }
    }

    fn parse_fn_def(&mut self, is_async: bool) -> ParseResult<FnDef> {
        // Use peek_at(1) to disambiguate: consume the function name only when
        // the current token is an Ident AND the token after it is `(`.
        // This correctly handles anonymous lambdas like `fn(x: T) { ... }` where
        // the Ident would actually be a parameter name, not the function name.
        let name = if let Token::Ident(_) = self.peek() {
            if self.peek_at(1) == &Token::LParen {
                Some(self.expect_ident()?)
            } else {
                // Next token after Ident is not `(` — treat as anonymous fn.
                None
            }
        } else {
            None
        };
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let ret_ty = if self.eat(&Token::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block_expr()?;
        Ok(FnDef {
            name,
            params,
            ret_ty,
            body: Box::new(body),
            is_async,
        })
    }

    fn parse_params(&mut self) -> ParseResult<Vec<Param>> {
        let mut params = Vec::new();
        while !self.at(&Token::RParen) && !self.at(&Token::Eof) {
            let name = self.expect_ident()?;
            let ty = if self.eat(&Token::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            let default = if self.eat(&Token::Assign) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            params.push(Param { name, ty, default });
            self.eat(&Token::Comma);
        }
        Ok(params)
    }

    fn parse_block_expr(&mut self) -> ParseResult<Expr> {
        let line = self.line();
        self.expect(&Token::LBrace)?;
        let mut stmts = Vec::new();
        while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(Expr::new(ExprKind::Block(stmts), line))
    }

    fn parse_stmt(&mut self) -> ParseResult<Expr> {
        let line = self.line();
        match self.peek().clone() {
            Token::Let => {
                self.advance();
                let mutable = self.eat(&Token::Mut);
                let name = self.expect_ident()?;
                let ty = if self.eat(&Token::Colon) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                self.expect(&Token::Assign)?;
                let value = self.parse_expr()?;
                self.eat(&Token::Semicolon);
                Ok(Expr::new(
                    ExprKind::Let {
                        name,
                        ty,
                        value: Box::new(value),
                        mutable,
                    },
                    line,
                ))
            }
            Token::Return => {
                self.advance();
                let val = if !self.at(&Token::Semicolon) && !self.at(&Token::RBrace) {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                self.eat(&Token::Semicolon);
                Ok(Expr::new(ExprKind::Return(val), line))
            }
            Token::Break => {
                self.advance();
                let val = if !self.at(&Token::Semicolon) && !self.at(&Token::RBrace) {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                self.eat(&Token::Semicolon);
                Ok(Expr::new(ExprKind::Break(val), line))
            }
            Token::Continue => {
                self.advance();
                self.eat(&Token::Semicolon);
                Ok(Expr::new(ExprKind::Continue, line))
            }
            _ => {
                let expr = self.parse_expr()?;
                self.eat(&Token::Semicolon);
                Ok(expr)
            }
        }
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> ParseResult<Expr> {
        let line = self.line();
        // `let` binding expression
        if self.at(&Token::Let) {
            self.advance();
            let name = self.expect_ident()?;
            let ty = if self.eat(&Token::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect(&Token::Assign)?;
            let value = self.parse_expr()?;
            return Ok(Expr::new(
                ExprKind::Let {
                    name,
                    ty,
                    value: Box::new(value),
                    mutable: false,
                },
                line,
            ));
        }
        let lhs = self.parse_assign()?;
        // Pipeline operator |>
        if self.eat(&Token::Pipe) {
            let rhs = self.parse_expr()?;
            return Ok(Expr::new(
                ExprKind::Pipe(Box::new(lhs), Box::new(rhs)),
                line,
            ));
        }
        Ok(lhs)
    }

    fn parse_assign(&mut self) -> ParseResult<Expr> {
        let line = self.line();
        let lhs = self.parse_or()?;
        if self.eat(&Token::Assign) {
            let rhs = self.parse_assign()?;
            return Ok(Expr::new(
                ExprKind::Assign(Box::new(lhs), Box::new(rhs)),
                line,
            ));
        }
        Ok(lhs)
    }

    fn parse_or(&mut self) -> ParseResult<Expr> {
        let line = self.line();
        let mut lhs = self.parse_and()?;
        while self.at(&Token::Or) {
            self.advance();
            let rhs = self.parse_and()?;
            lhs = Expr::new(
                ExprKind::BinOp(BinOp::Or, Box::new(lhs), Box::new(rhs)),
                line,
            );
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> ParseResult<Expr> {
        let line = self.line();
        let mut lhs = self.parse_compare()?;
        while self.at(&Token::And) {
            self.advance();
            let rhs = self.parse_compare()?;
            lhs = Expr::new(
                ExprKind::BinOp(BinOp::And, Box::new(lhs), Box::new(rhs)),
                line,
            );
        }
        Ok(lhs)
    }

    fn parse_compare(&mut self) -> ParseResult<Expr> {
        let line = self.line();
        let lhs = self.parse_add()?;
        let op = match self.peek() {
            Token::Eqq => Some(BinOp::Eq),
            Token::Ne => Some(BinOp::Ne),
            Token::Lt => Some(BinOp::Lt),
            Token::Le => Some(BinOp::Le),
            Token::Gt => Some(BinOp::Gt),
            Token::Ge => Some(BinOp::Ge),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let rhs = self.parse_add()?;
            return Ok(Expr::new(
                ExprKind::BinOp(op, Box::new(lhs), Box::new(rhs)),
                line,
            ));
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> ParseResult<Expr> {
        let line = self.line();
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Token::Plus => Some(BinOp::Add),
                Token::Minus => Some(BinOp::Sub),
                Token::Concat => Some(BinOp::Concat),
                _ => None,
            };
            match op {
                Some(op) => {
                    self.advance();
                    let r = self.parse_mul()?;
                    lhs = Expr::new(ExprKind::BinOp(op, Box::new(lhs), Box::new(r)), line);
                }
                None => break,
            }
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> ParseResult<Expr> {
        let line = self.line();
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => Some(BinOp::Mul),
                Token::Slash => Some(BinOp::Div),
                Token::Percent => Some(BinOp::Rem),
                _ => None,
            };
            match op {
                Some(op) => {
                    self.advance();
                    let r = self.parse_unary()?;
                    lhs = Expr::new(ExprKind::BinOp(op, Box::new(lhs), Box::new(r)), line);
                }
                None => break,
            }
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> ParseResult<Expr> {
        let line = self.line();
        match self.peek().clone() {
            Token::Not => {
                self.advance();
                let e = self.parse_unary()?;
                Ok(Expr::new(ExprKind::UnOp(UnOp::Not, Box::new(e)), line))
            }
            Token::Minus => {
                self.advance();
                let e = self.parse_unary()?;
                Ok(Expr::new(ExprKind::UnOp(UnOp::Neg, Box::new(e)), line))
            }
            _ => self.parse_call_chain(),
        }
    }

    fn parse_call_chain(&mut self) -> ParseResult<Expr> {
        let line = self.line();
        let mut base = self.parse_atom()?;
        loop {
            match self.peek() {
                Token::LParen => {
                    self.advance();
                    let args = self.parse_arg_list()?;
                    self.expect(&Token::RParen)?;
                    base = Expr::new(ExprKind::Call(Box::new(base), args), line);
                }
                Token::Dot => {
                    self.advance();
                    let field = self.expect_ident()?;
                    if self.at(&Token::LParen) {
                        self.advance();
                        let args = self.parse_arg_list()?;
                        self.expect(&Token::RParen)?;
                        base = Expr::new(ExprKind::MethodCall(Box::new(base), field, args), line);
                    } else {
                        base = Expr::new(ExprKind::Field(Box::new(base), field), line);
                    }
                }
                Token::LBracket => {
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    base = Expr::new(ExprKind::Index(Box::new(base), Box::new(idx)), line);
                }
                Token::Await => {
                    self.advance();
                    base = Expr::new(ExprKind::Await(Box::new(base)), line);
                }
                _ => break,
            }
        }
        Ok(base)
    }

    fn parse_arg_list(&mut self) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();
        while !self.at(&Token::RParen) && !self.at(&Token::Eof) {
            args.push(self.parse_expr()?);
            self.eat(&Token::Comma);
        }
        Ok(args)
    }

    fn parse_atom(&mut self) -> ParseResult<Expr> {
        let line = self.line();
        match self.peek().clone() {
            Token::Nil => {
                self.advance();
                Ok(Expr::new(ExprKind::Nil, line))
            }
            Token::Bool(b) => {
                self.advance();
                Ok(Expr::new(ExprKind::Bool(b), line))
            }
            Token::Int(n) => {
                self.advance();
                Ok(Expr::new(ExprKind::Int(n), line))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Expr::new(ExprKind::Float(f), line))
            }
            Token::Str(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::new(ExprKind::Str(s), line))
            }
            Token::Ident(s) => {
                let s = s.clone();
                self.advance();
                // struct literal: Name { field: value, ... }
                if self.at(&Token::LBrace)
                    && matches!(s.chars().next(), Some(c) if c.is_uppercase())
                {
                    self.advance();
                    let mut fields = Vec::new();
                    while !self.at(&Token::RBrace) {
                        let fname = self.expect_ident()?;
                        self.expect(&Token::Colon)?;
                        let fval = self.parse_expr()?;
                        fields.push((fname, fval));
                        self.eat(&Token::Comma);
                    }
                    self.expect(&Token::RBrace)?;
                    Ok(Expr::new(ExprKind::Struct(s, fields), line))
                } else {
                    Ok(Expr::new(ExprKind::Var(s), line))
                }
            }
            Token::LParen => {
                self.advance();
                if self.at(&Token::RParen) {
                    self.advance();
                    return Ok(Expr::new(ExprKind::Tuple(vec![]), line));
                }
                let first = self.parse_expr()?;
                if self.eat(&Token::Comma) {
                    let mut elems = vec![first];
                    while !self.at(&Token::RParen) {
                        elems.push(self.parse_expr()?);
                        self.eat(&Token::Comma);
                    }
                    self.expect(&Token::RParen)?;
                    Ok(Expr::new(ExprKind::Tuple(elems), line))
                } else {
                    self.expect(&Token::RParen)?;
                    Ok(first)
                }
            }
            Token::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                while !self.at(&Token::RBracket) {
                    elems.push(self.parse_expr()?);
                    self.eat(&Token::Comma);
                }
                self.expect(&Token::RBracket)?;
                Ok(Expr::new(ExprKind::List(elems), line))
            }
            Token::LBrace => self.parse_block_expr(),
            Token::If => {
                self.advance();
                let cond = self.parse_expr()?;
                let then = self.parse_block_expr()?;
                let else_ = if self.eat(&Token::Else) {
                    Some(Box::new(self.parse_block_expr()?))
                } else {
                    None
                };
                Ok(Expr::new(
                    ExprKind::If(Box::new(cond), Box::new(then), else_),
                    line,
                ))
            }
            Token::While => {
                self.advance();
                let cond = self.parse_expr()?;
                let body = self.parse_block_expr()?;
                Ok(Expr::new(
                    ExprKind::While(Box::new(cond), Box::new(body)),
                    line,
                ))
            }
            Token::For => {
                self.advance();
                let var = self.expect_ident()?;
                self.expect(&Token::In)?;
                let iter = self.parse_expr()?;
                let body = self.parse_block_expr()?;
                Ok(Expr::new(
                    ExprKind::For(var, Box::new(iter), Box::new(body)),
                    line,
                ))
            }
            Token::Fn => {
                self.advance();
                let fndef = self.parse_fn_def(false)?;
                Ok(Expr::new(ExprKind::Fn(fndef), line))
            }
            Token::Async => {
                self.advance();
                self.expect(&Token::Fn)?;
                let fndef = self.parse_fn_def(true)?;
                Ok(Expr::new(ExprKind::Fn(fndef), line))
            }
            Token::Match => {
                self.advance();
                let scrutinee = self.parse_expr()?;
                self.expect(&Token::LBrace)?;
                let mut arms = Vec::new();
                while !self.at(&Token::RBrace) {
                    let pattern = self.parse_pattern()?;
                    let guard = if self.eat(&Token::If) {
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };
                    self.expect(&Token::FatArrow)?;
                    let body = self.parse_expr()?;
                    arms.push(MatchArm {
                        pattern,
                        guard,
                        body,
                    });
                    self.eat(&Token::Comma);
                }
                self.expect(&Token::RBrace)?;
                Ok(Expr::new(ExprKind::Match(Box::new(scrutinee), arms), line))
            }
            Token::Spawn => {
                self.advance();
                self.expect(&Token::LParen)?;
                let actor = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(Expr::new(ExprKind::Spawn(Box::new(actor)), line))
            }
            Token::Send => {
                self.advance();
                self.expect(&Token::LParen)?;
                let actor = self.parse_expr()?;
                self.expect(&Token::Comma)?;
                let msg = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(Expr::new(
                    ExprKind::Send(Box::new(actor), Box::new(msg)),
                    line,
                ))
            }
            Token::Receive => {
                self.advance();
                Ok(Expr::new(ExprKind::Receive, line))
            }
            other => Err(err(
                format!("unexpected token in expression: {:?}", other),
                line,
            )),
        }
    }

    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        match self.peek().clone() {
            Token::Ident(s) if s == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Token::Nil => {
                self.advance();
                Ok(Pattern::Nil)
            }
            Token::Bool(b) => {
                self.advance();
                Ok(Pattern::Bool(b))
            }
            Token::Int(n) => {
                self.advance();
                Ok(Pattern::Int(n))
            }
            Token::Str(s) => {
                let s = s.clone();
                self.advance();
                Ok(Pattern::Str(s))
            }
            Token::Ident(s) => {
                let s = s.clone();
                self.advance();
                if self.eat(&Token::LBrace) {
                    // struct pattern
                    let mut fields = Vec::new();
                    while !self.at(&Token::RBrace) {
                        let fname = self.expect_ident()?;
                        self.expect(&Token::Colon)?;
                        let fpat = self.parse_pattern()?;
                        fields.push((fname, fpat));
                        self.eat(&Token::Comma);
                    }
                    self.expect(&Token::RBrace)?;
                    Ok(Pattern::Struct(s, fields))
                } else {
                    Ok(Pattern::Bind(s))
                }
            }
            Token::LParen => {
                self.advance();
                let mut pats = Vec::new();
                while !self.at(&Token::RParen) {
                    pats.push(self.parse_pattern()?);
                    self.eat(&Token::Comma);
                }
                self.expect(&Token::RParen)?;
                Ok(Pattern::Tuple(pats))
            }
            Token::LBracket => {
                self.advance();
                let mut head = Vec::new();
                while !self.at(&Token::RBracket) && !self.at(&Token::DotDot) {
                    head.push(self.parse_pattern()?);
                    self.eat(&Token::Comma);
                }
                let rest = if self.eat(&Token::DotDot) {
                    let name = self.expect_ident()?;
                    Some(Box::new(Pattern::Bind(name)))
                } else {
                    None
                };
                self.expect(&Token::RBracket)?;
                Ok(Pattern::List(head, rest))
            }
            other => Err(err(
                format!("expected pattern, got {:?}", other),
                self.line(),
            )),
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn parse_module(src: &str, name: &str) -> ParseResult<SylvaModule> {
    let tokens = lex(src).map_err(|e| ParseError {
        message: e.0,
        line: e.1,
    })?;
    let mut parser = Parser::new(tokens);
    parser.parse_module(name)
}

pub fn parse_expr(src: &str) -> ParseResult<Expr> {
    let tokens = lex(src).map_err(|e| ParseError {
        message: e.0,
        line: e.1,
    })?;
    let mut parser = Parser::new(tokens);
    parser.parse_expr()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fn() {
        let src = "fn add(a: Int, b: Int) -> Int { a + b }";
        let m = parse_module(src, "test").unwrap();
        assert_eq!(m.items.len(), 1);
        assert!(matches!(m.items[0], Item::FnDef(_)));
    }

    #[test]
    fn parse_let() {
        let expr = parse_expr("let x = 42").unwrap();
        assert!(matches!(expr.kind, ExprKind::Let { .. }));
    }

    #[test]
    fn parse_if_else() {
        let expr = parse_expr("if x > 0 { x } else { 0 }").unwrap();
        assert!(matches!(expr.kind, ExprKind::If(..)));
    }

    #[test]
    fn parse_pipeline() {
        let expr = parse_expr("xs |> filter |> sort").unwrap();
        assert!(matches!(expr.kind, ExprKind::Pipe(..)));
    }

    #[test]
    fn parse_match() {
        let src = r#"match x { 1 => "one", 2 => "two", _ => "other" }"#;
        let expr = parse_expr(src).unwrap();
        assert!(matches!(expr.kind, ExprKind::Match(..)));
    }
}
