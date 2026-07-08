//! Sylva parser — recursive-descent over the indentation-aware token stream
//! from `lexer.rs`. Blocks are `:` + `Indent ... Dedent` (real Python shape),
//! not `{ }` (Titan's shape) — this is the parser-level expression of the
//! same uniqueness decision made in the lexer.

use crate::ast::*;
use crate::diag::{OmniError, Phase, Pos, Span};
use crate::lexer::{TokKind, Token};

pub type OResult<T> = Result<T, Box<OmniError>>;

pub struct Parser<'a> {
    toks: Vec<Token>,
    p: usize,
    file: &'a str,
    /// Disabled while parsing an `if`/`while` condition in the Rust-syntax
    /// dialect (see `parse_rust_if`) so `if config {` isn't misread as a
    /// struct-literal `config { ... }` instead of the `if`'s block start —
    /// the same ambiguity Rust itself resolves by banning struct literals
    /// there.
    allow_struct_lit: bool,
    /// Disabled while parsing a Rust-syntax-dialect expression (see
    /// `parse_rust_expr_with_ref`) — see `parse_ternary`'s doc comment.
    allow_python_ternary: bool,
}

impl<'a> Parser<'a> {
    pub fn new(toks: Vec<Token>, file: &'a str) -> Self {
        Parser { toks, p: 0, file, allow_struct_lit: true, allow_python_ternary: true }
    }

