//! Nexus lexer — an ordinary token stream. Nexus's real uniqueness isn't in
//! its lexical shape; it's that the language is **declarative and
//! constraint-based** rather than a general-purpose imperative language at
//! all (see `interp.rs`'s module doc comment). CSS/YAML-style `key: value`
//! property syntax (not `=`) is used deliberately, matching the languages
//! this absorbs.

use crate::diag::{OmniError, Phase, Pos, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokKind {
    Ident,
    Keyword,
    Int,
    Float,
    Str,
    Op,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokKind,
    pub value: String,
    pub span: Span,
}

const KEYWORDS: &[&str] = &["box", "layout", "constrain", "direction", "row", "column", "children", "in", "parent"];

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
            if c == '"' {
                out.push(self.lex_string(start)?);
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
                ' ' | '\t' | '\r' | '\n' | ',' => {
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
        Ok(self.mk(if is_float { TokKind::Float } else { TokKind::Int }, s, start))
    }

    fn lex_string(&mut self, start: Pos) -> Result<Token, Box<OmniError>> {
        self.advance();
        let mut s = String::new();
        loop {
            let c = self.peek(0);
            if c == '\0' {
                return Err(Box::new(self.err("unterminated string literal", start)));
            }
            if c == '"' {
                self.advance();
                break;
            }
            s.push(self.advance());
        }
        Ok(self.mk(TokKind::Str, s, start))
    }

    fn lex_ident_or_keyword(&mut self, start: Pos) -> Token {
        let mut s = String::new();
        while is_ident_continue(self.peek(0)) {
            s.push(self.advance());
        }
        if KEYWORDS.contains(&s.as_str()) {
            return self.mk(TokKind::Keyword, s, start);
        }
        self.mk(TokKind::Ident, s, start)
    }

    fn lex_op(&mut self, start: Pos) -> Result<Token, Box<OmniError>> {
        let c = self.advance();
        let s: String = match c {
            '=' => {
                if self.peek(0) == '=' {
                    self.advance();
                    "==".to_string()
                } else {
                    return Err(Box::new(self.err("bare '=' is not valid — did you mean ':' (property) or '==' (constraint)?", start)));
                }
            }
            '>' => {
                if self.peek(0) == '=' {
                    self.advance();
                    ">=".to_string()
                } else {
                    ">".to_string()
                }
            }
            '<' => {
                if self.peek(0) == '=' {
                    self.advance();
                    "<=".to_string()
                } else {
                    "<".to_string()
                }
            }
            '+' => "+".to_string(),
            '-' => "-".to_string(),
            '*' => "*".to_string(),
            '/' => "/".to_string(),
            '(' => "(".to_string(),
            ')' => ")".to_string(),
            '[' => "[".to_string(),
            ']' => "]".to_string(),
            '{' => "{".to_string(),
            '}' => "}".to_string(),
            ':' => ":".to_string(),
            '.' => ".".to_string(),
            '@' => "@".to_string(),
            ';' => ";".to_string(),
            '%' => "%".to_string(),
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
