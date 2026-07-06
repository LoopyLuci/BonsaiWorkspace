//! Vera parser. The interesting part is `parse_node`: inside a `render { }`
//! block (or inside a tag's children, or inside a `{if}`/`{for}` control
//! block), the parser is in **markup context** — `<` always starts a tag,
//! never a less-than comparison — because the grammar only ever calls
//! `parse_node` from those specific positions, never as a general
//! expression. Expressions only reappear inside `{ }` interpolation braces
//! and attribute value braces, where `parse_expr` takes back over. This
//! avoids the classic JSX `<` ambiguity without any lexer-level tricks.

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
        let mut components = Vec::new();
        let mut script = Vec::new();
        while !self.at_eof() {
            if self.is_kw("component") {
                components.push(self.parse_component()?);
            } else {
                script.push(self.parse_stmt()?);
            }
        }
        Ok(Module { components, script })
    }

    fn parse_component(&mut self) -> OResult<ComponentDef> {
        let start = self.start();
        self.expect_kw("component")?;
        let name = self.expect_ident()?;
        self.expect_op("(")?;
        let mut props = Vec::new();
        while !self.is_op(")") && !self.at_eof() {
            props.push(self.expect_ident()?);
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        self.expect_op("{")?;
        let mut state = Vec::new();
        let mut computed = Vec::new();
        let mut methods = Vec::new();
        let mut render = Vec::new();
        let mut saw_render = false;
        while !self.is_op("}") && !self.at_eof() {
            if self.eat_kw("state") {
                let n = self.expect_ident()?;
                self.expect_op("=")?;
                let v = self.parse_expr()?;
                state.push((n, v));
            } else if self.eat_kw("computed") {
                let n = self.expect_ident()?;
                self.expect_op("=")?;
                let v = self.parse_expr()?;
                computed.push((n, v));
            } else if self.is_kw("fn") {
                methods.push(self.parse_fn_def()?);
            } else if self.cur().kind == TokKind::Ident && self.cur().value == "render" {
                self.advance();
                self.expect_op("{")?;
                render = self.parse_nodes_until_close()?;
                self.expect_op("}")?;
                saw_render = true;
            } else {
                return Err(self.err(format!("expected 'state'/'computed'/'fn'/'render' in component body, found '{}'", self.cur_desc())));
            }
        }
        self.expect_op("}")?;
        if !saw_render {
            return Err(self.err(format!("component '{name}' has no render block")));
        }
        Ok(ComponentDef { name, props, state, computed, methods, render, span: self.sp(start) })
    }

    fn parse_fn_def(&mut self) -> OResult<FnDef> {
        let start = self.start();
        self.expect_kw("fn")?;
        let name = self.expect_ident()?;
        self.expect_op("(")?;
        let mut params = Vec::new();
        while !self.is_op(")") && !self.at_eof() {
            params.push(self.expect_ident()?);
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        self.expect_op("{")?;
        let body = self.parse_stmts_until_close()?;
        self.expect_op("}")?;
        Ok(FnDef { name, params, body, span: self.sp(start) })
    }

    // ── statements ───────────────────────────────────────────────────────────

    fn parse_stmts_until_close(&mut self) -> OResult<Vec<Stmt>> {
        let mut out = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            out.push(self.parse_stmt()?);
        }
        Ok(out)
    }

    fn parse_stmt(&mut self) -> OResult<Stmt> {
        let start = self.start();
        if self.is_kw("if") {
            return self.parse_if_stmt();
        }
        if self.is_kw("return") {
            self.advance();
            let value = if self.is_op("}") { None } else { Some(self.parse_expr()?) };
            return Ok(Stmt::Return { value, span: self.sp(start) });
        }
        if self.cur().kind == TokKind::Ident && self.next_tok().kind == TokKind::Op && matches!(self.next_tok().value.as_str(), "=" | "+=" | "-=") {
            let name = self.advance().value;
            let aug = self.advance().value;
            let rhs = self.parse_expr()?;
            let value = if aug == "=" {
                rhs
            } else {
                let op = aug.trim_end_matches('=').to_string();
                Expr::BinOp { op, left: Box::new(Expr::Ident { name: name.clone(), span: self.sp(start) }), right: Box::new(rhs), span: self.sp(start) }
            };
            return Ok(Stmt::Assign { name, value, span: self.sp(start) });
        }
        Ok(Stmt::Expr(self.parse_expr()?))
    }

    fn parse_if_stmt(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.expect_kw("if")?;
        let cond = self.parse_expr()?;
        self.expect_op("{")?;
        let body = self.parse_stmts_until_close()?;
        self.expect_op("}")?;
        let mut orelse = Vec::new();
        if self.eat_kw("else") {
            self.expect_op("{")?;
            orelse = self.parse_stmts_until_close()?;
            self.expect_op("}")?;
        }
        Ok(Stmt::If { branches: vec![(cond, body)], orelse, span: self.sp(start) })
    }

    // ── expressions ──────────────────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> OResult<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_and()?;
        while self.eat_kw("or") || self.eat_op("||") {
            let right = self.parse_and()?;
            left = Expr::BinOp { op: "or".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_not()?;
        while self.eat_kw("and") || self.eat_op("&&") {
            let right = self.parse_not()?;
            left = Expr::BinOp { op: "and".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> OResult<Expr> {
        let start = self.start();
        if self.eat_kw("not") || self.eat_op("!") {
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
        let mut e = self.parse_atom()?;
        loop {
            if self.is_op("(") {
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
            } else if self.eat_op(".") {
                let name = self.expect_ident()?;
                e = Expr::Attr { obj: Box::new(e), name, span: self.sp(start) };
            } else {
                break;
            }
        }
        Ok(e)
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
            TokKind::Bool => {
                self.advance();
                return Ok(Expr::Bool { v: t.value == "true", span: self.sp(start) });
            }
            TokKind::Ident => {
                self.advance();
                return Ok(Expr::Ident { name: t.value, span: self.sp(start) });
            }
            _ => {}
        }
        // `fn(params) { stmt* }` or `|params| expr` — event-handler closures.
        if self.is_kw("fn") {
            self.advance();
            self.expect_op("(")?;
            let mut params = Vec::new();
            while !self.is_op(")") && !self.at_eof() {
                params.push(self.expect_ident()?);
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op(")")?;
            self.expect_op("{")?;
            let body = self.parse_stmts_until_close()?;
            self.expect_op("}")?;
            return Ok(Expr::Lambda { params, body, span: self.sp(start) });
        }
        if self.eat_op("||") {
            // `|| expr` — zero-arg shorthand handler (common for onClick).
            let e = self.parse_expr()?;
            return Ok(Expr::Lambda { params: vec![], body: vec![Stmt::Expr(e)], span: self.sp(start) });
        }
        if self.is_op("|") {
            self.advance();
            let mut params = Vec::new();
            while !self.is_op("|") && !self.at_eof() {
                params.push(self.expect_ident()?);
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op("|")?;
            let e = self.parse_expr()?;
            return Ok(Expr::Lambda { params, body: vec![Stmt::Expr(e)], span: self.sp(start) });
        }
        if self.eat_op("(") {
            let e = self.parse_expr()?;
            self.expect_op(")")?;
            return Ok(e);
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
        Err(self.err(format!("expected an expression but found '{}'", self.cur_desc())))
    }

    // ── markup / nodes ─────────────────────────────────────────────────────────

    fn parse_nodes_until_close(&mut self) -> OResult<Vec<Node>> {
        let mut out = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            out.push(self.parse_node()?);
        }
        Ok(out)
    }

    /// The markup grammar. Reached only from `render { }`, a tag's own
    /// children, or inside a `{if}`/`{for}` control block's body — never
    /// from general expression context, so `<` is unambiguous here.
    fn parse_node(&mut self) -> OResult<Node> {
        if self.is_op("<") {
            return self.parse_element();
        }
        if self.cur().kind == TokKind::Str {
            let t = self.advance();
            return Ok(Node::Text(t.value));
        }
        if self.eat_op("{") {
            if self.is_kw("if") {
                return self.parse_node_if();
            }
            if self.is_kw("for") {
                return self.parse_node_for();
            }
            let e = self.parse_expr()?;
            self.expect_op("}")?;
            return Ok(Node::Expr(e));
        }
        Err(self.err(format!("expected a markup node (tag, string, or {{expr}}) but found '{}'", self.cur_desc())))
    }

    fn parse_element(&mut self) -> OResult<Node> {
        let start = self.start();
        self.expect_op("<")?;
        let tag = self.expect_ident()?;
        let mut attrs = Vec::new();
        while self.cur().kind == TokKind::Ident {
            let aname = self.advance().value;
            let aval = if self.eat_op("=") {
                if self.eat_op("{") {
                    let e = self.parse_expr()?;
                    self.expect_op("}")?;
                    e
                } else if self.cur().kind == TokKind::Str {
                    let t = self.advance();
                    Expr::Str { v: t.value, span: self.sp(start) }
                } else {
                    return Err(self.err("expected '{expr}' or a string literal as attribute value"));
                }
            } else {
                Expr::Bool { v: true, span: self.sp(start) } // boolean shorthand: `disabled` == `disabled={true}`
            };
            attrs.push((aname, aval));
        }
        if self.eat_op("/>") {
            return Ok(Node::Element { tag, attrs, children: vec![], span: self.sp(start) });
        }
        self.expect_op(">")?;
        let children = self.parse_nodes_until_close_tag()?;
        self.expect_op("<")?;
        self.expect_op("/")?;
        let close_tag = self.expect_ident()?;
        if close_tag != tag {
            return Err(self.err(format!("mismatched closing tag: expected '</{tag}>' but found '</{close_tag}>'")));
        }
        self.expect_op(">")?;
        Ok(Node::Element { tag, attrs, children, span: self.sp(start) })
    }

    /// Like `parse_nodes_until_close` but stops at a closing tag's `</`
    /// instead of `}` (tag children vs. a `render`/control-block body).
    fn parse_nodes_until_close_tag(&mut self) -> OResult<Vec<Node>> {
        let mut out = Vec::new();
        while !self.at_eof() && !(self.is_op("<") && self.next_tok().kind == TokKind::Op && self.next_tok().value == "/") {
            out.push(self.parse_node()?);
        }
        Ok(out)
    }

    fn parse_node_if(&mut self) -> OResult<Node> {
        self.expect_kw("if")?;
        let cond = self.parse_expr()?;
        self.expect_op("{")?;
        let then_branch = self.parse_nodes_until_close()?;
        self.expect_op("}")?;
        let mut else_branch = Vec::new();
        if self.eat_kw("else") {
            self.expect_op("{")?;
            else_branch = self.parse_nodes_until_close()?;
            self.expect_op("}")?;
        }
        self.expect_op("}")?; // closes the outer `{if ...}` wrapper
        Ok(Node::If { cond, then_branch, else_branch })
    }

    fn parse_node_for(&mut self) -> OResult<Node> {
        self.expect_kw("for")?;
        let var = self.expect_ident()?;
        self.expect_kw("in")?;
        let iter = self.parse_expr()?;
        self.expect_op("{")?;
        let body = self.parse_nodes_until_close()?;
        self.expect_op("}")?;
        self.expect_op("}")?; // closes the outer `{for ...}` wrapper
        Ok(Node::For { var, iter, body })
    }
}
