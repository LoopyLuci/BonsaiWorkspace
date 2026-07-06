//! Vera lexer — an ordinary token stream (idents/keywords/literals/ops).
//! Vera's real distinguishing feature isn't lexer trickery — it's that the
//! **parser** enters a dedicated markup-parsing mode inside `render { }`
//! blocks, where `<Tag>`/`</Tag>`/`{expr}` read as tag trees instead of
//! normal expressions. That structural separation (markup context vs.
//! expression context, chosen by the parser, not by lookahead heuristics)
//! is what makes embedded reactive markup possible without the classic
//! JSX `<` operator-vs-tag ambiguity — see `parser.rs`.

use crate::diag::{OmniError, Phase, Pos, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokKind {
    Ident,
    Keyword,
    Int,
    Float,
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
    "component", "state", "computed", "fn", "let", "if", "else", "for", "in", "return",
    "true", "false", "and", "or", "not", "match",
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
            // Rust-style raw string `r"..."` / `r#"..."#` — checked before
            // the general identifier path ('r' alone is a valid identifier)
            // and before '#' would otherwise be an unrecognized character.
            if c == 'r' && (self.peek(1) == '"' || self.peek(1) == '#') {
                out.push(self.lex_raw_string(start)?);
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
        self.advance(); // opening quote
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
            if c == '\\' {
                self.advance();
                let e = self.advance();
                s.push(match e {
                    'n' => '\n',
                    't' => '\t',
                    '\\' => '\\',
                    '"' => '"',
                    other => return Err(Box::new(self.err(format!("unknown escape '\\{other}'"), start))),
                });
                continue;
            }
            s.push(self.advance());
        }
        Ok(self.mk(TokKind::Str, s, start))
    }

    /// `r"..."` / `r#"..."#` / `r##"..."##` — no escape processing, and
    /// embedded newlines are allowed (used for multi-line format templates).
    fn lex_raw_string(&mut self, start: Pos) -> Result<Token, Box<OmniError>> {
        self.advance(); // 'r'
        let mut hashes = 0usize;
        while self.peek(0) == '#' {
            self.advance();
            hashes += 1;
        }
        if self.peek(0) != '"' {
            return Err(Box::new(self.err("expected '\"' to start a raw string", start)));
        }
        self.advance(); // opening '"'
        let mut s = String::new();
        loop {
            if self.peek(0) == '\0' {
                return Err(Box::new(self.err("unterminated raw string literal", start)));
            }
            if self.peek(0) == '"' {
                let mut k = 1;
                while k <= hashes && self.peek(k) == '#' {
                    k += 1;
                }
                if k == hashes + 1 {
                    self.advance();
                    for _ in 0..hashes {
                        self.advance();
                    }
                    break;
                }
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
                    "=".to_string()
                }
            }
            '!' => two!('=', "!=".to_string(), "!".to_string()),
            '<' => two!('=', "<=".to_string(), "<".to_string()),
            '>' => two!('=', ">=".to_string(), ">".to_string()),
            '+' => two!('=', "+=".to_string(), "+".to_string()),
            '-' => two!('>', "->".to_string(), "-".to_string()),
            '*' => "*".to_string(),
            '/' => two!('>', "/>".to_string(), "/".to_string()),
            '%' => "%".to_string(),
            '(' => "(".to_string(),
            ')' => ")".to_string(),
            '[' => "[".to_string(),
            ']' => "]".to_string(),
            '{' => "{".to_string(),
            '}' => "}".to_string(),
            ',' => ",".to_string(),
            ':' => ":".to_string(),
            '.' => two!('.', "..".to_string(), ".".to_string()),
            ';' => ";".to_string(),
            '|' => two!('|', "||".to_string(), "|".to_string()),
            '&' => two!('&', "&&".to_string(), "&".to_string()),
            other => return Err(Box::new(self.err(format!("unexpected character '{other}'"), start))),
        };
        Ok(self.mk(TokKind::Op, s, start))
    }
}

/// Any non-ASCII character (`≤`, `→`, emoji, ...) is treated as
/// identifier-like too, not just Unicode *letters* — Vera's markup mode
/// (see the module doc comment) reconstructs bare inline text nodes
/// (`<span>parallel ≤ {n}</span>`) from the ordinary token stream (see
/// `parser.rs::parse_node`'s bare-text case), and a symbol like `≤` used as
/// literal text content would otherwise be an unrecognized-character lex
/// error. Vera has no actual use for these as *operators*, so there's no
/// ambiguity to worry about.
fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || !c.is_ascii()
}
fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || !c.is_ascii()
}
