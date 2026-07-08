//! Sylva lexer — genuinely Python-flavored: **significant whitespace**, not
//! braces. This is the defining structural choice that makes Sylva unique
//! from Titan (brace-delimited, Rust-like) rather than a reskinned copy of
//! it. Blocks are introduced by a `:`-terminated header line and one level
//! of indentation; the lexer emits synthetic `Indent`/`Dedent`/`Newline`
//! tokens (the classic Python tokenizer model) so the parser never has to
//! reason about column counting itself.

use crate::diag::{OmniError, Phase, Pos, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokKind {
    Ident,
    Keyword,
    Int,
    Float,
    Str,
    FStr, // f-string: raw template text, interpolation split at parse time
    Bool,
    None_,
    Op,
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokKind,
    pub value: String,
    pub span: Span,
}

const KEYWORDS: &[&str] = &[
    "def", "class", "if", "elif", "else", "for", "in", "while", "return", "break", "continue",
    "and", "or", "not", "lambda", "import", "from", "as", "try", "except", "finally", "raise",
    "yield", "async", "await", "with", "pass", "global", "nonlocal", "del", "assert", "is",
    "let", "const", "self",
    // Rust-style `use path::{a, b}` import, used verbatim across several
    // omni-integration specs regardless of a language's own native import
    // syntax (Sylva's is `import`/`from..import`) — see `parse_use`.
    "use",
    // Some omni-integration specs are written wholesale in Rust syntax
    // (struct/impl/fn/pub/mut) rather than Sylva's native Python-flavored
    // grammar — see `parse_rust_fn`/`parse_rust_struct`/`parse_rust_impl`.
    "pub", "fn", "struct", "impl", "mut", "mod", "match",
    // Declarative config-block DSL (ML pipeline description) used by the
    // omni-integration specs — see `parse_config_block`.
    "layer", "model", "pipeline", "evolve",
];

pub struct Lexer<'a> {
    chars: Vec<char>,
    i: usize,
    line: u32,
    col: u32,
    file: &'a str,
    /// Indentation stack, in spaces. Starts at `[0]`.
    indents: Vec<usize>,
    /// Bracket nesting depth — newlines inside `(...)`/`[...]`/`{...}` don't
    /// end a logical line (matches Python's implicit line-joining).
    bracket_depth: i32,
    /// True at the start of a logical line, before any non-whitespace token
    /// has been emitted — controls when indentation is measured.
    at_line_start: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str, file: &'a str) -> Self {
        let chars: Vec<char> = src.chars().collect();
        Lexer { chars, i: 0, line: 1, col: 1, file, indents: vec![0], bracket_depth: 0, at_line_start: true }
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
            if self.at_line_start && self.bracket_depth == 0 {
                if !self.measure_indent(&mut out)? {
                    // measure_indent returns false when it hit EOF while
                    // skipping a blank/comment-only line; loop back around.
                    continue;
                }
            }
            self.skip_inline_ws_and_comments();
            let start = self.pos();
            let c = self.peek(0);

            if c == '\0' {
                self.close_out(&mut out, start);
                break;
            }
            if c == '\n' {
                self.advance();
                if self.bracket_depth == 0 {
                    out.push(self.mk(TokKind::Newline, "\n", start));
                    self.at_line_start = true;
                }
                continue;
            }
            if c.is_ascii_digit() {
                out.push(self.lex_number(start)?);
                continue;
            }
            if c == '"' || c == '\'' {
                out.push(self.lex_string(start, false)?);
                continue;
            }
            if (c == 'f' || c == 'F') && (self.peek(1) == '"' || self.peek(1) == '\'') {
                self.advance();
                out.push(self.lex_string(start, true)?);
                continue;
            }
            // Rust-style raw string: `r"..."` / `r#"..."#` / `r##"..."##`
            // (arbitrarily many `#`s, matched on both sides). Must be
            // checked before the general identifier path (`r` alone is a
            // valid identifier) and before '#' is treated as a line-comment
            // marker.
            if c == 'r' && (self.peek(1) == '"' || self.peek(1) == '#') {
                out.push(self.lex_raw_string(start)?);
                continue;
            }
            if is_ident_start(c) {
                out.push(self.lex_ident_or_keyword(start));
                continue;
            }
            if c == '(' || c == '[' || c == '{' {
                self.bracket_depth += 1;
            } else if c == ')' || c == ']' || c == '}' {
                self.bracket_depth -= 1;
            }
            out.push(self.lex_op(start)?);
        }
        Ok(out)
    }

    /// Consumes leading whitespace of a logical line and emits Indent/Dedent
    /// tokens by comparing against the indent stack. Blank lines and
    /// comment-only lines are skipped entirely (no Newline token for them,
    /// matching Python semantics). Returns `Ok(true)` once real content is
    /// reached (or EOF), `Ok(false)` if it consumed a blank line and the
    /// caller should re-enter the loop.
    fn measure_indent(&mut self, out: &mut Vec<Token>) -> Result<bool, Box<OmniError>> {
        let start = self.pos();
        let mut width = 0usize;
        loop {
            match self.peek(0) {
                ' ' => {
                    width += 1;
                    self.advance();
                }
                '\t' => {
                    width += 8 - (width % 8);
                    self.advance();
                }
                _ => break,
            }
        }
        // Blank line or comment-only line: consume the newline and retry.
        if self.peek(0) == '\n' {
            self.advance();
            return Ok(false);
        }
        if self.peek(0) == '#' || (self.peek(0) == '/' && self.peek(1) == '/') {
            while self.peek(0) != '\n' && self.peek(0) != '\0' {
                self.advance();
            }
            if self.peek(0) == '\n' {
                self.advance();
            }
            return Ok(false);
        }
        if self.peek(0) == '\0' {
            self.at_line_start = false;
            return Ok(true);
        }

        self.at_line_start = false;
        let cur = *self.indents.last().unwrap();
        if width > cur {
            self.indents.push(width);
            out.push(self.mk(TokKind::Indent, "", start));
        } else if width < cur {
            while *self.indents.last().unwrap() > width {
                self.indents.pop();
                out.push(self.mk(TokKind::Dedent, "", start));
            }
            if *self.indents.last().unwrap() != width {
                return Err(Box::new(self.err("inconsistent indentation (dedent doesn't match any enclosing indent level)", start)));
            }
        }
        Ok(true)
    }

    fn close_out(&mut self, out: &mut Vec<Token>, start: Pos) {
        if !matches!(out.last().map(|t| t.kind), Some(TokKind::Newline) | None) {
            out.push(self.mk(TokKind::Newline, "\n", start));
        }
        while self.indents.len() > 1 {
            self.indents.pop();
            out.push(self.mk(TokKind::Dedent, "", start));
        }
        out.push(self.mk(TokKind::Eof, "", start));
    }

    fn skip_inline_ws_and_comments(&mut self) {
        loop {
            match self.peek(0) {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\\' if self.peek(1) == '\n' => {
                    // explicit line continuation
                    self.advance();
                    self.advance();
                }
                '#' => {
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
            s.push(self.advance()); // '.'
            while self.peek(0).is_ascii_digit() || self.peek(0) == '_' {
                let c = self.advance();
                if c != '_' {
                    s.push(c);
                }
            }
        }
        if self.peek(0) == 'e' || self.peek(0) == 'E' {
            is_float = true;
            s.push(self.advance());
            if self.peek(0) == '+' || self.peek(0) == '-' {
                s.push(self.advance());
            }
            while self.peek(0).is_ascii_digit() {
                s.push(self.advance());
            }
        }
        // Rust-style numeric-type suffix (`0.0f32`, `128u64`, `12i32`) —
        // consumed and discarded; this bootstrap's numbers are uniformly
        // i64/f64. An `f` suffix also marks the literal as a float even
        // with no decimal point (`0f32`).
        if matches!(self.peek(0), 'f' | 'u' | 'i') {
            let save = (self.i, self.line, self.col);
            let kind = self.advance();
            let mut has_digits = false;
            while self.peek(0).is_ascii_digit() {
                self.advance();
                has_digits = true;
            }
            if has_digits {
                if kind == 'f' {
                    is_float = true;
                }
            } else {
                (self.i, self.line, self.col) = save;
            }
        }
        Ok(self.mk(if is_float { TokKind::Float } else { TokKind::Int }, s, start))
    }

    fn lex_string(&mut self, start: Pos, is_fstring: bool) -> Result<Token, Box<OmniError>> {
        let quote = self.advance(); // opening quote
        // Triple-quoted string: consume two more of the same quote.
        let triple = self.peek(0) == quote && self.peek(1) == quote;
        if triple {
            self.advance();
            self.advance();
        }
        let mut s = String::new();
        loop {
            let c = self.peek(0);
            if c == '\0' {
                return Err(Box::new(self.err("unterminated string literal", start)));
            }
            if triple {
                if c == quote && self.peek(1) == quote && self.peek(2) == quote {
                    self.advance();
                    self.advance();
                    self.advance();
                    break;
                }
            } else if c == quote {
                self.advance();
                break;
            } else if c == '\n' {
                return Err(Box::new(self.err("unterminated string literal (newline before closing quote)", start)));
            }
            if c == '\\' {
                self.advance();
                let e = self.advance();
                s.push(self.escape(e, start)?);
                continue;
            }
            s.push(self.advance());
        }
        Ok(self.mk(if is_fstring { TokKind::FStr } else { TokKind::Str }, s, start))
    }

    /// `r"..."` / `r#"..."#` / `r##"..."##` — no escape processing, and
    /// (unlike `lex_string`) embedded newlines are allowed since these are
    /// used for multi-line format templates in the omni-integration specs.
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
                // Closing delimiter only if followed by exactly `hashes` '#'s.
                let mut k = 1;
                while k <= hashes && self.peek(k) == '#' {
                    k += 1;
                }
                if k == hashes + 1 {
                    self.advance(); // '"'
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

    fn escape(&self, e: char, start: Pos) -> Result<char, Box<OmniError>> {
        Ok(match e {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            '0' => '\0',
            '\\' => '\\',
            '\'' => '\'',
            '"' => '"',
            '{' => '{',
            '}' => '}',
            other => return Err(Box::new(self.err(format!("unknown escape '\\{other}'"), start))),
        })
    }

    fn lex_ident_or_keyword(&mut self, start: Pos) -> Token {
        let mut s = String::new();
        while is_ident_continue(self.peek(0)) {
            s.push(self.advance());
        }
        if s == "True" {
            return self.mk(TokKind::Bool, "true", start);
        }
        if s == "False" {
            return self.mk(TokKind::Bool, "false", start);
        }
        if s == "None" {
            return self.mk(TokKind::None_, "none", start);
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
            '*' => {
                if self.peek(0) == '*' {
                    self.advance();
                    "**".to_string()
                } else if self.peek(0) == '=' {
                    self.advance();
                    "*=".to_string()
                } else {
                    "*".to_string()
                }
            }
            '/' => two!('=', "/=".to_string(), "/".to_string()),
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
            '@' => "@".to_string(),
            '|' => two!('|', "||".to_string(), "|".to_string()),
            '&' => two!('&', "&&".to_string(), "&".to_string()),
            ';' => ";".to_string(),
            '?' => "?".to_string(),
            '^' => "^".to_string(),
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
