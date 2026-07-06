//! Helix lexer — an ordinary token stream. Helix's real uniqueness is in
//! its *type system and execution model* (see `interp.rs`), not lexical
//! shape: swizzle access (`v.xyz`, `v.rgb`) reuses plain `.ident` postfix
//! tokens — the interpreter decides whether an attribute name is a swizzle
//! pattern based on its characters, not a special lexer mode.

use crate::diag::{OmniError, Phase, Pos, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokKind {
    Ident,
    Keyword,
    Int,
    Float,
    Bool,
    Op,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokKind,
    pub value: String,
    pub span: Span,
}

const KEYWORDS: &[&str] = &["kernel", "shader", "vertex", "fragment", "compute", "fn", "let", "if", "else", "for", "in", "return", "true", "false"];

pub struct Lexer<'a> {
    chars: Vec<char>,
    i: usize,
    line: u32,
    col: u32,
    file: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str, file: &'a str) -> Self {
        Lexer { chars: src.chars().collect(), i: 0, line: 1, col: 1, file }
    }

    fn peek(&self, k: usize) -> char {
        self.chars.get(self.i + k).copied().unwrap_or('\0')
    }
    fn advance(&mut self) -> char {
        let c = self.peek(0);
        self.i += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        c
    }
    fn pos(&self) -> Pos {
        Pos { line: self.line, col: self.col }
    }
    fn mk(&self, kind: TokKind, value: impl Into<String>, start: Pos) -> Token {
        Token { kind, value: value.into(), span: Span { start, end: self.pos() } }
    }
    fn err(&self, msg: impl Into<String>, start: Pos) -> OmniError {
        OmniError::new(Phase::Lex, msg, Span { start, end: self.pos() }, self.file)
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, Box<OmniError>> {
        let mut out = Vec::new();
        loop {
            self.skip_ws_and_comments();
            let start = self.pos();
            let c = self.peek(0);
            if c == '\0' {
                out.push(self.mk(TokKind::Eof, "", start));
                break;
            }
            if c.is_ascii_digit() {
                out.push(self.lex_number(start)?);
                continue;
            }
            if is_ident_start(c) {
                out.push(self.lex_ident_or_keyword(start));
                continue;
            }
            out.push(self.lex_op(start)?);
        }
        Ok(out)
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            match self.peek(0) {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                '/' if self.peek(1) == '/' => {
                    while self.peek(0) != '\n' && self.peek(0) != '\0' {
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn lex_number(&mut self, start: Pos) -> Result<Token, Box<OmniError>> {
        // Hex literal (`0xFFu`) — WGSL/Rust-dialect glue code. Kept as its
        // decimal-string equivalent since Helix's numeric model is
        // uniformly f64/i64 internally, not textual.
        if self.peek(0) == '0' && (self.peek(1) == 'x' || self.peek(1) == 'X') {
            self.advance();
            self.advance();
            let mut hex = String::new();
            while self.peek(0).is_ascii_hexdigit() {
                hex.push(self.advance());
            }
            self.skip_numeric_suffix();
            let v = i64::from_str_radix(&hex, 16).unwrap_or(0);
            return Ok(self.mk(TokKind::Int, v.to_string(), start));
        }
        let mut s = String::new();
        let mut is_float = false;
        while self.peek(0).is_ascii_digit() {
            s.push(self.advance());
        }
        if self.peek(0) == '.' && self.peek(1).is_ascii_digit() {
            is_float = true;
            s.push(self.advance());
            while self.peek(0).is_ascii_digit() {
                s.push(self.advance());
            }
        }
        // Scientific notation (`1e-8`, `1e30`) — common in shader constant
        // expressions.
        if (self.peek(0) == 'e' || self.peek(0) == 'E') && (self.peek(1).is_ascii_digit() || ((self.peek(1) == '+' || self.peek(1) == '-') && self.peek(2).is_ascii_digit())) {
            is_float = true;
            s.push(self.advance()); // 'e'/'E'
            if self.peek(0) == '+' || self.peek(0) == '-' {
                s.push(self.advance());
            }
            while self.peek(0).is_ascii_digit() {
                s.push(self.advance());
            }
        }
        // Trailing `f`/`f32`/`u`/`u32`/`i32` suffix, common in shader
        // literals — consumed and ignored (Helix's numeric model is
        // uniformly f64/i64 internally, not width-typed).
        if matches!(self.peek(0), 'f' | 'u' | 'i') && !is_ident_continue(self.peek(1)) {
            is_float = is_float || self.peek(0) == 'f';
            self.advance();
        } else if matches!(self.peek(0), 'f' | 'u' | 'i') && self.peek(1).is_ascii_digit() {
            is_float = is_float || self.peek(0) == 'f';
            self.skip_numeric_suffix();
        }
        Ok(self.mk(if is_float { TokKind::Float } else { TokKind::Int }, s, start))
    }

    /// Consumes a `u32`/`f32`/`i32`-shaped width suffix fused directly onto
    /// a numeric literal with no separating whitespace.
    fn skip_numeric_suffix(&mut self) {
        if matches!(self.peek(0), 'u' | 'f' | 'i') {
            self.advance();
            while self.peek(0).is_ascii_digit() {
                self.advance();
            }
        }
    }

    fn lex_ident_or_keyword(&mut self, start: Pos) -> Token {
        let mut s = String::new();
        while is_ident_continue(self.peek(0)) {
            s.push(self.advance());
        }
        if s == "true" || s == "false" {
            return self.mk(TokKind::Bool, s, start);
        }
        if KEYWORDS.contains(&s.as_str()) {
            return self.mk(TokKind::Keyword, s, start);
        }
        self.mk(TokKind::Ident, s, start)
    }

    fn lex_op(&mut self, start: Pos) -> Result<Token, Box<OmniError>> {
        let c = self.advance();
        macro_rules! two {
            ($next:expr, $tok:expr, $else_tok:expr) => {
                if self.peek(0) == $next {
                    self.advance();
                    $tok
                } else {
                    $else_tok
                }
            };
        }
        let s: String = match c {
            '=' => two!('=', "==".to_string(), "=".to_string()),
            '!' => two!('=', "!=".to_string(), "!".to_string()),
            '<' => {
                if self.peek(0) == '=' {
                    self.advance();
                    "<=".to_string()
                } else if self.peek(0) == '<' {
                    self.advance();
                    "<<".to_string()
                } else {
                    "<".to_string()
                }
            }
            '>' => {
                if self.peek(0) == '=' {
                    self.advance();
                    ">=".to_string()
                } else if self.peek(0) == '>' {
                    self.advance();
                    ">>".to_string()
                } else {
                    ">".to_string()
                }
            }
            '+' => two!('=', "+=".to_string(), "+".to_string()),
            '-' => {
                if self.peek(0) == '=' {
                    self.advance();
                    "-=".to_string()
                } else if self.peek(0) == '>' {
                    self.advance();
                    "->".to_string()
                } else {
                    "-".to_string()
                }
            }
            '*' => two!('=', "*=".to_string(), "*".to_string()),
            '/' => two!('=', "/=".to_string(), "/".to_string()),
            '%' => "%".to_string(),
            '(' => "(".to_string(),
            ')' => ")".to_string(),
            '[' => "[".to_string(),
            ']' => "]".to_string(),
            '{' => "{".to_string(),
            '}' => "}".to_string(),
            ',' => ",".to_string(),
            ':' => two!(':', "::".to_string(), ":".to_string()),
            '.' => {
                if self.peek(0) == '.' {
                    self.advance();
                    "..".to_string()
                } else {
                    ".".to_string()
                }
            }
            ';' => ";".to_string(),
            '|' => two!('|', "||".to_string(), "|".to_string()),
            '&' => two!('&', "&&".to_string(), "&".to_string()),
            '^' => "^".to_string(),
            '@' => "@".to_string(),
            other => return Err(Box::new(self.err(format!("unexpected character '{other}'"), start))),
        };
        Ok(self.mk(TokKind::Op, s, start))
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}
fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
