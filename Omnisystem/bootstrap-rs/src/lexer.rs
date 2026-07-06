//! Titan lexer — source text to a token stream with precise spans.
//!
//! Handles line and nested block comments, decimal/hex/bin/oct integers and
//! floats with `_` separators and type suffixes, string/char literals with
//! escapes, lifetimes, identifiers/keywords, and greedy longest-match
//! operators.

use crate::diag::{OResult, OmniError, Phase, Pos, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokKind {
    Int,
    Float,
    Str,
    Char,
    Bool,
    Ident,
    Keyword,
    Lifetime,
    Op,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokKind,
    pub value: String,
    pub span: Span,
}

pub const KEYWORDS: &[&str] = &[
    "use", "pub", "mod", "struct", "enum", "impl", "trait", "fn", "let", "mut", "const", "static",
    "if", "else", "match", "while", "for", "in", "loop", "break", "continue", "return", "self",
    "Self", "true", "false", "as", "where", "ref", "move", "dyn", "async", "await", "type",
    "unsafe", "extern",
];

/// Multi-char operators first so matching is greedy.
pub const OPERATORS: &[&str] = &[
    "..=", "...", "<<=", ">>=", "->", "=>", "::", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=",
    "*=", "/=", "%=", "&=", "|=", "^=", "<<", ">>", "..", "+", "-", "*", "/", "%", "=", "<", ">",
    "!", "&", "|", "^", "~", "?", ".", ",", ";", ":", "(", ")", "{", "}", "[", "]", "@", "#",
];

pub struct Lexer<'a> {
    src: Vec<char>,
    file: &'a str,
    i: usize,
    line: u32,
    col: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str, file: &'a str) -> Self {
        Lexer { src: src.chars().collect(), file, i: 0, line: 1, col: 1 }
    }

    fn pos(&self) -> Pos {
        Pos { line: self.line, col: self.col }
    }

    fn err(&self, msg: impl Into<String>, start: Pos) -> OmniError {
        OmniError::new(Phase::Lex, msg, Span { start, end: self.pos() }, self.file)
    }

    fn peek(&self, k: usize) -> char {
        self.src.get(self.i + k).copied().unwrap_or('\0')
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

    fn starts(&self, s: &str) -> bool {
        let rest = &self.src[self.i.min(self.src.len())..];
        s.chars().enumerate().all(|(k, c)| rest.get(k) == Some(&c))
    }

    pub fn tokenize(mut self) -> OResult<Vec<Token>> {
        let mut toks = Vec::new();
        loop {
            self.skip_trivia()?;
            let start = self.pos();
            if self.i >= self.src.len() {
                toks.push(Token { kind: TokKind::Eof, value: String::new(), span: Span { start, end: start } });
                return Ok(toks);
            }
            let c = self.peek(0);
            let tok = if c == 'r' && (self.peek(1) == '"' || (self.peek(1) == '#' && (self.peek(2) == '"' || self.peek(2) == '#'))) {
                self.lex_raw_string(start)?
            } else if c == 'b' && self.peek(1) == '"' {
                self.advance(); // consume 'b' — byte strings lex as ordinary strings
                self.lex_string(start)?
            } else if c == 'b' && self.peek(1) == '\'' {
                // byte char b'E' — lexes as an integer (the byte value)
                self.advance();
                let t = self.lex_char_or_lifetime(start)?;
                let byte = t.value.chars().next().map(|ch| ch as u32).unwrap_or(0);
                Token { kind: TokKind::Int, value: byte.to_string(), span: t.span }
            } else if c == '_' || c.is_ascii_alphabetic() {
                self.lex_ident(start)
            } else if c.is_ascii_digit() {
                self.lex_number(start)?
            } else if c == '"' {
                self.lex_string(start)?
            } else if c == '\'' {
                self.lex_char_or_lifetime(start)?
            } else {
                self.lex_operator(start)?
            };
            toks.push(tok);
        }
    }

    fn skip_trivia(&mut self) -> OResult<()> {
        loop {
            let c = self.peek(0);
            if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
                self.advance();
            } else if c == '/' && self.peek(1) == '/' {
                while self.i < self.src.len() && self.peek(0) != '\n' {
                    self.advance();
                }
            } else if c == '/' && self.peek(1) == '*' {
                let start = self.pos();
                self.advance();
                self.advance();
                let mut depth = 1u32;
                while depth > 0 {
                    if self.i >= self.src.len() {
                        return Err(self.err("unterminated block comment", start));
                    }
                    if self.starts("/*") {
                        self.advance();
                        self.advance();
                        depth += 1;
                    } else if self.starts("*/") {
                        self.advance();
                        self.advance();
                        depth -= 1;
                    } else {
                        self.advance();
                    }
                }
            } else {
                return Ok(());
            }
        }
    }

    fn mk(&self, kind: TokKind, value: String, start: Pos) -> Token {
        Token { kind, value, span: Span { start, end: self.pos() } }
    }

    fn lex_ident(&mut self, start: Pos) -> Token {
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

    fn lex_number(&mut self, start: Pos) -> OResult<Token> {
        // hex / bin / oct
        if self.peek(0) == '0' && matches!(self.peek(1), 'x' | 'b' | 'o') {
            self.advance();
            let base = self.advance();
            let mut digits = String::new();
            while self.peek(0).is_ascii_alphanumeric() || self.peek(0) == '_' {
                let c = self.advance();
                if c != '_' {
                    digits.push(c);
                }
            }
            // strip a numeric type suffix like u8/i32 from the digit run
            let radix = match base { 'x' => 16, 'o' => 8, _ => 2 };
            let cut = digits
                .char_indices()
                .find(|(_, ch)| !ch.is_digit(radix))
                .map(|(idx, _)| idx)
                .unwrap_or(digits.len());
            let (num, _suffix) = digits.split_at(cut);
            let val = i64::from_str_radix(num, radix)
                .map_err(|_| self.err(format!("invalid numeric literal `{digits}`"), start))?;
            return Ok(self.mk(TokKind::Int, val.to_string(), start));
        }
        let mut raw = String::new();
        let mut is_float = false;
        while self.peek(0).is_ascii_digit() || self.peek(0) == '_' {
            let c = self.advance();
            if c != '_' {
                raw.push(c);
            }
        }
        if self.peek(0) == '.' && self.peek(1).is_ascii_digit() {
            is_float = true;
            raw.push(self.advance());
            while self.peek(0).is_ascii_digit() || self.peek(0) == '_' {
                let c = self.advance();
                if c != '_' {
                    raw.push(c);
                }
            }
        }
        if matches!(self.peek(0), 'e' | 'E') && (self.peek(1).is_ascii_digit() || (matches!(self.peek(1), '+' | '-') && self.peek(2).is_ascii_digit())) {
            is_float = true;
            raw.push(self.advance());
            if matches!(self.peek(0), '+' | '-') {
                raw.push(self.advance());
            }
            while self.peek(0).is_ascii_digit() {
                raw.push(self.advance());
            }
        }
        // type suffix (i32, u8, f64, usize, ...)
        let mut suffix = String::new();
        while is_ident_continue(self.peek(0)) {
            suffix.push(self.advance());
        }
        if suffix.starts_with('f') {
            is_float = true;
        }
        Ok(self.mk(if is_float { TokKind::Float } else { TokKind::Int }, raw, start))
    }

    /// r"..." / r#"..."# / r##"..."## — no escapes, closes on `"` + same # count.
    fn lex_raw_string(&mut self, start: Pos) -> OResult<Token> {
        self.advance(); // r
        let mut hashes = 0usize;
        while self.peek(0) == '#' {
            self.advance();
            hashes += 1;
        }
        if self.peek(0) != '"' {
            return Err(self.err("expected '\"' to open raw string", start));
        }
        self.advance();
        let closer: String = format!("\"{}", "#".repeat(hashes));
        let mut out = String::new();
        loop {
            if self.i >= self.src.len() {
                return Err(self.err("unterminated raw string literal", start));
            }
            if self.starts(&closer) {
                for _ in 0..closer.len() {
                    self.advance();
                }
                break;
            }
            out.push(self.advance());
        }
        Ok(self.mk(TokKind::Str, out, start))
    }

    fn lex_string(&mut self, start: Pos) -> OResult<Token> {
        self.advance(); // opening "
        let mut out = String::new();
        loop {
            if self.i >= self.src.len() {
                return Err(self.err("unterminated string literal", start));
            }
            let c = self.advance();
            if c == '"' {
                break;
            }
            if c == '\\' {
                let e = self.advance();
                out.push(self.escape(e, start)?);
            } else {
                out.push(c);
            }
        }
        Ok(self.mk(TokKind::Str, out, start))
    }

    fn lex_char_or_lifetime(&mut self, start: Pos) -> OResult<Token> {
        self.advance(); // opening '
        // lifetime: '<ident> not closed by another quote
        if is_ident_start(self.peek(0)) {
            let save = (self.i, self.line, self.col);
            let mut name = String::new();
            while is_ident_continue(self.peek(0)) {
                name.push(self.advance());
            }
            if self.peek(0) != '\'' {
                return Ok(self.mk(TokKind::Lifetime, name, start));
            }
            // it was a char literal like 'a' — rewind
            (self.i, self.line, self.col) = save;
        }
        let c = self.advance();
        let ch = if c == '\\' {
            let e = self.advance();
            self.escape(e, start)?
        } else {
            c
        };
        if self.peek(0) != '\'' {
            return Err(self
                .err("unterminated char literal", start)
                .with_help("char literals hold exactly one character, e.g. 'a'"));
        }
        self.advance();
        Ok(self.mk(TokKind::Char, ch.to_string(), start))
    }

    fn escape(&self, e: char, start: Pos) -> OResult<char> {
        Ok(match e {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            '0' => '\0',
            '\\' => '\\',
            '"' => '"',
            '\'' => '\'',
            other => return Err(self.err(format!("unknown escape sequence \\{}", other), start)),
        })
    }

    fn lex_operator(&mut self, start: Pos) -> OResult<Token> {
        for op in OPERATORS {
            if self.starts(op) {
                for _ in 0..op.len() {
                    self.advance();
                }
                return Ok(self.mk(TokKind::Op, (*op).to_string(), start));
            }
        }
        let bad = self.advance();
        Err(self.err(format!("unexpected character '{bad}'"), start))
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic()
}
fn is_ident_continue(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}
