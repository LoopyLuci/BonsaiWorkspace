//! Aether lexer — genuinely Erlang/OTP + Elixir-flavored, the third distinct
//! block-delimiter convention across the Omni-Languages (Titan=braces,
//! Sylva=indentation, **Aether=`do`/`end` keywords**). Distinctive tokens:
//! atoms (`:ok`, `:error`), the pipe operator (`|>`), `#{}` string
//! interpolation, and `->` clause arrows for pattern-matched function heads.

use crate::diag::{OmniError, Phase, Pos, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokKind {
    Ident,
    Keyword,
    Atom, // :ok, :error, :some_atom
    Int,
    Float,
    Str,
    IStr, // interpolated string: raw template text with #{expr} splices
    Bool,
    Nil,
    Op,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokKind,
    pub value: String,
    pub span: Span,
}

const KEYWORDS: &[&str] = &[
    "defmodule", "def", "defp", "do", "end", "when", "if", "unless", "else", "cond", "case",
    "for", "in", "while", "actor", "spawn", "receive", "after", "supervisor", "worker",
    "one_for_one", "one_for_all", "rest_for_one", "import", "return", "break", "continue",
    "and", "or", "not", "true", "false", "nil",
];

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
            if c == ':' && (is_ident_start(self.peek(1))) {
                out.push(self.lex_atom(start));
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
                '#' if self.peek(1) != '{' => {
                    while self.peek(0) != '\n' && self.peek(0) != '\0' {
                        self.advance();
                    }
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
        while self.peek(0).is_ascii_digit() || self.peek(0) == '_' {
            let c = self.advance();
            if c != '_' {
                s.push(c);
            }
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

    /// String literal. If it contains a `#{...}` splice anywhere, it's
    /// tokenized as `IStr` (interpolated) so the parser knows to split it;
    /// otherwise `Str` (plain), matching Elixir's `"...#{expr}..."` syntax —
    /// distinct from Sylva's `f"..."` prefix convention.
    fn lex_string(&mut self, start: Pos) -> Result<Token, Box<OmniError>> {
        self.advance(); // opening quote
        let mut s = String::new();
        let mut has_splice = false;
        loop {
            let c = self.peek(0);
            if c == '\0' {
                return Err(Box::new(self.err("unterminated string literal", start)));
            }
            if c == '"' {
                self.advance();
                break;
            }
            if c == '\\' {
                self.advance();
                let e = self.advance();
                s.push(self.escape(e, start)?);
                continue;
            }
            if c == '#' && self.peek(1) == '{' {
                has_splice = true;
            }
            s.push(self.advance());
        }
        Ok(self.mk(if has_splice { TokKind::IStr } else { TokKind::Str }, s, start))
    }

    fn escape(&self, e: char, start: Pos) -> Result<char, Box<OmniError>> {
        Ok(match e {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            '\\' => '\\',
            '"' => '"',
            '#' => '#',
            other => return Err(Box::new(self.err(format!("unknown escape '\\{other}'"), start))),
        })
    }

    fn lex_atom(&mut self, start: Pos) -> Token {
        self.advance(); // ':'
        let mut s = String::new();
        while is_ident_continue(self.peek(0)) {
            s.push(self.advance());
        }
        self.mk(TokKind::Atom, s, start)
    }

    fn lex_ident_or_keyword(&mut self, start: Pos) -> Token {
        let mut s = String::new();
        while is_ident_continue(self.peek(0)) {
            s.push(self.advance());
        }
        // Elixir-style trailing `?`/`!` on identifiers (`is_empty?`, `save!`).
        if self.peek(0) == '?' || self.peek(0) == '!' {
            s.push(self.advance());
        }
        match s.as_str() {
            "true" => return self.mk(TokKind::Bool, "true", start),
            "false" => return self.mk(TokKind::Bool, "false", start),
            "nil" => return self.mk(TokKind::Nil, "nil", start),
            _ => {}
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
            '<' => two!('=', "<=".to_string(), "<".to_string()),
            '>' => two!('=', ">=".to_string(), ">".to_string()),
            '+' => "+".to_string(),
            '-' => two!('>', "->".to_string(), "-".to_string()),
            '*' => "*".to_string(),
            '/' => "/".to_string(),
            '%' => "%".to_string(),
            '(' => "(".to_string(),
            ')' => ")".to_string(),
            '[' => "[".to_string(),
            ']' => "]".to_string(),
            '{' => "{".to_string(),
            '}' => "}".to_string(),
            ',' => ",".to_string(),
            ':' => ":".to_string(),
            '.' => {
                if self.peek(0) == '.' {
                    self.advance();
                    "..".to_string()
                } else {
                    ".".to_string()
                }
            }
            ';' => ";".to_string(),
            '|' => two!('>', "|>".to_string(), "|".to_string()),
            '&' => two!('&', "&&".to_string(), "&".to_string()),
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
