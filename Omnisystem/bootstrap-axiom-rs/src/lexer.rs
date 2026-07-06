//! Axiom lexer — an ordinary token stream. Axiom's real uniqueness is that
//! `theorem`/`invariant` statements are **actually verified** (see
//! `interp.rs`), not lexical shape. `=>` (material implication) is the one
//! genuinely domain-distinctive operator — no other Omni-Language needs
//! propositional-logic implication.

use crate::diag::{OmniError, Phase, Pos, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokKind {
    Ident,
    Keyword,
    Int,
    Str,
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

const KEYWORDS: &[&str] = &[
    "axiom", "theorem", "invariant", "forall", "in", "over", "states", "let", "and", "or", "not", "true", "false",
    // Structured-theorem-block keywords (preconditions/postconditions/invariants/
    // assertions blocks, plus statement/quantifier forms used inside them).
    "preconditions", "postconditions", "invariants", "assertions", "if", "else", "assert", "exists", "where",
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
            if c.is_ascii_digit() || (c == '-' && self.peek(1).is_ascii_digit() && self.prev_allows_unary()) {
                out.push(self.lex_number(start)?);
                continue;
            }
            if c == '"' {
                out.push(self.lex_string(start)?);
                continue;
            }
            if c == '\'' {
                out.push(self.lex_char(start)?);
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

    /// Heuristic kept intentionally simple: this is only consulted right
    /// after whitespace-skipping at a fresh token boundary, so a `-`
    /// immediately followed by a digit here is extremely likely meant as a
    /// literal's sign (e.g. `-5..5`) rather than binary subtraction, which
    /// would require a preceding operand token already consumed. Since we
    /// don't track that context precisely, negative-int lexing is opt-in
    /// only inside range-bound parsing (see `parser.rs::parse_range`),
    /// which calls a dedicated signed-int lexer path instead of relying on
    /// this; this general path never actually fires for `-` today. Kept as
    /// a documented no-op rather than removed, since a future extension
    /// (e.g. unary-minus literals outside ranges) would want it.
    fn prev_allows_unary(&self) -> bool {
        false
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
        let mut s = String::new();
        if self.peek(0) == '-' {
            s.push(self.advance());
        }
        // Digits may contain `_` separators (e.g. `1_000_000`) — stripped here
        // since Axiom's Int value model has no concept of them; only affects
        // literal spelling, not magnitude.
        while self.peek(0).is_ascii_digit() || self.peek(0) == '_' {
            let c = self.advance();
            if c != '_' {
                s.push(c);
            }
        }
        // Rust-style integer-type suffixes (`1_000_000_000u64`, `64u32`) are
        // consumed and discarded — this bootstrap's Int is uniformly i64.
        if matches!(self.peek(0), 'u' | 'i') {
            let save = self.i;
            let (save_line, save_col) = (self.line, self.col);
            self.advance();
            let mut suffix_digits = false;
            while self.peek(0).is_ascii_digit() {
                self.advance();
                suffix_digits = true;
            }
            if !suffix_digits {
                // Not actually a suffix (e.g. bare trailing ident) — rewind.
                self.i = save;
                self.line = save_line;
                self.col = save_col;
            }
        }
        Ok(self.mk(TokKind::Int, s, start))
    }

    fn lex_string(&mut self, start: Pos) -> Result<Token, Box<OmniError>> {
        self.advance(); // opening '"'
        let mut s = String::new();
        loop {
            match self.peek(0) {
                '"' => {
                    self.advance();
                    break;
                }
                '\0' | '\n' => return Err(Box::new(self.err("unterminated string literal", start))),
                '\\' => {
                    self.advance();
                    let esc = self.advance();
                    s.push(match esc {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '"' => '"',
                        '\\' => '\\',
                        other => other,
                    });
                }
                _ => s.push(self.advance()),
            }
        }
        Ok(self.mk(TokKind::Str, s, start))
    }

    /// `'c'` / `'\n'` — modeled as a one-character `Str` (this bootstrap has
    /// no separate char value type; parse-level dialect extension only).
    fn lex_char(&mut self, start: Pos) -> Result<Token, Box<OmniError>> {
        self.advance(); // opening '\''
        let c = if self.peek(0) == '\\' {
            self.advance();
            match self.advance() {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\'' => '\'',
                '\\' => '\\',
                other => other,
            }
        } else {
            self.advance()
        };
        if self.peek(0) != '\'' {
            return Err(Box::new(self.err("unterminated character literal", start)));
        }
        self.advance();
        Ok(self.mk(TokKind::Str, c.to_string(), start))
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
            '=' => {
                if self.peek(0) == '=' {
                    self.advance();
                    "==".to_string()
                } else if self.peek(0) == '>' {
                    self.advance();
                    "=>".to_string()
                } else {
                    // Bare '=' is valid only as `let name = expr` assignment
                    // (see `parser.rs::parse_theorem_stmt`) in the
                    // omni-integration structured-theorem dialect; this
                    // bootstrap's original ground/quantified propositions
                    // never used bare '=' (equality is always '==').
                    "=".to_string()
                }
            }
            '!' => two!('=', "!=".to_string(), "!".to_string()),
            '<' => {
                if self.peek(0) == '=' && self.peek(1) == '>' {
                    self.advance();
                    self.advance();
                    "<=>".to_string()
                } else {
                    two!('=', "<=".to_string(), "<".to_string())
                }
            }
            '>' => two!('=', ">=".to_string(), ">".to_string()),
            '+' => "+".to_string(),
            '-' => "-".to_string(),
            '*' => "*".to_string(),
            '/' => "/".to_string(),
            '%' => "%".to_string(),
            '(' => "(".to_string(),
            ')' => ")".to_string(),
            '{' => "{".to_string(),
            '}' => "}".to_string(),
            '[' => "[".to_string(),
            ']' => "]".to_string(),
            '|' => two!('|', "||".to_string(), "|".to_string()),
            '&' => two!('&', "&&".to_string(), "&".to_string()),
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
