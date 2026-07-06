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
}

impl<'a> Parser<'a> {
    pub fn new(toks: Vec<Token>, file: &'a str) -> Self {
        Parser { toks, p: 0, file }
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

    // ── expressions (precedence climbing) ────────────────────────────────

    pub fn parse_expr(&mut self) -> OResult<Expr> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> OResult<Expr> {
        let start = self.start();
        let body = self.parse_lambda()?;
        if self.eat_kw("if") {
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
        self.parse_or()
    }

    fn parse_or(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_and()?;
        while self.eat_kw("or") {
            let right = self.parse_and()?;
            left = Expr::BoolOp { op: "or".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_not()?;
        while self.eat_kw("and") {
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
        // `|`/`&` reserved for future set-ops; treated at same tier as add
        // for now (no real bitwise-vs-logical ambiguity in this subset).
        self.parse_add()
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
            if self.eat_op(".") {
                let name = self.expect_ident()?;
                e = Expr::Attr { obj: Box::new(e), name, span: self.sp(start) };
            } else if self.is_op("(") {
                self.advance();
                let (args, kwargs) = self.parse_call_args()?;
                self.expect_op(")")?;
                e = Expr::Call { func: Box::new(e), args, kwargs, span: self.sp(start) };
            } else if self.eat_op("[") {
                e = self.parse_subscript(e, start)?;
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_subscript(&mut self, obj: Expr, start: Pos) -> OResult<Expr> {
        // Distinguish `x[i]` from `x[lo:hi]` / `x[lo:hi:step]`.
        let lo = if self.is_op(":") { None } else { Some(Box::new(self.parse_expr()?)) };
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
                return Ok(Expr::Ident { name: t.value, span: self.sp(start) });
            }
            TokKind::Keyword if t.value == "self" => {
                self.advance();
                return Ok(Expr::Ident { name: "self".to_string(), span: self.sp(start) });
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
