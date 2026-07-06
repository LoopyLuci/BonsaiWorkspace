//! Aether parser — recursive descent, `do`/`end`-block-terminated (no
//! indentation tracking needed, unlike Sylva; no braces, unlike Titan).
//! Function clauses sharing a name are *not* merged here — the interpreter
//! groups them at registration time — so this stays a straightforward
//! single-pass parse.

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
    fn next_tok(&self) -> &Token {
        &self.toks[(self.p + 1).min(self.toks.len() - 1)]
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
        if self.cur().kind == TokKind::Eof { "end of file".to_string() } else { self.cur().value.clone() }
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

    pub fn parse_module(&mut self) -> OResult<Module> {
        if self.is_kw("defmodule") {
            self.advance();
            let name = self.expect_ident()?;
            self.expect_kw("do")?;
            let items = self.parse_items_until_end()?;
            self.expect_kw("end")?;
            return Ok(Module { name: Some(name), items });
        }
        let mut items = Vec::new();
        while !self.at_eof() {
            items.push(self.parse_item()?);
        }
        Ok(Module { name: None, items })
    }

    fn parse_items_until_end(&mut self) -> OResult<Vec<Item>> {
        let mut items = Vec::new();
        while !self.is_kw("end") && !self.at_eof() {
            items.push(self.parse_item()?);
        }
        Ok(items)
    }

    fn parse_item(&mut self) -> OResult<Item> {
        if self.is_kw("def") || self.is_kw("defp") {
            return Ok(Item::FnClause(self.parse_fn_clause()?));
        }
        if self.is_kw("actor") {
            return Ok(Item::ActorDef(self.parse_actor()?));
        }
        Ok(Item::TopStmt(self.parse_stmt()?))
    }

    fn parse_fn_clause(&mut self) -> OResult<FnClause> {
        let start = self.start();
        let is_private = self.is_kw("defp");
        self.advance(); // def/defp
        let name = self.expect_ident()?;
        self.expect_op("(")?;
        let mut params = Vec::new();
        while !self.is_op(")") && !self.at_eof() {
            params.push(self.parse_pattern()?);
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        let guard = if self.eat_kw("when") { Some(self.parse_expr()?) } else { None };
        self.expect_kw("do")?;
        let body = self.parse_stmts_until(&["end"])?;
        self.expect_kw("end")?;
        Ok(FnClause { name, params, guard, body, is_private, span: self.sp(start) })
    }

    fn parse_actor(&mut self) -> OResult<ActorDef> {
        let start = self.start();
        self.expect_kw("actor")?;
        let name = self.expect_ident()?;
        self.expect_kw("do")?;
        let mut fns = Vec::new();
        let mut receives = Vec::new();
        while !self.is_kw("end") && !self.at_eof() {
            if self.is_kw("def") || self.is_kw("defp") {
                fns.push(self.parse_fn_clause()?);
                continue;
            }
            if self.is_kw("receive") {
                receives.push(self.parse_receive_clause()?);
                continue;
            }
            return Err(self.err(format!("expected 'def' or 'receive' inside actor body, found '{}'", self.cur_desc())));
        }
        self.expect_kw("end")?;
        Ok(ActorDef { name, fns, receives, span: self.sp(start) })
    }

    fn parse_receive_clause(&mut self) -> OResult<ReceiveClause> {
        self.advance(); // 'receive' keyword
        let msg_pattern = self.parse_pattern()?;
        self.expect_op(",")?;
        let state_binding = self.expect_ident()?;
        let guard = if self.eat_kw("when") { Some(self.parse_expr()?) } else { None };
        self.expect_kw("do")?;
        let body = self.parse_stmts_until(&["end"])?;
        self.expect_kw("end")?;
        Ok(ReceiveClause { msg_pattern, state_binding, guard, body })
    }

    // ── patterns ─────────────────────────────────────────────────────────────

    fn parse_pattern(&mut self) -> OResult<Pattern> {
        if self.cur().kind == TokKind::Ident && self.cur().value == "_" {
            self.advance();
            return Ok(Pattern::Wild);
        }
        if self.cur().kind == TokKind::Atom {
            let t = self.advance();
            return Ok(Pattern::Atom(t.value));
        }
        if matches!(self.cur().kind, TokKind::Int | TokKind::Float | TokKind::Str | TokKind::Bool | TokKind::Nil) {
            let e = self.parse_atom_expr()?;
            return Ok(Pattern::Lit(e));
        }
        if self.eat_op("{") {
            let mut elems = Vec::new();
            while !self.is_op("}") && !self.at_eof() {
                elems.push(self.parse_pattern()?);
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op("}")?;
            return Ok(Pattern::Tuple(elems));
        }
        if self.eat_op("[") {
            if self.eat_op("]") {
                return Ok(Pattern::List(vec![]));
            }
            let mut elems = vec![self.parse_pattern()?];
            while self.eat_op(",") {
                elems.push(self.parse_pattern()?);
            }
            if self.eat_op("|") {
                let tail = self.parse_pattern()?;
                self.expect_op("]")?;
                // Build nested cons cells right-to-left: [a, b | t] => Cons(a, Cons(b, t))
                let mut result = tail;
                for elem in elems.into_iter().rev() {
                    result = Pattern::Cons(Box::new(elem), Box::new(result));
                }
                return Ok(result);
            }
            self.expect_op("]")?;
            return Ok(Pattern::List(elems));
        }
        if self.cur().kind == TokKind::Ident {
            let name = self.advance().value;
            return Ok(Pattern::Bind(name));
        }
        Err(self.err(format!("expected a pattern but found '{}'", self.cur_desc())))
    }

    // ── statements ───────────────────────────────────────────────────────────

    /// Parses statements until the current token is one of `stops` (checked
    /// as a keyword) — the caller consumes the stop token itself.
    fn parse_stmts_until(&mut self, stops: &[&str]) -> OResult<Vec<Stmt>> {
        let mut out = Vec::new();
        while !self.at_eof() && !stops.iter().any(|s| self.is_kw(s)) {
            out.push(self.parse_stmt()?);
        }
        Ok(out)
    }

    fn parse_stmt(&mut self) -> OResult<Stmt> {
        let start = self.start();
        if self.is_kw("if") {
            return self.parse_if();
        }
        if self.is_kw("case") {
            return self.parse_case();
        }
        if self.is_kw("for") {
            return self.parse_for();
        }
        if self.is_kw("return") {
            self.advance();
            let value = if self.is_kw("end") || self.at_eof() { None } else { Some(self.parse_expr()?) };
            return Ok(Stmt::Return { value, span: self.sp(start) });
        }
        // Assignment: `ident = expr` (lookahead so we don't misparse `==`).
        if self.cur().kind == TokKind::Ident && self.next_tok().kind == TokKind::Op && self.next_tok().value == "=" {
            let name = self.advance().value;
            self.advance(); // '='
            let value = self.parse_expr()?;
            return Ok(Stmt::Assign { name, value, span: self.sp(start) });
        }
        Ok(Stmt::Expr(self.parse_expr()?))
    }

    fn parse_if(&mut self) -> OResult<Stmt> {
        let start = self.start();
        let (branches, orelse) = self.parse_if_body()?;
        Ok(Stmt::If { branches, orelse, span: self.sp(start) })
    }

    fn parse_if_body(&mut self) -> OResult<(Vec<(Expr, Vec<Stmt>)>, Vec<Stmt>)> {
        self.expect_kw("if")?;
        let cond = self.parse_expr()?;
        self.expect_kw("do")?;
        let body = self.parse_stmts_until(&["else", "end"])?;
        let mut orelse = Vec::new();
        if self.eat_kw("else") {
            orelse = self.parse_stmts_until(&["end"])?;
        }
        self.expect_kw("end")?;
        Ok((vec![(cond, body)], orelse))
    }

    fn parse_case(&mut self) -> OResult<Stmt> {
        let start = self.start();
        let (scrut, arms) = self.parse_case_body()?;
        Ok(Stmt::Case { scrut, arms, span: self.sp(start) })
    }

    fn parse_case_body(&mut self) -> OResult<(Expr, Vec<CaseArm>)> {
        self.advance(); // 'case'
        let scrut = self.parse_expr()?;
        self.expect_kw("do")?;
        let mut arms = Vec::new();
        while !self.is_kw("end") && !self.at_eof() {
            let pattern = self.parse_pattern()?;
            let guard = if self.eat_kw("when") { Some(self.parse_expr()?) } else { None };
            self.expect_op("->")?;
            let body = if self.is_kw("do") {
                self.advance();
                let b = self.parse_stmts_until(&["end"])?;
                self.expect_kw("end")?;
                b
            } else {
                vec![Stmt::Expr(self.parse_expr()?)]
            };
            arms.push(CaseArm { pattern, guard, body });
        }
        self.expect_kw("end")?;
        Ok((scrut, arms))
    }

    fn parse_for(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.expect_kw("for")?;
        let var = self.expect_ident()?;
        self.expect_kw("in")?;
        let iter = self.parse_expr()?;
        self.expect_kw("do")?;
        let body = self.parse_stmts_until(&["end"])?;
        self.expect_kw("end")?;
        Ok(Stmt::For { var, iter, body, span: self.sp(start) })
    }

    // ── expressions ──────────────────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> OResult<Expr> {
        self.parse_pipe()
    }

    /// `lhs |> rhs(args)` desugars into `rhs(lhs, args)` — Elixir's pipe.
    fn parse_pipe(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_or()?;
        while self.eat_op("|>") {
            let rhs = self.parse_or()?;
            left = match rhs {
                Expr::Call { func, mut args, span } => {
                    args.insert(0, left);
                    Expr::Call { func, args, span }
                }
                other => Expr::Call { func: Box::new(other), args: vec![left], span: self.sp(start) },
            };
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_and()?;
        while self.eat_kw("or") {
            let right = self.parse_and()?;
            left = Expr::BinOp { op: "or".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_not()?;
        while self.eat_kw("and") {
            let right = self.parse_not()?;
            left = Expr::BinOp { op: "and".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> OResult<Expr> {
        let start = self.start();
        if self.eat_kw("not") {
            let e = self.parse_not()?;
            return Ok(Expr::UnaryOp { op: "not".to_string(), expr: Box::new(e), span: self.sp(start) });
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> OResult<Expr> {
        let start = self.start();
        let left = self.parse_add()?;
        for op in ["==", "!=", "<=", ">=", "<", ">"] {
            if self.is_op(op) {
                self.advance();
                let right = self.parse_add()?;
                return Ok(Expr::BinOp { op: op.to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) });
            }
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
        if self.is_op("-") {
            self.advance();
            let e = self.parse_unary()?;
            return Ok(Expr::UnaryOp { op: "-".to_string(), expr: Box::new(e), span: self.sp(start) });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut e = self.parse_atom_expr()?;
        loop {
            if self.eat_op(".") {
                let name = self.expect_ident()?;
                e = Expr::Attr { obj: Box::new(e), name, span: self.sp(start) };
            } else if self.is_op("(") {
                self.advance();
                let mut args = Vec::new();
                while !self.is_op(")") && !self.at_eof() {
                    args.push(self.parse_expr()?);
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op(")")?;
                e = Expr::Call { func: Box::new(e), args, span: self.sp(start) };
            } else if self.eat_op("[") {
                let idx = self.parse_expr()?;
                self.expect_op("]")?;
                e = Expr::Index { obj: Box::new(e), index: Box::new(idx), span: self.sp(start) };
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_atom_expr(&mut self) -> OResult<Expr> {
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
            TokKind::IStr => {
                self.advance();
                let parts = self.parse_istr_parts(&t.value, start)?;
                return Ok(Expr::IStr { parts, span: self.sp(start) });
            }
            TokKind::Bool => {
                self.advance();
                return Ok(Expr::Bool { v: t.value == "true", span: self.sp(start) });
            }
            TokKind::Nil => {
                self.advance();
                return Ok(Expr::Nil { span: self.sp(start) });
            }
            TokKind::Atom => {
                self.advance();
                return Ok(Expr::Atom { name: t.value, span: self.sp(start) });
            }
            TokKind::Ident => {
                self.advance();
                return Ok(Expr::Ident { name: t.value, span: self.sp(start) });
            }
            _ => {}
        }
        if self.is_kw("case") {
            let (scrut, arms) = self.parse_case_body()?;
            return Ok(Expr::Case { scrut: Box::new(scrut), arms, span: self.sp(start) });
        }
        if self.is_kw("if") {
            let (branches, orelse) = self.parse_if_body()?;
            return Ok(Expr::If { branches, orelse, span: self.sp(start) });
        }
        if self.is_kw("spawn") {
            self.advance();
            let actor = self.expect_ident()?;
            self.expect_op("(")?;
            let mut args = Vec::new();
            while !self.is_op(")") && !self.at_eof() {
                args.push(self.parse_expr()?);
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op(")")?;
            return Ok(Expr::Spawn { actor, args, span: self.sp(start) });
        }
        if self.cur().kind == TokKind::Ident && self.cur().value == "fn" {
            return self.parse_lambda();
        }
        if self.eat_op("(") {
            let e = self.parse_expr()?;
            self.expect_op(")")?;
            return Ok(e);
        }
        if self.eat_op("{") {
            let mut elems = Vec::new();
            while !self.is_op("}") && !self.at_eof() {
                elems.push(self.parse_expr()?);
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op("}")?;
            return Ok(Expr::Tuple { elems, span: self.sp(start) });
        }
        if self.eat_op("[") {
            let mut elems = Vec::new();
            while !self.is_op("]") && !self.at_eof() {
                elems.push(self.parse_expr()?);
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op("]")?;
            return Ok(Expr::List { elems, span: self.sp(start) });
        }
        if self.is_op("%") && self.next_tok().kind == TokKind::Op && self.next_tok().value == "{" {
            self.advance(); // '%'
            self.advance(); // '{'
            let mut entries = Vec::new();
            while !self.is_op("}") && !self.at_eof() {
                // `name: value` shorthand (Elixir keyword-list-style map key):
                // a bare identifier immediately followed by `:` is sugar for
                // an atom key — `name` here is NOT a variable reference.
                let key = if self.cur().kind == TokKind::Ident && self.next_tok().kind == TokKind::Op && self.next_tok().value == ":" {
                    let name = self.advance().value;
                    self.advance(); // ':'
                    Expr::Atom { name, span: self.sp(start) }
                } else if self.cur().kind == TokKind::Atom {
                    let a = self.advance().value;
                    self.expect_op(":")?;
                    Expr::Atom { name: a, span: self.sp(start) }
                } else {
                    let k = self.parse_expr()?;
                    self.expect_op(":")?;
                    k
                };
                let val = self.parse_expr()?;
                entries.push((key, val));
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op("}")?;
            return Ok(Expr::Map { entries, span: self.sp(start) });
        }
        Err(self.err(format!("expected an expression but found '{}'", self.cur_desc())))
    }

    fn parse_lambda(&mut self) -> OResult<Expr> {
        let start = self.start();
        self.advance(); // 'fn'
        let mut params = Vec::new();
        let has_parens = self.eat_op("(");
        while !self.is_op("->") && !self.at_eof() {
            params.push(self.parse_pattern()?);
            if !self.eat_op(",") {
                break;
            }
        }
        if has_parens {
            self.expect_op(")")?;
        }
        self.expect_op("->")?;
        let body = self.parse_expr()?;
        self.expect_kw("end")?;
        Ok(Expr::Lambda { params, body: Box::new(body), span: self.sp(start) })
    }

    /// Splits an interpolated string's raw text into literal/`#{expr}` parts.
    fn parse_istr_parts(&mut self, raw: &str, start: Pos) -> OResult<Vec<IStrPart>> {
        let mut parts = Vec::new();
        let chars: Vec<char> = raw.chars().collect();
        let mut i = 0;
        let mut lit = String::new();
        while i < chars.len() {
            if chars[i] == '#' && chars.get(i + 1) == Some(&'{') {
                if !lit.is_empty() {
                    parts.push(IStrPart::Lit(std::mem::take(&mut lit)));
                }
                i += 2;
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
                i += 1; // closing '}'
                let sub_toks = crate::lexer::Lexer::new(&expr_src, self.file)
                    .tokenize()
                    .map_err(|e| Box::new(OmniError::new(Phase::Parse, format!("in string interpolation: {}", e.message), Span { start, end: start }, self.file)))?;
                let mut sub_parser = Parser::new(sub_toks, self.file);
                parts.push(IStrPart::Expr(sub_parser.parse_expr()?));
                continue;
            }
            lit.push(chars[i]);
            i += 1;
        }
        if !lit.is_empty() {
            parts.push(IStrPart::Lit(lit));
        }
        Ok(parts)
    }
}