    fn cur(&self) -> &Token {
        &self.toks[self.p.min(self.toks.len() - 1)]
    }
    fn at_eof(&self) -> bool {
        self.cur().kind == TokKind::Eof
    }
    fn advance(&mut self) -> Token {
        let t = self.toks[self.p.min(self.toks.len() - 1)].clone();
        if self.p < self.toks.len() - 1 {
            self.p += 1;
        }
        t
    }
    fn is_op(&self, v: &str) -> bool {
        self.cur().kind == TokKind::Op && self.cur().value == v
    }
    /// The `k`-th token's value ahead of `cur()` if it's an `Op`, else `None`.
    fn peek_op_at(&self, k: usize) -> Option<&str> {
        let i = (self.p + k).min(self.toks.len() - 1);
        (self.toks[i].kind == TokKind::Op).then(|| self.toks[i].value.as_str())
    }
    fn is_kw(&self, v: &str) -> bool {
        self.cur().kind == TokKind::Keyword && self.cur().value == v
    }
    fn eat_op(&mut self, v: &str) -> bool {
        if self.is_op(v) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn eat_kw(&mut self, v: &str) -> bool {
        if self.is_kw(v) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn expect_op(&mut self, v: &str) -> OResult<Token> {
        if self.is_op(v) {
            Ok(self.advance())
        } else {
            Err(self.err(format!("expected '{v}' but found '{}'", self.cur_desc())))
        }
    }
    fn expect_kw(&mut self, v: &str) -> OResult<Token> {
        if self.is_kw(v) {
            Ok(self.advance())
        } else {
            Err(self.err(format!("expected keyword '{v}' but found '{}'", self.cur_desc())))
        }
    }
    fn expect_ident(&mut self) -> OResult<String> {
        if self.cur().kind == TokKind::Ident {
            Ok(self.advance().value)
        } else {
            Err(self.err(format!("expected identifier but found '{}'", self.cur_desc())))
        }
    }
    fn cur_desc(&self) -> String {
        match self.cur().kind {
            TokKind::Eof => "end of file".to_string(),
            TokKind::Newline => "newline".to_string(),
            TokKind::Indent => "indent".to_string(),
            TokKind::Dedent => "dedent".to_string(),
            _ => self.cur().value.clone(),
        }
    }
    fn start(&self) -> Pos {
        self.cur().span.start
    }
    fn sp(&self, start: Pos) -> Span {
        Span { start, end: self.toks[self.p.saturating_sub(1).min(self.toks.len() - 1)].span.end }
    }
    fn err(&self, msg: impl Into<String>) -> Box<OmniError> {
        Box::new(OmniError::new(Phase::Parse, msg, self.cur().span, self.file))
    }

    fn skip_newlines(&mut self) {
        while self.cur().kind == TokKind::Newline {
            self.advance();
        }
    }

    pub fn parse_module(&mut self) -> OResult<Module> {
        self.skip_newlines();
        let mut body = Vec::new();
        while !self.at_eof() {
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        Ok(Module { body })
    }

    /// `:` NEWLINE Indent stmt* Dedent — the one block-introduction shape
    /// every compound statement (`def`, `class`, `if`, `for`, `while`,
    /// `try`) shares.
    fn parse_block(&mut self) -> OResult<Vec<Stmt>> {
        self.expect_op(":")?;
        // Single-line body: `if x: return 1` (no indent block follows).
        if self.cur().kind != TokKind::Newline {
            let s = self.parse_simple_stmt_line()?;
            return Ok(s);
        }
        self.skip_newlines();
        if self.cur().kind != TokKind::Indent {
            return Err(self.err("expected an indented block"));
        }
        self.advance(); // Indent
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if self.cur().kind == TokKind::Dedent || self.at_eof() {
                break;
            }
            stmts.push(self.parse_stmt()?);
        }
        if self.cur().kind == TokKind::Dedent {
            self.advance();
        }
        Ok(stmts)
    }

    /// One or more `;`-separated simple statements on the same line as a
    /// one-line block header (`if x: a = 1; b = 2`).
    fn parse_simple_stmt_line(&mut self) -> OResult<Vec<Stmt>> {
        let mut out = vec![self.parse_stmt_no_newline()?];
        while self.eat_op(";") {
            if self.cur().kind == TokKind::Newline || self.at_eof() {
                break;
            }
            out.push(self.parse_stmt_no_newline()?);
        }
        Ok(out)
    }

    fn parse_stmt(&mut self) -> OResult<Stmt> {
        let s = self.parse_stmt_inner()?;
        Ok(s)
    }

    fn parse_stmt_no_newline(&mut self) -> OResult<Stmt> {
        self.parse_stmt_inner()
    }

    fn parse_stmt_inner(&mut self) -> OResult<Stmt> {
        let start = self.start();

        if self.is_op("@") {
            return self.parse_decorated();
        }
        if self.is_kw("def") || (self.is_kw("async") && self.toks.get(self.p + 1).is_some_and(|t| t.value == "def")) {
            return Ok(Stmt::FnDef(self.parse_fndef(vec![])?));
        }
        if self.is_kw("class") {
            return Ok(Stmt::ClassDef(self.parse_classdef()?));
        }
        if self.is_kw("if") {
            return self.parse_if();
        }
        if self.is_kw("while") {
            return self.parse_while();
        }
        if self.is_kw("for") {
            return self.parse_for();
        }
        if self.is_kw("try") {
            return self.parse_try();
        }
        if self.is_kw("return") {
            self.advance();
            let value = if self.at_stmt_end() { None } else { Some(self.parse_expr()?) };
            self.end_simple_stmt()?;
            return Ok(Stmt::Return { value, span: self.sp(start) });
        }
        if self.is_kw("break") {
            self.advance();
            self.end_simple_stmt()?;
            return Ok(Stmt::Break(self.sp(start)));
        }
        if self.is_kw("continue") {
            self.advance();
            self.end_simple_stmt()?;
            return Ok(Stmt::Continue(self.sp(start)));
        }
        if self.is_kw("pass") {
            self.advance();
            self.end_simple_stmt()?;
            return Ok(Stmt::Pass);
        }
        if self.is_kw("raise") {
            self.advance();
            let exc = if self.at_stmt_end() { None } else { Some(self.parse_expr()?) };
            self.end_simple_stmt()?;
            return Ok(Stmt::Raise { exc, span: self.sp(start) });
        }
        if self.is_kw("assert") {
            self.advance();
            let cond = self.parse_expr()?;
            let msg = if self.eat_op(",") { Some(self.parse_expr()?) } else { None };
            self.end_simple_stmt()?;
            return Ok(Stmt::Assert { cond, msg, span: self.sp(start) });
        }
        if self.is_kw("global") || self.is_kw("nonlocal") {
            self.advance();
            let mut names = vec![self.expect_ident()?];
            while self.eat_op(",") {
                names.push(self.expect_ident()?);
            }
            self.end_simple_stmt()?;
            return Ok(Stmt::Global { names });
        }
        if self.is_kw("del") {
            self.advance();
            let target = self.parse_expr()?;
            self.end_simple_stmt()?;
            return Ok(Stmt::Del { target, span: self.sp(start) });
        }
        if self.is_kw("import") || self.is_kw("from") {
            return self.parse_import();
        }
        // `pub` may prefix `struct`/`fn`/`impl`/`mod`/`use` — peek past it to
        // route to the right parser (each parser itself consumes the
        // optional `pub`).
        let after_pub = if self.is_kw("pub") { self.toks.get(self.p + 1).map(|t| t.value.as_str()) } else { None };
        if self.is_kw("use") || after_pub == Some("use") {
            self.eat_kw("pub");
            return self.parse_use();
        }
        if self.is_kw("layer") || self.is_kw("model") || self.is_kw("pipeline") || self.is_kw("evolve") {
            return self.parse_config_block();
        }
        if self.is_kw("struct") || after_pub == Some("struct") {
            return Ok(Stmt::ClassDef(self.parse_rust_struct()?));
        }
        if self.is_kw("impl") {
            return Ok(Stmt::ClassDef(self.parse_rust_impl()?));
        }
        if self.is_kw("fn") || after_pub == Some("fn") {
            return Ok(Stmt::FnDef(self.parse_rust_fn()?));
        }
        if self.is_kw("mod") || after_pub == Some("mod") {
            return self.parse_rust_mod();
        }
        if self.is_kw("let") || self.is_kw("const") {
            self.advance();
            let name = self.expect_ident()?;
            let value = if self.eat_op("=") { Some(self.parse_expr()?) } else { None };
            self.end_simple_stmt()?;
            return Ok(Stmt::Let { name, value, span: self.sp(start) });
        }

        // Expression statement or assignment.
        let expr = self.parse_expr()?;
        if let Some(op) = self.peek_aug_assign() {
            self.advance();
            let rhs = self.parse_expr()?;
            let value = if op == "=" {
                rhs
            } else {
                let binop = op.trim_end_matches('=').to_string();
                Expr::BinOp { op: binop, left: Box::new(expr.clone()), right: Box::new(rhs), span: self.sp(start) }
            };
            self.end_simple_stmt()?;
            return Ok(Stmt::Assign { target: expr, value, span: self.sp(start) });
        }
        self.end_simple_stmt()?;
        Ok(Stmt::Expr(expr))
    }

    fn peek_aug_assign(&self) -> Option<String> {
        if self.cur().kind != TokKind::Op {
            return None;
        }
        match self.cur().value.as_str() {
            "=" | "+=" | "-=" | "*=" | "/=" => Some(self.cur().value.clone()),
            _ => None,
        }
    }

    fn at_stmt_end(&self) -> bool {
        matches!(self.cur().kind, TokKind::Newline | TokKind::Eof) || self.is_op(";")
    }
    fn end_simple_stmt(&mut self) -> OResult<()> {
        if self.is_op(";") || self.cur().kind == TokKind::Newline || self.at_eof() {
            return Ok(());
        }
        // Also fine if the current token starts on a later source line than
        // the previous one: happens when a Python-style simple statement
        // (`use`, `let`, ..) appears inside a brace-delimited Rust-dialect
        // block (`mod { .. }`), where this lexer emits no `Newline` token at
        // all once `bracket_depth > 0` (see `lexer.rs`) — same class of
        // "line-is-the-real-separator-but-suppressed" issue as
        // `skip_config_value`'s doc comment describes.
        if self.p > 0 && self.toks[self.p - 1].span.end.line < self.cur().span.start.line {
            return Ok(());
        }
        Err(self.err(format!("expected end of statement but found '{}'", self.cur_desc())))
    }

    fn parse_decorated(&mut self) -> OResult<Stmt> {
        let mut decorators = Vec::new();
        while self.eat_op("@") {
            decorators.push(self.parse_expr()?);
            self.skip_newlines();
        }
        Ok(Stmt::FnDef(self.parse_fndef(decorators)?))
    }

    fn parse_fndef(&mut self, decorators: Vec<Expr>) -> OResult<FnDef> {
        let start = self.start();
        let is_async = self.eat_kw("async");
        self.expect_kw("def")?;
        let name = self.expect_ident()?;
        self.expect_op("(")?;
        let params = self.parse_params(")", true)?;
        self.expect_op(")")?;
        if self.eat_op("->") {
            self.parse_type_hint_ignored()?;
        }
        let body = self.parse_block()?;
        Ok(FnDef { name, params, body, decorators, is_async, span: self.sp(start) })
    }

    /// Type hints are accepted but not enforced anywhere (Sylva is
    /// dynamically typed) — parsed and discarded so `def f(x: int) -> int:`
    /// style annotations don't error, without pretending to check them.
    fn parse_type_hint_ignored(&mut self) -> OResult<()> {
        // A type hint is any identifier path, optionally subscripted
        // (`List[int]`) — consume tokens until we hit `:`, `)`, `,`, or `=`.
        let mut depth = 0;
        loop {
            match self.cur().kind {
                TokKind::Op if self.cur().value == "[" => {
                    depth += 1;
                    self.advance();
                }
                TokKind::Op if self.cur().value == "]" => {
                    depth -= 1;
                    self.advance();
                }
                TokKind::Op if depth == 0 && matches!(self.cur().value.as_str(), ":" | ")" | "," | "=") => break,
                TokKind::Newline | TokKind::Eof if depth == 0 => break,
                _ => {
                    self.advance();
                }
            }
        }
        Ok(())
    }

    /// Like `expect_ident`, but also accepts the `self` keyword as a
    /// parameter name (`def method(self, ...)`  — `self` is lexed as a
    /// keyword so it can also be used as an expression atom).
    fn expect_param_name(&mut self) -> OResult<String> {
        if self.is_kw("self") {
            self.advance();
            return Ok("self".to_string());
        }
        self.expect_ident()
    }

    /// `(a, b)` / `(a, (b, c))`, arbitrarily nested — flattened into one
    /// comma-joined synthetic name (see the closure-param call site's doc
    /// comment for why a flat name is fine here). The leading `(` is
    /// consumed by this call (not the caller).
    fn parse_tuple_pattern_name(&mut self) -> OResult<String> {
        self.expect_op("(")?;
        let mut names = Vec::new();
        while !self.is_op(")") && !self.at_eof() {
            let n = if self.is_op("(") { self.parse_tuple_pattern_name()? } else { self.expect_param_name()? };
            names.push(n);
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        Ok(names.join(","))
    }

    /// Like `expect_ident`, but also accepts this dialect's new DSL
    /// keywords (`layer`/`model`/`pipeline`/etc.) as a field/attribute name
    /// — they're only reserved as block-introducers at statement position,
    /// but the omni-integration specs also use them as ordinary struct/
    /// config-block field names (e.g. `model: judge_model`).
    fn expect_field_name(&mut self) -> OResult<String> {
        const DIALECT_KEYWORDS: &[&str] = &["layer", "model", "pipeline", "evolve", "use", "pub", "fn", "struct", "impl", "mut"];
        if self.cur().kind == TokKind::Keyword && DIALECT_KEYWORDS.contains(&self.cur().value.as_str()) {
            return Ok(self.advance().value);
        }
        self.expect_ident()
    }

    /// `close` is the terminator token (`)` for `def`/normal calls, `:` for
    /// `lambda`). `allow_type_hints` must be false for lambda params — a
    /// lambda's own `:` (introducing its body) would otherwise be
    /// misparsed as a type-hint separator for the last parameter.
    fn parse_params(&mut self, close: &str, allow_type_hints: bool) -> OResult<Vec<Param>> {
        let mut params = Vec::new();
        while !self.is_op(close) && !self.at_eof() {
            let is_vararg = self.eat_op("*") && !self.is_op("*");
            let is_kwarg = if !is_vararg { self.eat_op("**") } else { false };
            let name = self.expect_param_name()?;
            if allow_type_hints && self.eat_op(":") {
                self.parse_type_hint_ignored()?;
            }
            let default = if self.eat_op("=") { Some(self.parse_expr()?) } else { None };
            params.push(Param { name, default, is_vararg, is_kwarg });
            if !self.eat_op(",") {
                break;
            }
        }
        Ok(params)
    }

    fn parse_classdef(&mut self) -> OResult<ClassDef> {
        let start = self.start();
        self.expect_kw("class")?;
        let name = self.expect_ident()?;
        let mut bases = Vec::new();
        if self.eat_op("(") {
            while !self.is_op(")") && !self.at_eof() {
                bases.push(self.expect_ident()?);
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op(")")?;
        }
        self.expect_op(":")?;
        self.skip_newlines();
        if self.cur().kind != TokKind::Indent {
            return Err(self.err("expected an indented class body"));
        }
        self.advance();
        let mut methods = Vec::new();
        let mut class_vars = Vec::new();
        loop {
            self.skip_newlines();
            if self.cur().kind == TokKind::Dedent || self.at_eof() {
                break;
            }
            if self.is_kw("pass") {
                self.advance();
                self.end_simple_stmt()?;
                continue;
            }
            let mut decorators = Vec::new();
            while self.eat_op("@") {
                decorators.push(self.parse_expr()?);
                self.skip_newlines();
            }
            if self.is_kw("def") || self.is_kw("async") {
                methods.push(self.parse_fndef(decorators)?);
            } else {
                let vname = self.expect_ident()?;
                if self.eat_op(":") {
                    self.parse_type_hint_ignored()?;
                }
                self.expect_op("=")?;
                let val = self.parse_expr()?;
                self.end_simple_stmt()?;
                class_vars.push((vname, val));
            }
        }
        if self.cur().kind == TokKind::Dedent {
            self.advance();
        }
        Ok(ClassDef { name, bases, methods, class_vars, span: self.sp(start) })
    }

    fn parse_if(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.expect_kw("if")?;
        let cond = self.parse_expr()?;
        let body = self.parse_block()?;
        let mut branches = vec![(cond, body)];
        let mut orelse = Vec::new();
        loop {
            self.skip_newlines_peek_only();
            if self.is_kw("elif") {
                self.advance();
                let c = self.parse_expr()?;
                let b = self.parse_block()?;
                branches.push((c, b));
                continue;
            }
            if self.is_kw("else") {
                self.advance();
                orelse = self.parse_block()?;
            }
            break;
        }
        Ok(Stmt::If { branches, orelse, span: self.sp(start) })
    }

    /// Skips newlines *only* when what follows is a continuation keyword
    /// (`elif`/`else`/`except`/`finally`) at the same statement level —
    /// otherwise leaves the newline alone so the outer block-scanner sees it.
    fn skip_newlines_peek_only(&mut self) {
        let save = self.p;
        while self.cur().kind == TokKind::Newline {
            self.advance();
        }
        if !(self.is_kw("elif") || self.is_kw("else") || self.is_kw("except") || self.is_kw("finally")) {
            self.p = save;
        }
    }

    fn parse_while(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.expect_kw("while")?;
        let cond = self.parse_expr()?;
        let body = self.parse_block()?;
        let mut orelse = Vec::new();
        self.skip_newlines_peek_only();
        if self.eat_kw("else") {
            orelse = self.parse_block()?;
        }
        Ok(Stmt::While { cond, body, orelse, span: self.sp(start) })
    }

    /// `x` or `x, y, z` — a `for`/comprehension target list (tuple unpacking).
    fn parse_for_target(&mut self) -> OResult<Vec<String>> {
        let mut names = vec![self.expect_ident()?];
        while self.eat_op(",") {
            names.push(self.expect_ident()?);
        }
        Ok(names)
    }

    fn parse_for(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.expect_kw("for")?;
        let target = self.parse_for_target()?;
        self.expect_kw("in")?;
        let iter = self.parse_expr()?;
        let body = self.parse_block()?;
        let mut orelse = Vec::new();
        self.skip_newlines_peek_only();
        if self.eat_kw("else") {
            orelse = self.parse_block()?;
        }
        Ok(Stmt::For { target, iter, body, orelse, span: self.sp(start) })
    }

    fn parse_try(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.expect_kw("try")?;
        let body = self.parse_block()?;
        let mut handlers = Vec::new();
        loop {
            self.skip_newlines_peek_only();
            if !self.is_kw("except") {
                break;
            }
            self.advance();
            let mut exc_type = None;
            let mut bind = None;
            if !self.is_op(":") {
                exc_type = Some(self.expect_ident()?);
                if self.eat_kw("as") {
                    bind = Some(self.expect_ident()?);
                }
            }
            let hbody = self.parse_block()?;
            handlers.push(ExceptClause { exc_type, bind, body: hbody });
        }
        let mut orelse = Vec::new();
        self.skip_newlines_peek_only();
        if self.eat_kw("else") {
            orelse = self.parse_block()?;
        }
        let mut finally = Vec::new();
        self.skip_newlines_peek_only();
        if self.eat_kw("finally") {
            finally = self.parse_block()?;
        }
        Ok(Stmt::Try { body, handlers, orelse, finally, span: self.sp(start) })
    }

    fn parse_import(&mut self) -> OResult<Stmt> {
        let start = self.start();
        if self.eat_kw("from") {
            let mut path = self.expect_ident()?;
            while self.eat_op(".") {
                path.push('.');
                path.push_str(&self.expect_ident()?);
            }
            self.expect_kw("import")?;
            // `from x import *` / `from x import a, b` — bootstrap treats
            // the whole clause as one import of `path`; individual-name
            // binding is future work.
            if !self.eat_op("*") {
                self.expect_ident()?;
                while self.eat_op(",") {
                    self.expect_ident()?;
                }
            }
            self.end_simple_stmt()?;
            return Ok(Stmt::Import { path, alias: None, span: self.sp(start) });
        }
        self.expect_kw("import")?;
        let mut path = self.expect_ident()?;
        while self.eat_op(".") {
            path.push('.');
            path.push_str(&self.expect_ident()?);
        }
        let alias = if self.eat_kw("as") { Some(self.expect_ident()?) } else { None };
        self.end_simple_stmt()?;
        Ok(Stmt::Import { path, alias, span: self.sp(start) })
    }

    /// `use path::item` / `use path::{a, b, c}` — Rust-style grouped import,
    /// recorded as a whole-module `Stmt::Import` of `path` (individual
    /// imported names discarded), matching this parser's existing
    /// `from x import a, b` treatment.
    fn parse_use(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.expect_kw("use")?;
        let mut path = self.expect_ident()?;
        while self.eat_op(":") {
            self.expect_op(":")?;
            if self.eat_op("{") {
                self.expect_ident()?;
                while self.eat_op(",") {
                    if self.is_op("}") {
                        break;
                    }
                    self.expect_ident()?;
                }
                self.expect_op("}")?;
                break;
            }
            path.push('.');
            path.push_str(&self.expect_ident()?);
        }
        self.end_simple_stmt()?;
        Ok(Stmt::Import { path, alias: None, span: self.sp(start) })
    }

    /// `layer`/`model`/`pipeline NAME { field: expr, ... }` (see
    /// `ast::Stmt::ConfigBlock`). Fields are newline- or comma-separated —
    /// inside `{ }` this lexer emits no `Newline` tokens (bracket_depth > 0,
    /// see `lexer.rs`), so entries are just parsed back-to-back with an
    /// optional `,` consumed between them.
    fn parse_config_block(&mut self) -> OResult<Stmt> {
        let start = self.start();
        let kind = self.advance().value; // "layer" | "model" | "pipeline" | "stage"
        let name = self.expect_ident()?;
        self.expect_op("{")?;
        let mut fields = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            // A nested named sub-block (`stage NAME { ... }`, no `:`) — the
            // omni-integration `pipeline` DSL nests these inside itself.
            // Recorded as one field (its internal structure isn't needed
            // for the parse-only `check` bar).
            if self.cur().kind == TokKind::Ident
                && self.toks.get(self.p + 1).is_some_and(|t| t.kind == TokKind::Ident)
                && self.toks.get(self.p + 2).is_some_and(|t| t.value == "{")
            {
                let stage_kind = self.cur().value.clone();
                let nested = self.parse_config_block()?; // consumes kind+name+body itself
                fields.push((stage_kind, Expr::None_ { span: self.sp(start) }));
                let _ = nested;
                self.eat_op(",");
                continue;
            }
            let field = self.expect_field_name()?;
            // Method shorthand (`evolve` blocks): `name(params) [-> Type] {
            // body }`, `fn`-less. Parsed as a function and discarded — not
            // needed for the parse-only `check` bar.
            if self.is_op("(") {
                let start2 = self.start();
                self.advance();
                let mut params = Vec::new();
                while !self.is_op(")") && !self.at_eof() {
                    let pname = self.expect_param_name()?;
                    if self.eat_op(":") {
                        self.parse_rust_type()?;
                    }
                    params.push(Param { name: pname, default: None, is_vararg: false, is_kwarg: false });
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op(")")?;
                if self.eat_op("->") {
                    self.parse_rust_type()?;
                }
                let body = self.parse_rust_block()?;
                let _ = FnDef { name: field.clone(), params, body, decorators: Vec::new(), is_async: false, span: self.sp(start2) };
                fields.push((field, Expr::None_ { span: self.sp(start) }));
                self.eat_op(",");
                continue;
            }
            self.expect_op(":")?;
            // Field values in this DSL vary widely (plain expressions,
            // `name: Type, name: Type` typed-param lists, `x => expr`
            // arrow-lambdas, generic types) — skipped generically by
            // bracket-depth rather than parsed as a strict expression,
            // since only "this parses without error" matters here, not the
            // value's structure.
            self.skip_config_value()?;
            fields.push((field, Expr::None_ { span: self.sp(start) }));
            self.eat_op(",");
        }
        self.expect_op("}")?;
        Ok(Stmt::ConfigBlock { kind, name, fields, span: self.sp(start) })
    }

    /// Advances past a config-block field's value, stopping (without
    /// consuming) at depth-0 `}` or the first token that starts a new
    /// source line. This DSL's fields are really newline-separated (`input:
    /// [1]` / `output: [768]` each on their own line, no trailing commas) —
    /// but this lexer suppresses `Newline` tokens once inside `{ }`
    /// (`bracket_depth > 0`, see `lexer.rs`), so the *line number* on each
    /// token's own span is used directly instead, sidestepping that
    /// suppression entirely. Depth-0 `,` is intentionally NOT a stop
    /// condition: some fields legitimately hold a comma-separated list on
    /// one line (`input: prompt, chosen`). Tracks `(`/`[`/`{`/`<` as one
    /// unified nesting depth so a value that itself spans multiple lines
    /// inside brackets (`messages: [\n  a,\n  b\n]`) isn't cut early.
    fn skip_config_value(&mut self) -> OResult<()> {
        let mut depth = 0i32;
        let mut last_line = self.cur().span.start.line;
        loop {
            if self.cur().kind == TokKind::Eof {
                return Err(self.err("unexpected end of file inside a config-block value"));
            }
            if depth == 0 && (self.is_op("}") || self.cur().span.start.line > last_line) {
                return Ok(());
            }
            if self.cur().kind == TokKind::Op {
                match self.cur().value.as_str() {
                    "(" | "[" | "{" | "<" => depth += 1,
                    ")" | "]" | "}" | ">" => depth -= 1,
                    _ => {}
                }
            }
            last_line = self.cur().span.end.line;
            self.advance();
        }
    }

    /// A Rust-style type annotation: `&`/`&mut` reference sigils, an
    /// identifier path (`a::b::C`), optional `<...>` generic args (which may
    /// themselves contain balanced `(`/`[`/`<`), and optional `(...)` tuple
    /// members (for `()`/`(A, B)`). Fully discarded — Sylva is dynamically
    /// typed even in this Rust-syntax dialect.
    fn parse_rust_type(&mut self) -> OResult<()> {
        self.eat_op("&");
        self.eat_kw("mut");
        if self.eat_op("[") {
            // Slice/array type: `[T]`, `[T; N]` (`N` is an int literal or
            // const identifier, not itself a type — just skipped, one token).
            self.parse_rust_type()?;
            if self.eat_op(";") {
                self.advance();
            }
            self.expect_op("]")?;
            return Ok(());
        }
        if self.eat_op("(") {
            // Tuple type: `()`, `(A, B)`.
            while !self.is_op(")") && !self.at_eof() {
                self.parse_rust_type()?;
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op(")")?;
            return Ok(());
        }
        self.expect_ident()?;
        while self.eat_op(":") {
            self.expect_op(":")?;
            self.expect_ident()?;
        }
        if self.eat_op("<") {
            loop {
                self.parse_rust_type()?;
                if !self.eat_op(",") {
                    break;
                }
            }
            if !self.eat_close_angle() {
                return Err(self.err(format!("expected '>' but found '{}'", self.cur_desc())));
            }
        }
        Ok(())
    }

    /// Consumes one `>` closing a generic's `<...>`. Adjacent closing
    /// angles with no space (`Vec<Vec<T>>`) lex as a single `>>`/`>>>>`
    /// token (see `lexer.rs`'s `'>' =>` case) rather than one `>` per
    /// nesting level, so a multi-char closer is "split": this call consumes
    /// one level's worth by shrinking the token in place, leaving the rest
    /// for the enclosing `parse_rust_type` call's own `eat_close_angle`.
    fn eat_close_angle(&mut self) -> bool {
        if self.is_op(">") {
            self.advance();
            true
        } else if self.is_op(">>") {
            self.toks[self.p].value = ">".to_string();
            true
        } else {
            false
        }
    }

    /// `[pub] fn name([&[mut] self,] [name: Type],*) [-> Type] { rust_stmts }`.
    fn parse_rust_fn(&mut self) -> OResult<FnDef> {
        let start = self.start();
        self.eat_kw("pub");
        self.expect_kw("fn")?;
        let name = self.expect_ident()?;
        self.expect_op("(")?;
        let mut params = Vec::new();
        while !self.is_op(")") && !self.at_eof() {
            self.eat_op("&");
            self.eat_kw("mut");
            let pname = self.expect_param_name()?;
            if self.eat_op(":") {
                self.parse_rust_type()?;
            }
            params.push(Param { name: pname, default: None, is_vararg: false, is_kwarg: false });
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        if self.eat_op("->") {
            self.parse_rust_type()?;
        }
        let body = self.parse_rust_block()?;
        Ok(FnDef { name, params, body, decorators: Vec::new(), is_async: false, span: self.sp(start) })
    }

    /// `[pub] struct Name { field: Type, ... }` — parsed as a `ClassDef`
    /// with no methods; each field becomes a class-level var initialized to
    /// `None` (Sylva has no distinct "declared but unset" state).
    fn parse_rust_struct(&mut self) -> OResult<ClassDef> {
        let start = self.start();
        self.eat_kw("pub");
        self.expect_kw("struct")?;
        let name = self.expect_ident()?;
        self.expect_op("{")?;
        let mut class_vars = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            self.eat_kw("pub");
            let field = self.expect_field_name()?;
            self.expect_op(":")?;
            self.parse_rust_type()?;
            class_vars.push((field, Expr::None_ { span: self.sp(start) }));
            self.eat_op(",");
        }
        self.expect_op("}")?;
        Ok(ClassDef { name, bases: Vec::new(), methods: Vec::new(), class_vars, span: self.sp(start) })
    }

    /// `impl Name { [pub] fn ... }*` — parsed as a `ClassDef` whose methods
    /// are the `impl` block's functions (merging with a same-named `struct`
    /// isn't attempted; both just become independent module-level items,
    /// which is fine at the parse-only `check` level this exists for).
    fn parse_rust_impl(&mut self) -> OResult<ClassDef> {
        let start = self.start();
        self.expect_kw("impl")?;
        let name = self.expect_ident()?;
        self.expect_op("{")?;
        let mut methods = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            methods.push(self.parse_rust_fn()?);
        }
        self.expect_op("}")?;
        Ok(ClassDef { name, bases: Vec::new(), methods, class_vars: Vec::new(), span: self.sp(start) })
    }

    /// `[pub] mod Name { items }` — items are ordinary top-level statements
    /// (struct/fn/impl/nested mod/etc.), parsed the same way as the
    /// module's own top level.
    fn parse_rust_mod(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.eat_kw("pub");
        self.expect_kw("mod")?;
        let name = self.expect_ident()?;
        self.expect_op("{")?;
        let mut body = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            body.push(self.parse_stmt()?);
        }
        self.expect_op("}")?;
        Ok(Stmt::Mod { name, body, span: self.sp(start) })
    }

    /// `{ stmts }` for the Rust-syntax dialect. Unlike `parse_block`
    /// (Python-shape, `:` + Indent/Dedent), no explicit statement
    /// terminator is required between entries — the lexer emits no
    /// `Newline` inside braces, and Rust statements are self-delimiting by
    /// their own grammar (a trailing `;`, if present, is just consumed and
    /// discarded as if it were whitespace).
    fn parse_rust_block(&mut self) -> OResult<Vec<Stmt>> {
        self.expect_op("{")?;
        let mut out = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            out.push(self.parse_rust_stmt()?);
            self.eat_op(";");
        }
        self.expect_op("}")?;
        Ok(out)
    }

    fn parse_rust_stmt(&mut self) -> OResult<Stmt> {
        let start = self.start();
        if self.is_kw("fn") || (self.is_kw("pub") && self.toks.get(self.p + 1).is_some_and(|t| t.value == "fn")) {
            return Ok(Stmt::FnDef(self.parse_rust_fn()?));
        }
        if self.eat_kw("let") || self.eat_kw("const") {
            self.eat_kw("mut");
            let name = self.expect_ident()?;
            if self.eat_op(":") {
                self.parse_rust_type()?;
            }
            let value = if self.eat_op("=") { Some(self.parse_rust_expr_with_ref()?) } else { None };
            return Ok(Stmt::Let { name, value, span: self.sp(start) });
        }
        if self.eat_kw("return") {
            let value = if self.is_op("}") || self.is_op(";") { None } else { Some(self.parse_expr()?) };
            return Ok(Stmt::Return { value, span: self.sp(start) });
        }
        if self.is_kw("if") {
            return self.parse_rust_if();
        }
        // `for (a, &b, ..) in iter { stmts }` — a leading `&`/`&mut` on any
        // destructured binding is discarded (this interpreter's `for`
        // target is just a flat list of names; see `ast::Stmt::For`).
        if self.eat_kw("for") {
            let mut target = Vec::new();
            if self.eat_op("(") {
                loop {
                    self.eat_op("&");
                    self.eat_kw("mut");
                    target.push(self.expect_param_name()?);
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op(")")?;
            } else {
                self.eat_op("&");
                self.eat_kw("mut");
                target.push(self.expect_param_name()?);
            }
            self.expect_kw("in")?;
            // Same struct-literal-vs-block ambiguity as `if`/`while` (see
            // `allow_struct_lit`'s doc comment): `for x in xs { .. }`'s `xs`
            // must not swallow the following `{` as a struct literal.
            self.allow_struct_lit = false;
            let lo = self.parse_rust_expr_with_ref()?;
            // `for i in lo..hi { .. }` — `..` isn't consumed by the general
            // expression grammar (see `parse_subscript`'s doc comment: only
            // recognized there and here), so it's still available to check
            // for after parsing `lo`. Rewritten to a call to this
            // bootstrap's existing `range(lo, hi)` builtin.
            let iter = if self.eat_op("..") {
                let hi = self.parse_rust_expr_with_ref()?;
                Expr::Call { func: Box::new(Expr::Ident { name: "range".to_string(), span: self.sp(start) }), args: vec![lo, hi], kwargs: Vec::new(), span: self.sp(start) }
            } else {
                lo
            };
            self.allow_struct_lit = true;
            let body = self.parse_rust_block()?;
            return Ok(Stmt::For { target, iter, body, orelse: Vec::new(), span: self.sp(start) });
        }
        // Expression statement or assignment (`target = expr`, `target op= expr`).
        let expr = self.parse_rust_expr_with_ref()?;
        if let Some(op) = self.peek_aug_assign() {
            self.advance();
            let rhs = self.parse_rust_expr_with_ref()?;
            let value = if op == "=" {
                rhs
            } else {
                let binop = op.trim_end_matches('=').to_string();
                Expr::BinOp { op: binop, left: Box::new(expr.clone()), right: Box::new(rhs), span: self.sp(start) }
            };
            return Ok(Stmt::Assign { target: expr, value, span: self.sp(start) });
        }
        Ok(Stmt::Expr(expr))
    }

    /// `if [let PATTERN =] cond { stmts } [else { stmts }]`. A `let`
    /// pattern is reduced to just evaluating the right-hand expression for
    /// truthiness (`Some`/`Ok`/`None`/`Err` aren't modeled as distinct
    /// values by this interpreter) — sufficient for parsing, not a real
    /// Option/Result match.
    fn parse_rust_if(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.expect_kw("if")?;
        self.allow_struct_lit = false; // see field doc comment
        let cond = if self.eat_kw("let") {
            // Pattern: `Some(name)` / `Ok(name)` / a bare identifier.
            self.expect_ident()?;
            if self.eat_op("(") {
                self.expect_ident()?;
                self.expect_op(")")?;
            }
            self.expect_op("=")?;
            self.parse_rust_expr_with_ref()?
        } else {
            self.parse_rust_expr_with_ref()?
        };
        self.allow_struct_lit = true;
        let body = self.parse_rust_block()?;
        let mut orelse = Vec::new();
        if self.eat_kw("else") {
            orelse = if self.is_kw("if") {
                vec![self.parse_rust_if()?]
            } else {
                self.parse_rust_block()?
            };
        }
        Ok(Stmt::If { branches: vec![(cond, body)], orelse, span: self.sp(start) })
    }

    /// `parse_expr`, but tolerates a leading `&`/`&mut` (address-of) by
    /// discarding it — this dialect's expressions are otherwise identical
    /// to Sylva's native ones (`format!`/`vec!` macros, method calls, `?`,
    /// etc. are already handled by the shared `parse_postfix`/`parse_atom`).
    fn parse_rust_expr_with_ref(&mut self) -> OResult<Expr> {
        self.eat_op("&");
        self.eat_kw("mut");
        // Save/restore (not unconditionally reset to `true`): this call
        // nests — a closure literal's body is itself parsed via this same
        // function (see `parse_lambda`'s `|params| body` case) — and an
        // inner call finishing must not clobber an outer call's gate.
        let prev = self.allow_python_ternary;
        self.allow_python_ternary = false;
        let e = self.parse_expr();
        self.allow_python_ternary = prev;
        e
    }

    /// `match subject { Pat(bindings) => arm, .. }` (the `match` keyword
    /// itself is already consumed by the caller, `parse_atom`). See
    /// `ast::Expr::Match`'s doc comment for what's modeled vs. simplified.
    /// The value a `{ .. }` block "evaluates to" when used where an
    /// expression is expected (closure/match-arm/if-expression bodies in
    /// the Rust dialect): its last `Stmt::Expr`/`Stmt::Return`, or a `None_`
    /// placeholder if it has none. Earlier statements are parsed (so they
    /// must be valid) but their effects are dropped — this interpreter
    /// doesn't need to execute them correctly for the parse-only `check`
    /// bar (see `ast::Expr::Match`'s doc comment for the same tradeoff).
    fn block_trailing_expr(stmts: Vec<Stmt>, fallback_span: Span) -> Expr {
        stmts
            .into_iter()
            .rev()
            .find_map(|s| match s {
                Stmt::Expr(e) => Some(e),
                Stmt::Return { value, .. } => value,
                _ => None,
            })
            .unwrap_or(Expr::None_ { span: fallback_span })
    }

    /// `if cond { expr } else { expr }` / `if cond { expr } else if .. { .. }`
    /// used in expression position (as opposed to `parse_rust_if`'s
    /// statement form) — e.g. `let k = if top_k <= 0 { 5 } else { top_k }`.
    /// The `if` keyword itself is already consumed by the caller.
    fn parse_rust_if_expr(&mut self, start: Pos) -> OResult<Expr> {
        let prev = self.allow_struct_lit;
        self.allow_struct_lit = false; // same ambiguity as the statement form
        let cond = self.parse_rust_expr_with_ref()?;
        self.allow_struct_lit = prev;
        let then_span = self.sp(start);
        let body = Self::block_trailing_expr(self.parse_rust_block()?, then_span);
        self.expect_kw("else")?;
        let orelse = if self.is_kw("if") {
            self.advance();
            self.parse_rust_if_expr(start)?
        } else {
            let else_span = self.sp(start);
            Self::block_trailing_expr(self.parse_rust_block()?, else_span)
        };
        Ok(Expr::Ternary { body: Box::new(body), cond: Box::new(cond), orelse: Box::new(orelse), span: self.sp(start) })
    }

    fn parse_match(&mut self, start: Pos) -> OResult<Expr> {
        let prev = self.allow_struct_lit;
        self.allow_struct_lit = false; // same ambiguity as `if`/`for`/`while`
        let subject = self.parse_rust_expr_with_ref()?;
        self.allow_struct_lit = prev;
        self.expect_op("{")?;
        let mut arms = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            // Pattern: `Name`, `Name(a, b)`, `_`, or Sylva's own `None`
            // literal token (lexed specially, not as a plain identifier).
            let pat_name = if self.cur().kind == TokKind::None_ {
                self.advance();
                "None".to_string()
            } else {
                self.expect_ident()?
            };
            let mut bindings = Vec::new();
            if self.eat_op("(") {
                while !self.is_op(")") && !self.at_eof() {
                    bindings.push(self.expect_param_name()?);
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op(")")?;
            }
            self.expect_op("=>")?;
            // An arm body may be a real expression, or a statement that
            // doesn't produce one for this match (`return expr`, `break`,
            // `continue`) — see `ast::Expr::Match`'s doc comment.
            let body = if self.eat_kw("return") {
                if !self.is_op(",") && !self.is_op("}") {
                    self.parse_rust_expr_with_ref()?;
                }
                Expr::None_ { span: self.sp(start) }
            } else if self.eat_kw("break") || self.eat_kw("continue") {
                Expr::None_ { span: self.sp(start) }
            } else if self.is_op("{") {
                let stmts = self.parse_rust_block()?;
                stmts
                    .into_iter()
                    .rev()
                    .find_map(|s| match s {
                        Stmt::Expr(e) => Some(e),
                        Stmt::Return { value, .. } => value,
                        _ => None,
                    })
                    .unwrap_or(Expr::None_ { span: self.sp(start) })
            } else {
                self.parse_rust_expr_with_ref()?
            };
            arms.push((pat_name, bindings, body));
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op("}")?;
        Ok(Expr::Match { subject: Box::new(subject), arms, span: self.sp(start) })
    }

    // ── expressions (precedence climbing) ────────────────────────────────

    pub fn parse_expr(&mut self) -> OResult<Expr> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> OResult<Expr> {
        let start = self.start();
        let body = self.parse_lambda()?;
        // Python's `body if cond else orelse` ternary — gated on
        // `allow_python_ternary` because the Rust-syntax dialect
        // (`parse_rust_expr_with_ref`) shares this same expression grammar,
        // and there a bare `if` immediately after an expression is virtually
        // always the *next statement* (`let x = expr\nif cond { .. }`), not
        // a ternary continuation; without the gate this would consume that
        // `if` and then fail requiring a Python-style `else`.
        if self.allow_python_ternary && self.eat_kw("if") {
            let cond = self.parse_lambda()?;
            self.expect_kw("else")?;
            let orelse = self.parse_ternary()?;
            return Ok(Expr::Ternary { body: Box::new(body), cond: Box::new(cond), orelse: Box::new(orelse), span: self.sp(start) });
        }
        Ok(body)
    }

    fn parse_lambda(&mut self) -> OResult<Expr> {
        let start = self.start();
        if self.eat_kw("lambda") {
            let params = if self.is_op(":") { vec![] } else { self.parse_params(":", false)? };
            self.expect_op(":")?;
            let body = self.parse_ternary()?;
            return Ok(Expr::Lambda { params, body: Box::new(body), span: self.sp(start) });
        }
        // Rust closure: `|x| x * x` / `|| expr` (no params) / `|x: T| -> T { .. }`.
        // Reuses `Expr::Lambda` — Sylva's own closures (`lambda x: ...`) are
        // already a single-expression body, matching this dialect's common
        // case; a `{ .. }` block body is parsed via `parse_rust_block` and
        // wrapped so `Lambda.body` stays a single `Expr`.
        if self.is_op("|") || self.is_op("||") {
            let params = if self.eat_op("||") {
                Vec::new()
            } else {
                self.expect_op("|")?;
                let mut params = Vec::new();
                while !self.is_op("|") && !self.at_eof() {
                    // Tuple-destructuring param: `|(x, y)| ..`, possibly
                    // nested (`|(rank, (idx, score))| ..`). This
                    // interpreter's `Param` models one name per slot, so the
                    // pattern is flattened (recursively) into one synthetic
                    // joined name — never actually bound/invoked for these
                    // specs.
                    let pname = if self.is_op("(") {
                        self.parse_tuple_pattern_name()?
                    } else {
                        self.expect_param_name()?
                    };
                    if self.eat_op(":") {
                        self.parse_rust_type()?;
                    }
                    params.push(Param { name: pname, default: None, is_vararg: false, is_kwarg: false });
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op("|")?;
                params
            };
            if self.eat_op("->") {
                self.parse_rust_type()?;
            }
            // A `{ .. }` block body's last expression statement becomes the
            // (single-`Expr`) `Lambda.body` this AST models; earlier
            // statements are parsed (so they must be valid) but discarded —
            // this dialect's closures are only ever used inline, and this
            // interpreter doesn't need to execute them correctly for the
            // parse-only `check` bar.
            let body = if self.is_op("{") {
                let stmts = self.parse_rust_block()?;
                stmts
                    .into_iter()
                    .rev()
                    .find_map(|s| match s {
                        Stmt::Expr(e) => Some(e),
                        Stmt::Return { value, .. } => value,
                        _ => None,
                    })
                    .unwrap_or(Expr::None_ { span: self.sp(start) })
            } else {
                self.parse_rust_expr_with_ref()?
            };
            return Ok(Expr::Lambda { params, body: Box::new(body), span: self.sp(start) });
        }
        self.parse_or()
    }

    fn parse_or(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_and()?;
        // `||` here is unambiguously infix logical-or (`parse_lambda`'s
        // zero-param-closure `||` only ever appears in a fresh-operand
        // position, already consumed before `left` was parsed).
        while self.eat_kw("or") || self.eat_op("||") {
            let right = self.parse_and()?;
            left = Expr::BoolOp { op: "or".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_not()?;
        while self.eat_kw("and") || self.eat_op("&&") {
            let right = self.parse_not()?;
            left = Expr::BoolOp { op: "and".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> OResult<Expr> {
        let start = self.start();
        if self.eat_kw("not") {
            let e = self.parse_not()?;
            return Ok(Expr::Not { expr: Box::new(e), span: self.sp(start) });
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> OResult<Expr> {
        let start = self.start();
        let left = self.parse_bitor()?;
        let mut ops = Vec::new();
        let mut comparators = Vec::new();
        loop {
            let op = if self.is_op("==") || self.is_op("!=") || self.is_op("<") || self.is_op("<=") || self.is_op(">") || self.is_op(">=") {
                self.advance().value
            } else if self.is_kw("in") {
                self.advance();
                "in".to_string()
            } else if self.is_kw("is") {
                self.advance();
                if self.eat_kw("not") { "is not".to_string() } else { "is".to_string() }
            } else if self.is_kw("not") && self.toks.get(self.p + 1).is_some_and(|t| t.value == "in") {
                self.advance();
                self.advance();
                "not in".to_string()
            } else {
                break;
            };
            ops.push(op);
            comparators.push(self.parse_bitor()?);
        }
        if ops.is_empty() {
            return Ok(left);
        }
        Ok(Expr::Compare { left: Box::new(left), ops, comparators, span: self.sp(start) })
    }

    fn parse_bitor(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_bitxor()?;
        while self.is_op("|") {
            let op = self.advance().value;
            let right = self.parse_bitxor()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    fn parse_bitxor(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_bitand()?;
        while self.is_op("^") {
            let op = self.advance().value;
            let right = self.parse_bitand()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    fn parse_bitand(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_shift()?;
        while self.is_op("&") {
            let op = self.advance().value;
            let right = self.parse_shift()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_add()?;
        while self.is_op("<<") || self.is_op(">>") {
            let op = self.advance().value;
            let right = self.parse_add()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_mul()?;
        loop {
            if self.is_op("+") || self.is_op("-") {
                let op = self.advance().value;
                let right = self.parse_mul()?;
                left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), span: self.sp(start) };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_unary()?;
        loop {
            if self.is_op("*") || self.is_op("/") || self.is_op("%") {
                let op = self.advance().value;
                let right = self.parse_unary()?;
                left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), span: self.sp(start) };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> OResult<Expr> {
        let start = self.start();
        if self.is_op("-") || self.is_op("+") {
            let op = self.advance().value;
            let e = self.parse_unary()?;
            return Ok(Expr::UnaryOp { op, expr: Box::new(e), span: self.sp(start) });
        }
        // Rust dereference (`*x`) — a no-op prefix in this dialect (this
        // interpreter has no pointer/reference values to dereference); safe
        // to recognize unconditionally here since a fresh-operand position
        // (as opposed to `parse_mul`'s infix `a * b`) is never ambiguous
        // with multiplication.
        if self.is_op("*") {
            self.advance();
            return self.parse_unary();
        }
        // Rust address-of (`&x`, `&mut x`) — also a no-op prefix here.
        // Handled at this shared `parse_unary` level (not just in
        // `parse_rust_expr_with_ref`) since `&expr` also appears as a plain
        // call argument (`f(&x)`) and struct-literal field value, which go
        // through the ordinary `parse_expr` path.
        if self.is_op("&") {
            self.advance();
            self.eat_kw("mut");
            return self.parse_unary();
        }
        if self.eat_kw("await") {
            let e = self.parse_unary()?;
            return Ok(Expr::Await { expr: Box::new(e), span: self.sp(start) });
        }
        if self.eat_kw("yield") {
            let value = if self.at_stmt_end() { None } else { Some(Box::new(self.parse_expr()?)) };
            return Ok(Expr::Yield { value, span: self.sp(start) });
        }
        self.parse_power()
    }

    fn parse_power(&mut self) -> OResult<Expr> {
        let start = self.start();
        let base = self.parse_postfix()?;
        if self.eat_op("**") {
            let exp = self.parse_unary()?; // right-associative
            return Ok(Expr::BinOp { op: "**".to_string(), left: Box::new(base), right: Box::new(exp), span: self.sp(start) });
        }
        Ok(base)
    }

    fn parse_postfix(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut e = self.parse_atom()?;
        loop {
            if self.eat_op("?") {
                e = Expr::Try { inner: Box::new(e), span: self.sp(start) };
            } else if self.eat_op(".") {
                // Tuple field access (`t.0`, `t.1`) — the index lexes as a
                // plain `Int` token, not an identifier.
                let name = if self.cur().kind == TokKind::Int {
                    self.advance().value
                } else {
                    self.expect_ident()?
                };
                // Turbofish: `.collect::<Vec<_>>()` — the explicit generic
                // args are parsed (for balance) and discarded.
                if self.is_op(":") && self.peek_op_at(1) == Some(":") && self.peek_op_at(2) == Some("<") {
                    self.advance();
                    self.advance();
                    self.advance();
                    loop {
                        self.parse_rust_type()?;
                        if !self.eat_op(",") {
                            break;
                        }
                    }
                    if !self.eat_close_angle() {
                        return Err(self.err(format!("expected '>' but found '{}'", self.cur_desc())));
                    }
                }
                e = Expr::Attr { obj: Box::new(e), name, span: self.sp(start) };
            } else if self.is_op("(") {
                self.advance();
                let (args, kwargs) = self.parse_call_args()?;
                self.expect_op(")")?;
                e = Expr::Call { func: Box::new(e), args, kwargs, span: self.sp(start) };
            } else if self.eat_op("[") {
                e = self.parse_subscript(e, start)?;
            } else if self.is_op("!") && matches!(self.peek_op_at(1), Some("(") | Some("[")) {
                // Rust-macro call: `name!(...)` (treated as an ordinary
                // call) or `name![...]` / `name![value; count]` (a list
                // literal, or `Expr::Repeat` for the `; count` form).
                self.advance(); // '!'
                if self.eat_op("(") {
                    let (args, kwargs) = self.parse_call_args()?;
                    self.expect_op(")")?;
                    e = Expr::Call { func: Box::new(e), args, kwargs, span: self.sp(start) };
                } else {
                    self.expect_op("[")?;
                    if self.is_op("]") {
                        self.advance();
                        e = Expr::List { elems: Vec::new(), span: self.sp(start) };
                    } else {
                        let first = self.parse_expr()?;
                        if self.eat_op(";") {
                            let count = self.parse_expr()?;
                            self.expect_op("]")?;
                            e = Expr::Repeat { value: Box::new(first), count: Box::new(count), span: self.sp(start) };
                        } else {
                            let mut elems = vec![first];
                            while self.eat_op(",") {
                                if self.is_op("]") {
                                    break;
                                }
                                elems.push(self.parse_expr()?);
                            }
                            self.expect_op("]")?;
                            e = Expr::List { elems, span: self.sp(start) };
                        }
                    }
                }
            } else if self.eat_kw("as") {
                // Rust cast (`b as u64`) — the target type is discarded;
                // this interpreter doesn't model distinct numeric widths.
                self.parse_rust_type()?;
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_subscript(&mut self, obj: Expr, start: Pos) -> OResult<Expr> {
        // Rust range slice: `x[a..b]` / `x[a..]` / `x[..b]` / `x[..]` —
        // distinct from the Python-style `x[lo:hi:step]` slicing below
        // (single `:`, handled by the existing branch), since this dialect
        // uses `..` (not lexed/consumed anywhere else in this grammar, so
        // unambiguous to check for after an optional `lo`).
        if self.is_op("..") {
            self.advance();
            let hi = if self.is_op("]") { None } else { Some(Box::new(self.parse_expr()?)) };
            self.expect_op("]")?;
            return Ok(Expr::Slice { obj: Box::new(obj), lo: None, hi, step: None, span: self.sp(start) });
        }
        // Distinguish `x[i]` from `x[lo:hi]` / `x[lo:hi:step]`.
        let lo = if self.is_op(":") { None } else { Some(Box::new(self.parse_expr()?)) };
        if self.eat_op("..") {
            let hi = if self.is_op("]") { None } else { Some(Box::new(self.parse_expr()?)) };
            self.expect_op("]")?;
            return Ok(Expr::Slice { obj: Box::new(obj), lo, hi, step: None, span: self.sp(start) });
        }
        if self.eat_op(":") {
            let hi = if self.is_op(":") || self.is_op("]") { None } else { Some(Box::new(self.parse_expr()?)) };
            let step = if self.eat_op(":") {
                if self.is_op("]") { None } else { Some(Box::new(self.parse_expr()?)) }
            } else {
                None
            };
            self.expect_op("]")?;
            return Ok(Expr::Slice { obj: Box::new(obj), lo, hi, step, span: self.sp(start) });
        }
        self.expect_op("]")?;
        Ok(Expr::Index { obj: Box::new(obj), index: lo.expect("index subscript must have an index"), span: self.sp(start) })
    }

    fn parse_call_args(&mut self) -> OResult<(Vec<Expr>, Vec<(String, Expr)>)> {
        let mut args = Vec::new();
        let mut kwargs = Vec::new();
        while !self.is_op(")") && !self.at_eof() {
            // kwarg: `name=expr` — lookahead ident '='
            if self.cur().kind == TokKind::Ident && self.toks.get(self.p + 1).is_some_and(|t| t.kind == TokKind::Op && t.value == "=") {
                let name = self.advance().value;
                self.advance(); // '='
                kwargs.push((name, self.parse_expr()?));
            } else {
                args.push(self.parse_expr()?);
            }
            if !self.eat_op(",") {
                break;
            }
        }
        Ok((args, kwargs))
    }

    fn parse_atom(&mut self) -> OResult<Expr> {
        let start = self.start();
        let t = self.cur().clone();
        match t.kind {
            TokKind::Int => {
                self.advance();
                return Ok(Expr::Int { v: t.value.parse().unwrap_or(0), span: self.sp(start) });
            }
            TokKind::Float => {
                self.advance();
                return Ok(Expr::Float { v: t.value.parse().unwrap_or(0.0), span: self.sp(start) });
            }
            TokKind::Str => {
                self.advance();
                return Ok(Expr::Str { v: t.value, span: self.sp(start) });
            }
            TokKind::FStr => {
                self.advance();
                let parts = self.parse_fstring_parts(&t.value, start)?;
                return Ok(Expr::FStr { parts, span: self.sp(start) });
            }
            TokKind::Bool => {
                self.advance();
                return Ok(Expr::Bool { v: t.value == "true", span: self.sp(start) });
            }
            TokKind::None_ => {
                self.advance();
                return Ok(Expr::None_ { span: self.sp(start) });
            }
            TokKind::Ident => {
                self.advance();
                // Rust namespaced path: `std::env::var` — folded into one
                // atomic identifier (`"std::env::var"`). This interpreter
                // has no module system to resolve it against; treated as an
                // opaque (probably-undefined-at-runtime, but parses fine)
                // name, which is all the parse-only `check` bar needs.
                let mut name = t.value;
                while self.is_op(":") && self.peek_op_at(1) == Some(":") {
                    self.advance();
                    self.advance();
                    name.push_str("::");
                    name.push_str(&self.expect_ident()?);
                }
                // Rust struct literal: `Name { field: expr, .. }` (see
                // `allow_struct_lit`'s doc comment for why this is gated).
                // Modeled as `Expr::Dict` with string-literal keys — this
                // interpreter has no distinct struct/record value, and a
                // dict is semantically close enough for the parse-only bar.
                if self.allow_struct_lit && self.is_op("{") {
                    self.advance();
                    let mut entries = Vec::new();
                    while !self.is_op("}") && !self.at_eof() {
                        let field = self.expect_field_name()?;
                        // Field-init shorthand: `{ config }` means
                        // `{ config: config }`.
                        let value = if self.eat_op(":") {
                            self.parse_expr()?
                        } else {
                            Expr::Ident { name: field.clone(), span: self.sp(start) }
                        };
                        entries.push((Expr::Str { v: field, span: self.sp(start) }, value));
                        if !self.eat_op(",") {
                            break;
                        }
                    }
                    self.expect_op("}")?;
                    return Ok(Expr::Dict { entries, span: self.sp(start) });
                }
                return Ok(Expr::Ident { name, span: self.sp(start) });
            }
            TokKind::Keyword if t.value == "self" => {
                self.advance();
                return Ok(Expr::Ident { name: "self".to_string(), span: self.sp(start) });
            }
            TokKind::Keyword if t.value == "match" => {
                self.advance();
                return self.parse_match(start);
            }
            TokKind::Keyword if t.value == "if" => {
                self.advance();
                return self.parse_rust_if_expr(start);
            }
            _ => {}
        }
        if self.eat_op("(") {
            if self.eat_op(")") {
                return Ok(Expr::Tuple { elems: vec![], span: self.sp(start) });
            }
            let first = self.parse_expr()?;
            if self.eat_op(",") {
                let mut elems = vec![first];
                while !self.is_op(")") && !self.at_eof() {
                    elems.push(self.parse_expr()?);
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op(")")?;
                return Ok(Expr::Tuple { elems, span: self.sp(start) });
            }
            self.expect_op(")")?;
            return Ok(first);
        }
        if self.eat_op("[") {
            if self.is_op("]") {
                self.advance();
                return Ok(Expr::List { elems: vec![], span: self.sp(start) });
            }
            let first = self.parse_expr()?;
            if self.is_kw("for") {
                return self.parse_list_comp(first, start);
            }
            let mut elems = vec![first];
            while self.eat_op(",") {
                if self.is_op("]") {
                    break;
                }
                elems.push(self.parse_expr()?);
            }
            self.expect_op("]")?;
            return Ok(Expr::List { elems, span: self.sp(start) });
        }
        if self.eat_op("{") {
            if self.is_op("}") {
                self.advance();
                return Ok(Expr::Dict { entries: vec![], span: self.sp(start) });
            }
            let key = self.parse_expr()?;
            self.expect_op(":")?;
            let val = self.parse_expr()?;
            if self.is_kw("for") {
                return self.parse_dict_comp(key, val, start);
            }
            let mut entries = vec![(key, val)];
            while self.eat_op(",") {
                if self.is_op("}") {
                    break;
                }
                let k = self.parse_expr()?;
                self.expect_op(":")?;
                let v = self.parse_expr()?;
                entries.push((k, v));
            }
            self.expect_op("}")?;
            return Ok(Expr::Dict { entries, span: self.sp(start) });
        }
        Err(self.err(format!("expected an expression but found '{}'", self.cur_desc())))
    }

    fn parse_list_comp(&mut self, expr: Expr, start: Pos) -> OResult<Expr> {
        self.expect_kw("for")?;
        let target = self.parse_for_target()?;
        self.expect_kw("in")?;
        let iter = self.parse_or()?;
        let cond = if self.eat_kw("if") { Some(Box::new(self.parse_or()?)) } else { None };
        self.expect_op("]")?;
        Ok(Expr::ListComp { expr: Box::new(expr), target, iter: Box::new(iter), cond, span: self.sp(start) })
    }

    fn parse_dict_comp(&mut self, key: Expr, value: Expr, start: Pos) -> OResult<Expr> {
        self.expect_kw("for")?;
        let target = self.parse_for_target()?;
        self.expect_kw("in")?;
        let iter = self.parse_or()?;
        let cond = if self.eat_kw("if") { Some(Box::new(self.parse_or()?)) } else { None };
        self.expect_op("}")?;
        Ok(Expr::DictComp { key: Box::new(key), value: Box::new(value), target, iter: Box::new(iter), cond, span: self.sp(start) })
    }

    /// Splits an f-string's raw text into literal/`{expr}` parts and
    /// recursively parses each `{expr}` with its own lexer+parser instance.
    fn parse_fstring_parts(&mut self, raw: &str, start: Pos) -> OResult<Vec<FStrPart>> {
        let mut parts = Vec::new();
        let chars: Vec<char> = raw.chars().collect();
        let mut i = 0;
        let mut lit = String::new();
        while i < chars.len() {
            if chars[i] == '{' && chars.get(i + 1) == Some(&'{') {
                lit.push('{');
                i += 2;
                continue;
            }
            if chars[i] == '}' && chars.get(i + 1) == Some(&'}') {
                lit.push('}');
                i += 2;
                continue;
            }
            if chars[i] == '{' {
                if !lit.is_empty() {
                    parts.push(FStrPart::Lit(std::mem::take(&mut lit)));
                }
                i += 1;
                let mut depth = 1;
                let mut expr_src = String::new();
                while i < chars.len() && depth > 0 {
                    match chars[i] {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    expr_src.push(chars[i]);
                    i += 1;
                }
                i += 1; // consume closing '}'
                let sub_toks = crate::lexer::Lexer::new(&expr_src, self.file)
                    .tokenize()
                    .map_err(|e| Box::new(OmniError::new(Phase::Parse, format!("in f-string expression: {}", e.message), Span { start, end: start }, self.file)))?;
                let mut sub_parser = Parser::new(sub_toks, self.file);
                let e = sub_parser.parse_expr()?;
                parts.push(FStrPart::Expr(e));
                continue;
            }
            lit.push(chars[i]);
            i += 1;
        }
        if !lit.is_empty() {
            parts.push(FStrPart::Lit(lit));
        }
        Ok(parts)
    }
}
