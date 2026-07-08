//! Diagnostics — source spans and rustc-style caret errors.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub line: u32, // 1-based
    pub col: u32,  // 1-based
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Pos,
    pub end: Pos,
}

impl Span {
    pub fn point(line: u32, col: u32) -> Self {
        let p = Pos { line, col };
        Span { start: p, end: p }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Lex,
    Parse,
    Runtime,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Phase::Lex => write!(f, "lex"),
            Phase::Parse => write!(f, "parse"),
            Phase::Runtime => write!(f, "runtime"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OmniError {
    pub phase: Phase,
    pub message: String,
    pub span: Option<Span>,
    pub file: String,
    pub help: Option<String>,
}

impl OmniError {
    pub fn new(phase: Phase, message: impl Into<String>, span: Span, file: &str) -> Self {
        OmniError { phase, message: message.into(), span: Some(span), file: file.to_string(), help: None }
    }

    /// Render a caret-underlined snippet against the given source text.
    pub fn render(&self, source: &str) -> String {
        let color = std::env::var("NO_COLOR").is_err();
        let (red, bold, dim, cyan, yellow, reset) = if color {
            ("\x1b[31m", "\x1b[1m", "\x1b[2m", "\x1b[36m", "\x1b[33m", "\x1b[0m")
        } else {
            ("", "", "", "", "", "")
        };
        let mut out = format!("{bold}{red}error[{}]{reset}{bold}: {}{reset}\n", self.phase, self.message);
        if let Some(span) = self.span {
            let line_no = span.start.line as usize;
            let line_text = source.lines().nth(line_no.saturating_sub(1)).unwrap_or("");
            let gutter = line_no.to_string();
            let pad = " ".repeat(gutter.len());
            let caret_pad = " ".repeat(span.start.col.saturating_sub(1) as usize);
            let underline_len = if span.end.line == span.start.line {
                (span.end.col.saturating_sub(span.start.col)).max(1) as usize
            } else {
                (line_text.len() as u32).saturating_sub(span.start.col).max(1) as usize
            };
            out += &format!(" {pad}{dim}{cyan}-->{reset} {}:{}:{}\n", self.file, span.start.line, span.start.col);
            out += &format!(" {pad}{dim}|{reset}\n");
            out += &format!(" {dim}{gutter}{reset} {dim}|{reset} {line_text}\n");
            out += &format!(" {pad}{dim}|{reset} {caret_pad}{red}{}{reset}\n", "^".repeat(underline_len));
        } else {
            out += &format!("  {dim}-->{reset} {}\n", self.file);
        }
        if let Some(h) = &self.help {
            out += &format!(" {dim}={reset} {bold}{yellow}help{reset}: {h}\n");
        }
        out
    }
}
