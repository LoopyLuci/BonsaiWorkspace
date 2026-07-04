//! Sylva-subset surface syntax → UniIR recursive-descent parser.
//!
//! Grammar (simplified):
//!
//! ```text
//! module   ::= fn_def*
//! fn_def   ::= ["@" device] "fn" IDENT "(" params ")" ["->" type] "{" expr "}"
//! params   ::= (IDENT ":" type ("," IDENT ":" type)*)?
//! type     ::= "Unit" | "Bool" | "Int" | "Float" | "Str" | "Bytes"
//!            | "[" type "]"          -- Array
//!            | "(" type ")"          -- parens
//!            | "fn" "(" types ")" "->" type
//!            | "ActorRef" "<" type ">"
//!            | "DataFrame"
//!            | "NDArray" "<" type ">"
//!            | IDENT                  -- Named
//! expr     ::= let_expr | if_expr | lambda | spawn | send | receive
//!            | "@" device expr        -- device annotation
//!            | "@sql" "(" STR ")"    -- sql query
//!            | bin_expr
//! let_expr ::= "let" IDENT [":" type] "=" expr ";" expr
//! if_expr  ::= "if" expr "{" expr "}" "else" "{" expr "}"
//! lambda   ::= "fn" "(" params ")" "{" expr "}"
//! spawn    ::= "spawn" "<" type ">" "(" expr ")"
//! send     ::= "send" "(" expr "," expr ")"
//! receive  ::= "receive" "<" type ">"
//! bin_expr ::= un_expr (BIN_OP un_expr)*
//! un_expr  ::= [UN_OP] call_expr
//! call_expr::= atom ("(" args ")")?  ("." IDENT)*
//! atom     ::= INT | FLOAT | BOOL | STR | IDENT | "(" expr ")" | "[" args "]" | "(" args ")"
//! ```

use crate::ops::*;

// ── Lexer ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Unit,

    // Identifiers and keywords
    Ident(String),
    Fn,
    Let,
    If,
    Else,
    Return,
    Loop,
    Break,
    Continue,
    Spawn,
    Send,
    Receive,
    Ask,
    Match,
    Sql, // @sql

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Assign, // =

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LAngle,
    RAngle,
    Comma,
    Colon,
    Semicolon,
    Arrow,
    Dot,
    At,
    DotDot,

    // Devices
    DeviceCpu,
    DeviceGpu,
    DeviceFpga,
    DeviceTpu,
    DeviceAuto,

    Eof,
}

#[derive(Debug)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error: {}", self.0)
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

fn err(msg: impl Into<String>) -> ParseError {
    ParseError(msg.into())
}

// ── Tokenizer ────────────────────────────────────────────────────────────────

pub fn tokenize(src: &str) -> ParseResult<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\r' | '\n' => {
                i += 1;
            }
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '"' => {
                i += 1;
                let mut s = String::new();
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        i += 1;
                        match chars.get(i) {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some('"') => s.push('"'),
                            Some('\\') => s.push('\\'),
                            _ => s.push('\\'),
                        }
                    } else {
                        s.push(chars[i]);
                    }
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(err("unterminated string"));
                }
                i += 1;
                tokens.push(Token::Str(s));
            }
            '@' => {
                i += 1;
                let mut kw = String::new();
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    kw.push(chars[i]);
                    i += 1;
                }
                match kw.as_str() {
                    "cpu" => tokens.push(Token::DeviceCpu),
                    "gpu" => tokens.push(Token::DeviceGpu),
                    "fpga" => tokens.push(Token::DeviceFpga),
                    "tpu" => tokens.push(Token::DeviceTpu),
                    "auto" => tokens.push(Token::DeviceAuto),
                    "sql" => tokens.push(Token::Sql),
                    _ => tokens.push(Token::At),
                }
            }
            '-' if chars.get(i + 1) == Some(&'>') => {
                tokens.push(Token::Arrow);
                i += 2;
            }
            '.' if chars.get(i + 1) == Some(&'.') => {
                tokens.push(Token::DotDot);
                i += 2;
            }
            '=' if chars.get(i + 1) == Some(&'=') => {
                tokens.push(Token::Eq);
                i += 2;
            }
            '!' if chars.get(i + 1) == Some(&'=') => {
                tokens.push(Token::Ne);
                i += 2;
            }
            '<' if chars.get(i + 1) == Some(&'=') => {
                tokens.push(Token::Le);
                i += 2;
            }
            '>' if chars.get(i + 1) == Some(&'=') => {
                tokens.push(Token::Ge);
                i += 2;
            }
            '<' if chars.get(i + 1) == Some(&'<') => {
                tokens.push(Token::Shl);
                i += 2;
            }
            '>' if chars.get(i + 1) == Some(&'>') => {
                tokens.push(Token::Shr);
                i += 2;
            }
            '&' if chars.get(i + 1) == Some(&'&') => {
                tokens.push(Token::And);
                i += 2;
            }
            '|' if chars.get(i + 1) == Some(&'|') => {
                tokens.push(Token::Or);
                i += 2;
            }
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '%' => {
                tokens.push(Token::Percent);
                i += 1;
            }
            '<' => {
                tokens.push(Token::Lt);
                i += 1;
            }
            '>' => {
                tokens.push(Token::Gt);
                i += 1;
            }
            '&' => {
                tokens.push(Token::BitAnd);
                i += 1;
            }
            '|' => {
                tokens.push(Token::BitOr);
                i += 1;
            }
            '^' => {
                tokens.push(Token::BitXor);
                i += 1;
            }
            '!' => {
                tokens.push(Token::Not);
                i += 1;
            }
            '=' => {
                tokens.push(Token::Assign);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '{' => {
                tokens.push(Token::LBrace);
                i += 1;
            }
            '}' => {
                tokens.push(Token::RBrace);
                i += 1;
            }
            '[' => {
                tokens.push(Token::LBracket);
                i += 1;
            }
            ']' => {
                tokens.push(Token::RBracket);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            ':' => {
                tokens.push(Token::Colon);
                i += 1;
            }
            ';' => {
                tokens.push(Token::Semicolon);
                i += 1;
            }
            '.' => {
                tokens.push(Token::Dot);
                i += 1;
            }
            _ if c.is_ascii_digit()
                || (c == '-' && chars.get(i + 1).is_some_and(|x| x.is_ascii_digit())) =>
            {
                let start = i;
                if c == '-' {
                    i += 1;
                }
                while i < chars.len() && (chars[i].is_ascii_digit()) {
                    i += 1;
                }
                let is_float = i < chars.len() && chars[i] == '.';
                if is_float {
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    let s: String = chars[start..i].iter().collect();
                    tokens.push(Token::Float(
                        s.parse().map_err(|_| err(format!("bad float: {s}")))?,
                    ));
                } else {
                    let s: String = chars[start..i].iter().collect();
                    tokens.push(Token::Int(
                        s.parse().map_err(|_| err(format!("bad int: {s}")))?,
                    ));
                }
            }
            _ if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                tokens.push(match word.as_str() {
                    "fn" => Token::Fn,
                    "let" => Token::Let,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "return" => Token::Return,
                    "loop" => Token::Loop,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "spawn" => Token::Spawn,
                    "send" => Token::Send,
                    "receive" => Token::Receive,
                    "ask" => Token::Ask,
                    "match" => Token::Match,
                    "true" => Token::Bool(true),
                    "false" => Token::Bool(false),
                    "unit" => Token::Unit,
                    _ => Token::Ident(word),
                });
            }
            other => return Err(err(format!("unexpected char: {other:?}"))),
        }
    }
    tokens.push(Token::Eof);
    Ok(tokens)
}

// ── Parser ───────────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek2(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
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
            Err(err(format!("expected {:?}, got {:?}", tok, self.peek())))
        }
    }

    fn expect_ident(&mut self) -> ParseResult<String> {
        match self.peek().clone() {
            Token::Ident(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(err(format!("expected identifier, got {:?}", other))),
        }
    }

    // ── Types ─────────────────────────────────────────────────────────────────

    fn parse_type(&mut self) -> ParseResult<IrType> {
        match self.peek().clone() {
            Token::Ident(ref name) => {
                let name = name.clone();
                self.advance();
                match name.as_str() {
                    "Unit" => Ok(IrType::Unit),
                    "Bool" => Ok(IrType::Bool),
                    "Int" => Ok(IrType::I64),
                    "Float" => Ok(IrType::F64),
                    "Str" => Ok(IrType::Str),
                    "Bytes" => Ok(IrType::Bytes),
                    "DataFrame" => Ok(IrType::DataFrame),
                    "ActorRef" => {
                        self.expect(&Token::Lt)?;
                        let inner = self.parse_type()?;
                        self.expect(&Token::Gt)?;
                        Ok(IrType::ActorRef(Box::new(inner)))
                    }
                    "NDArray" => {
                        self.expect(&Token::Lt)?;
                        let inner = self.parse_type()?;
                        self.expect(&Token::Gt)?;
                        Ok(IrType::NDArray(Box::new(inner)))
                    }
                    _ => Ok(IrType::Named(name)),
                }
            }
            Token::LBracket => {
                self.advance();
                let inner = self.parse_type()?;
                self.expect(&Token::RBracket)?;
                Ok(IrType::Array(Box::new(inner)))
            }
            Token::Fn => {
                self.advance();
                self.expect(&Token::LParen)?;
                let mut params = Vec::new();
                while self.peek() != &Token::RParen {
                    params.push(self.parse_type()?);
                    if self.peek() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect(&Token::RParen)?;
                self.expect(&Token::Arrow)?;
                let ret = self.parse_type()?;
                Ok(IrType::fun(params, ret))
            }
            Token::LParen => {
                self.advance();
                let ty = self.parse_type()?;
                self.expect(&Token::RParen)?;
                Ok(ty)
            }
            other => Err(err(format!("expected type, got {:?}", other))),
        }
    }

    // ── Module ────────────────────────────────────────────────────────────────

    pub fn parse_module(&mut self, name: &str) -> ParseResult<IrModule> {
        let mut module = IrModule::new(name);
        while self.peek() != &Token::Eof {
            // optional device annotation on fn
            // peek2() disambiguates: only consume the device token when the next
            // token is `fn`, avoiding false positives for bare @-identifiers.
            let device = if matches!(
                self.peek(),
                Token::DeviceCpu
                    | Token::DeviceGpu
                    | Token::DeviceFpga
                    | Token::DeviceTpu
                    | Token::DeviceAuto
            ) && self.peek2() == Some(&Token::Fn)
            {
                Some(self.parse_device())
            } else {
                None
            };
            let func = self.parse_fn(device)?;
            let name = func.name.clone();
            module.exports.push(name);
            module.functions.push(func);
        }
        Ok(module)
    }

    fn parse_device(&mut self) -> DeviceTarget {
        match self.advance().clone() {
            Token::DeviceCpu => DeviceTarget::Cpu,
            Token::DeviceGpu => DeviceTarget::Gpu,
            Token::DeviceFpga => DeviceTarget::Fpga,
            Token::DeviceTpu => DeviceTarget::Tpu,
            _ => DeviceTarget::Auto,
        }
    }

    fn parse_fn(&mut self, device: Option<DeviceTarget>) -> ParseResult<IrFunction> {
        self.expect(&Token::Fn)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;

        let mut params = Vec::new();
        while self.peek() != &Token::RParen {
            let pname = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let pty = self.parse_type()?;
            params.push(IrParam {
                name: pname,
                ty: pty,
                default: None,
            });
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(&Token::RParen)?;

        let ret = if self.peek() == &Token::Arrow {
            self.advance();
            self.parse_type()?
        } else {
            IrType::Unit
        };

        self.expect(&Token::LBrace)?;
        let mut body = self.parse_expr()?;
        self.expect(&Token::RBrace)?;

        if let Some(dev) = device {
            body = IrOp::DeviceAnnotation {
                target: dev,
                expr: Box::new(body),
            };
        }

        Ok(IrFunction {
            name,
            params,
            ret,
            body,
            effects: vec![],
            proof: None,
            schema: None,
        })
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    fn parse_expr(&mut self) -> ParseResult<IrOp> {
        match self.peek().clone() {
            Token::Let => self.parse_let(),
            Token::If => self.parse_if(),
            Token::Fn => self.parse_lambda(),
            Token::Return => {
                self.advance();
                let val = self.parse_expr()?;
                Ok(IrOp::Return(Box::new(val)))
            }
            Token::Loop => {
                self.advance();
                self.expect(&Token::LBrace)?;
                let body = self.parse_block()?;
                self.expect(&Token::RBrace)?;
                Ok(IrOp::Loop(Box::new(body)))
            }
            Token::Break => {
                self.advance();
                let val = if self.peek() != &Token::Semicolon && self.peek() != &Token::RBrace {
                    self.parse_expr()?
                } else {
                    IrOp::unit()
                };
                Ok(IrOp::Break(Box::new(val)))
            }
            Token::Continue => {
                self.advance();
                Ok(IrOp::Continue)
            }
            Token::Spawn => self.parse_spawn(),
            Token::Send => self.parse_send(),
            Token::Receive => self.parse_receive(),
            Token::Ask => self.parse_ask(),
            Token::Sql => self.parse_sql(),
            Token::DeviceCpu
            | Token::DeviceGpu
            | Token::DeviceFpga
            | Token::DeviceTpu
            | Token::DeviceAuto => {
                let dev = self.parse_device();
                let expr = self.parse_expr()?;
                Ok(IrOp::DeviceAnnotation {
                    target: dev,
                    expr: Box::new(expr),
                })
            }
            Token::Match => self.parse_match(),
            _ => self.parse_bin_expr(0),
        }
    }

    fn parse_block(&mut self) -> ParseResult<IrOp> {
        let mut stmts = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            let expr = self.parse_expr()?;
            stmts.push(expr);
            if self.peek() == &Token::Semicolon {
                self.advance();
            }
        }
        Ok(if stmts.len() == 1 {
            stmts.pop().unwrap()
        } else {
            IrOp::Block(stmts)
        })
    }

    fn parse_let(&mut self) -> ParseResult<IrOp> {
        self.expect(&Token::Let)?;
        let name = self.expect_ident()?;
        let ty = if self.peek() == &Token::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&Token::Assign)?;
        let value = self.parse_expr()?;
        self.expect(&Token::Semicolon)?;
        let rest = self.parse_expr()?;
        Ok(IrOp::Let {
            name,
            ty,
            value: Box::new(value),
            rest: Box::new(rest),
        })
    }

    fn parse_if(&mut self) -> ParseResult<IrOp> {
        self.expect(&Token::If)?;
        let cond = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        let then = self.parse_block()?;
        self.expect(&Token::RBrace)?;
        self.expect(&Token::Else)?;
        self.expect(&Token::LBrace)?;
        let else_ = self.parse_block()?;
        self.expect(&Token::RBrace)?;
        Ok(IrOp::if_(cond, then, else_))
    }

    fn parse_lambda(&mut self) -> ParseResult<IrOp> {
        self.expect(&Token::Fn)?;
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Token::RParen {
            let pname = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let pty = self.parse_type()?;
            params.push((pname, pty));
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(&Token::RParen)?;
        let ret = if self.peek() == &Token::Arrow {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(&Token::RBrace)?;
        Ok(IrOp::Lambda {
            params,
            ret,
            body: Box::new(body),
        })
    }

    fn parse_spawn(&mut self) -> ParseResult<IrOp> {
        self.expect(&Token::Spawn)?;
        self.expect(&Token::Lt)?;
        let ty = self.parse_type()?;
        self.expect(&Token::Gt)?;
        self.expect(&Token::LParen)?;
        let init = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        Ok(IrOp::Spawn {
            actor_ty: ty,
            init_msg: Box::new(init),
        })
    }

    fn parse_send(&mut self) -> ParseResult<IrOp> {
        self.expect(&Token::Send)?;
        self.expect(&Token::LParen)?;
        let actor = self.parse_expr()?;
        self.expect(&Token::Comma)?;
        let msg = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        Ok(IrOp::Send {
            actor_ref: Box::new(actor),
            msg: Box::new(msg),
        })
    }

    fn parse_receive(&mut self) -> ParseResult<IrOp> {
        self.expect(&Token::Receive)?;
        self.expect(&Token::Lt)?;
        let ty = self.parse_type()?;
        self.expect(&Token::Gt)?;
        Ok(IrOp::Receive { msg_ty: ty })
    }

    fn parse_ask(&mut self) -> ParseResult<IrOp> {
        self.expect(&Token::Ask)?;
        self.expect(&Token::LParen)?;
        let actor = self.parse_expr()?;
        self.expect(&Token::Comma)?;
        let msg = self.parse_expr()?;
        self.expect(&Token::Comma)?;
        let reply_ty = self.parse_type()?;
        self.expect(&Token::RParen)?;
        Ok(IrOp::Ask {
            actor_ref: Box::new(actor),
            msg: Box::new(msg),
            reply_ty,
        })
    }

    fn parse_sql(&mut self) -> ParseResult<IrOp> {
        self.expect(&Token::Sql)?;
        self.expect(&Token::LParen)?;
        let query = match self.advance().clone() {
            Token::Str(s) => s,
            other => return Err(err(format!("@sql expects string literal, got {:?}", other))),
        };
        let mut params = Vec::new();
        while self.peek() == &Token::Comma {
            self.advance();
            params.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(IrOp::SqlQuery { query, params })
    }

    fn parse_match(&mut self) -> ParseResult<IrOp> {
        self.expect(&Token::Match)?;
        let scrutinee = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        let mut arms = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            let pat = self.parse_pattern()?;
            self.expect(&Token::Arrow)?;
            let body = self.parse_expr()?;
            arms.push((pat, body));
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(IrOp::Match {
            scrutinee: Box::new(scrutinee),
            arms,
        })
    }

    fn parse_pattern(&mut self) -> ParseResult<IrPattern> {
        match self.peek().clone() {
            Token::Ident(s) if s == "_" => {
                self.advance();
                Ok(IrPattern::Wildcard)
            }
            Token::Ident(s) => {
                let s = s.clone();
                self.advance();
                // Check if it's a variant with fields: Name(p1, p2, ...)
                if self.peek() == &Token::LParen {
                    self.advance();
                    let mut fields = Vec::new();
                    while self.peek() != &Token::RParen {
                        fields.push(self.parse_pattern()?);
                        if self.peek() == &Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RParen)?;
                    Ok(IrPattern::Variant { name: s, fields })
                } else {
                    Ok(IrPattern::Bind(s))
                }
            }
            Token::Int(n) => {
                self.advance();
                Ok(IrPattern::Lit(IrLit::I64(n)))
            }
            Token::Bool(b) => {
                self.advance();
                Ok(IrPattern::Lit(IrLit::Bool(b)))
            }
            Token::Str(s) => {
                let s = s.clone();
                self.advance();
                Ok(IrPattern::Lit(IrLit::Str(s)))
            }
            Token::LParen => {
                self.advance();
                let mut pats = Vec::new();
                while self.peek() != &Token::RParen {
                    pats.push(self.parse_pattern()?);
                    if self.peek() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect(&Token::RParen)?;
                Ok(IrPattern::Tuple(pats))
            }
            other => Err(err(format!("expected pattern, got {:?}", other))),
        }
    }

    // ── Binary expressions (Pratt precedence) ─────────────────────────────────

    fn prec(tok: &Token) -> Option<(u8, BinOpKind)> {
        use BinOpKind::*;
        Some(match tok {
            Token::Or => (1, Or),
            Token::And => (2, And),
            Token::BitOr => (3, BitOr),
            Token::BitXor => (4, BitXor),
            Token::BitAnd => (5, BitAnd),
            Token::Eq => (6, Eq),
            Token::Ne => (6, Ne),
            Token::Lt => (7, Lt),
            Token::Le => (7, Le),
            Token::Gt => (7, Gt),
            Token::Ge => (7, Ge),
            Token::Shl => (8, Shl),
            Token::Shr => (8, Shr),
            Token::Plus => (9, Add),
            Token::Minus => (9, Sub),
            Token::Star => (10, Mul),
            Token::Slash => (10, Div),
            Token::Percent => (10, Rem),
            _ => return None,
        })
    }

    fn parse_bin_expr(&mut self, min_prec: u8) -> ParseResult<IrOp> {
        let mut lhs = self.parse_unary()?;
        loop {
            let (prec, op) = match Self::prec(self.peek()) {
                Some(p) if p.0 >= min_prec => p,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_bin_expr(prec + 1)?;
            lhs = IrOp::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> ParseResult<IrOp> {
        match self.peek().clone() {
            Token::Not => {
                self.advance();
                let e = self.parse_call()?;
                Ok(IrOp::UnOp {
                    op: UnOpKind::Not,
                    expr: Box::new(e),
                })
            }
            Token::Minus => {
                self.advance();
                let e = self.parse_call()?;
                Ok(IrOp::UnOp {
                    op: UnOpKind::Neg,
                    expr: Box::new(e),
                })
            }
            Token::Star => {
                self.advance();
                let e = self.parse_call()?;
                Ok(IrOp::UnOp {
                    op: UnOpKind::Deref,
                    expr: Box::new(e),
                })
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> ParseResult<IrOp> {
        let mut base = self.parse_atom()?;
        loop {
            match self.peek() {
                Token::LParen => {
                    self.advance();
                    let args = self.parse_arg_list()?;
                    self.expect(&Token::RParen)?;
                    base = IrOp::Apply {
                        func: Box::new(base),
                        args,
                    };
                }
                Token::Dot => {
                    self.advance();
                    let field = self.expect_ident()?;
                    // method call: base.field(args)
                    if self.peek() == &Token::LParen {
                        self.advance();
                        let args = self.parse_arg_list()?;
                        self.expect(&Token::RParen)?;
                        // lower to ToolCall if it looks like a bonsai.X() call
                        base = IrOp::Apply {
                            func: Box::new(IrOp::FieldAccess {
                                expr: Box::new(base),
                                field,
                            }),
                            args,
                        };
                    } else {
                        base = IrOp::FieldAccess {
                            expr: Box::new(base),
                            field,
                        };
                    }
                }
                Token::LBracket => {
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    base = IrOp::IndexAccess {
                        expr: Box::new(base),
                        index: Box::new(idx),
                    };
                }
                _ => break,
            }
        }
        Ok(base)
    }

    fn parse_arg_list(&mut self) -> ParseResult<Vec<IrOp>> {
        let mut args = Vec::new();
        while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
            args.push(self.parse_expr()?);
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        Ok(args)
    }

    fn parse_atom(&mut self) -> ParseResult<IrOp> {
        match self.peek().clone() {
            Token::Int(n) => {
                self.advance();
                Ok(IrOp::lit_i64(n))
            }
            Token::Float(f) => {
                self.advance();
                Ok(IrOp::Lit(IrLit::F64(f)))
            }
            Token::Bool(b) => {
                self.advance();
                Ok(IrOp::lit_bool(b))
            }
            Token::Str(s) => {
                let s = s.clone();
                self.advance();
                Ok(IrOp::lit_str(s))
            }
            Token::Unit => {
                self.advance();
                Ok(IrOp::unit())
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(IrOp::var(name))
            }
            Token::LParen => {
                self.advance();
                if self.peek() == &Token::RParen {
                    self.advance();
                    return Ok(IrOp::unit());
                }
                let first = self.parse_expr()?;
                if self.peek() == &Token::Comma {
                    // tuple
                    let mut elems = vec![first];
                    while self.peek() == &Token::Comma {
                        self.advance();
                        if self.peek() == &Token::RParen {
                            break;
                        }
                        elems.push(self.parse_expr()?);
                    }
                    self.expect(&Token::RParen)?;
                    Ok(IrOp::Tuple(elems))
                } else {
                    self.expect(&Token::RParen)?;
                    Ok(first)
                }
            }
            Token::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                    elems.push(self.parse_expr()?);
                    if self.peek() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(IrOp::Array(elems))
            }
            Token::LBrace => {
                self.advance();
                let block = self.parse_block()?;
                self.expect(&Token::RBrace)?;
                Ok(block)
            }
            other => Err(err(format!("unexpected token in expression: {:?}", other))),
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a Sylva-subset source string into an `IrModule`.
pub fn parse(src: &str, module_name: &str) -> ParseResult<IrModule> {
    let tokens = tokenize(src)?;
    let mut parser = Parser::new(tokens);
    parser.parse_module(module_name)
}

/// Parse a single expression from source.
pub fn parse_expr(src: &str) -> ParseResult<IrOp> {
    let tokens = tokenize(src)?;
    let mut parser = Parser::new(tokens);
    parser.parse_expr()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_fn() {
        let src = r#"fn add(a: Int, b: Int) -> Int { a + b }"#;
        let m = parse(src, "test").unwrap();
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.functions[0].name, "add");
    }

    #[test]
    fn parse_let_binding() {
        let src = r#"fn double(x: Int) -> Int { let y = x + x; y }"#;
        let m = parse(src, "test").unwrap();
        assert!(matches!(m.functions[0].body, IrOp::Let { .. }));
    }

    #[test]
    fn parse_if_else() {
        let src = r#"fn abs(x: Int) -> Int { if x >= 0 { x } else { 0 - x } }"#;
        let m = parse(src, "test").unwrap();
        assert!(matches!(m.functions[0].body, IrOp::If { .. }));
    }

    #[test]
    fn parse_device_annotation() {
        let src = r#"@gpu fn matmul(a: NDArray<Float>, b: NDArray<Float>) -> NDArray<Float> { a }"#;
        let m = parse(src, "test").unwrap();
        assert!(matches!(
            m.functions[0].body,
            IrOp::DeviceAnnotation {
                target: DeviceTarget::Gpu,
                ..
            }
        ));
    }

    #[test]
    fn parse_spawn_send() {
        let src = r#"fn start_worker(msg: Str) -> Unit {
            let ref = spawn<Str>(msg);
            send(ref, "hello")
        }"#;
        let m = parse(src, "test").unwrap();
        assert!(matches!(m.functions[0].body, IrOp::Let { .. }));
    }

    #[test]
    fn parse_sql_query() {
        let src = r#"fn get_users() -> Str { @sql("SELECT * FROM users") }"#;
        let m = parse(src, "test").unwrap();
        assert!(matches!(m.functions[0].body, IrOp::SqlQuery { .. }));
    }

    #[test]
    fn tokenize_operators() {
        let toks = tokenize("a + b * c").unwrap();
        assert!(toks.contains(&Token::Plus));
        assert!(toks.contains(&Token::Star));
    }

    #[test]
    fn parse_expr_arithmetic() {
        let op = parse_expr("1 + 2 * 3").unwrap();
        // Should be 1 + (2 * 3) due to precedence
        match op {
            IrOp::BinOp {
                op: BinOpKind::Add,
                lhs,
                rhs,
            } => {
                assert!(matches!(*lhs, IrOp::Lit(IrLit::I64(1))));
                assert!(matches!(
                    *rhs,
                    IrOp::BinOp {
                        op: BinOpKind::Mul,
                        ..
                    }
                ));
            }
            _ => panic!("expected BinOp::Add at top level"),
        }
    }
}
