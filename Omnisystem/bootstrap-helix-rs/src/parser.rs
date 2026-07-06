//! Helix parser — recursive descent, brace-delimited (Helix's uniqueness is
//! its type system/execution model, not its block punctuation — see
//! `ast.rs`/`interp.rs`).

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
        let mut fns = Vec::new();
        let mut kernels = Vec::new();
        let mut shaders = Vec::new();
        let mut script = Vec::new();
        while !self.at_eof() {
            if self.is_kw("fn") {
                fns.push(self.parse_fn()?);
            } else if self.is_kw("kernel") {
                kernels.push(self.parse_kernel()?);
            } else if self.is_kw("shader") {
                shaders.push(self.parse_shader()?);
            } else {
                script.push(self.parse_stmt()?);
            }
        }
        Ok(Module { fns, kernels, shaders, script })
    }

    fn parse_params(&mut self) -> OResult<Vec<String>> {
        self.expect_op("(")?;
        let mut params = Vec::new();
        while !self.is_op(")") && !self.at_eof() {
            params.push(self.expect_ident()?);
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        Ok(params)
    }

    fn parse_fn(&mut self) -> OResult<FnDef> {
        let start = self.start();
        self.expect_kw("fn")?;
        let name = self.expect_ident()?;
        let params = self.parse_params()?;
        self.expect_op("{")?;
        let body = self.parse_stmts_until_close()?;
        self.expect_op("}")?;
        Ok(FnDef { name, params, body, span: self.sp(start) })
    }

    fn parse_kernel(&mut self) -> OResult<KernelDef> {
        let start = self.start();
        self.expect_kw("kernel")?;
        let name = self.expect_ident()?;
        let params = self.parse_params()?;
        self.expect_op("{")?;
        let body = self.parse_stmts_until_close()?;
        self.expect_op("}")?;
        Ok(KernelDef { name, params, body, span: self.sp(start) })
    }

    fn parse_shader(&mut self) -> OResult<ShaderDef> {
        let start = self.start();
        self.expect_kw("shader")?;
        let stage = if self.eat_kw("vertex") {
            Stage::Vertex
        } else if self.eat_kw("fragment") {
            Stage::Fragment
        } else if self.eat_kw("compute") {
            Stage::Compute
        } else {
            return Err(self.err("expected 'vertex', 'fragment', or 'compute' after 'shader'"));
        };
        let name = self.expect_ident()?;
        let params = self.parse_params()?;
        self.expect_op("{")?;
        let body = self.parse_stmts_until_close()?;
        self.expect_op("}")?;
        Ok(ShaderDef { name, stage, params, body, span: self.sp(start) })
    }

    fn parse_stmts_until_close(&mut self) -> OResult<Vec<Stmt>> {
        let mut out = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            out.push(self.parse_stmt()?);
        }
        Ok(out)
    }

    fn parse_stmt(&mut self) -> OResult<Stmt> {
        let start = self.start();
        if self.eat_kw("let") {
            let name = self.expect_ident()?;
            self.expect_op("=")?;
            let value = self.parse_expr()?;
            return Ok(Stmt::Let { name, value, span: self.sp(start) });
        }
        if self.is_kw("if") {
            return self.parse_if();
        }
        if self.is_kw("for") {
            return self.parse_for();
        }
        if self.is_kw("return") {
            self.advance();
            let value = if self.is_op("}") { None } else { Some(self.parse_expr()?) };
            return Ok(Stmt::Return { value, span: self.sp(start) });
        }
        let e = self.parse_expr()?;
        if self.eat_op("=") {
            let value = self.parse_expr()?;
            return Ok(Stmt::Assign { target: e, value, span: self.sp(start) });
        }
        if self.eat_op("+=") {
            let rhs = self.parse_expr()?;
            let value = Expr::BinOp { op: "+".to_string(), left: Box::new(e.clone()), right: Box::new(rhs), span: self.sp(start) };
            return Ok(Stmt::Assign { target: e, value, span: self.sp(start) });
        }
        Ok(Stmt::Expr(e))
    }

    fn parse_if(&mut self) -> OResult<Stmt> {
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

    /// `for i in 0..N { ... }` — parsed generally (`hi` is any expression),
    /// but the *interpreter* rejects a non-literal `hi` with a clear error;
    /// see `ast.rs`'s doc comment for why that check belongs there.
    fn parse_for(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.expect_kw("for")?;
        let var = self.expect_ident()?;
        self.expect_kw("in")?;
        // Lower bound is always 0 in this bootstrap's range-for and is
        // simply discarded — parsed via `parse_atom` (no postfix chaining),
        // NOT `parse_expr`, because `parse_expr` would greedily consume the
        // first '.' of '..' as swizzle/attribute-access on the literal.
        let _lo = self.parse_atom()?;
        self.expect_op(".")?;
        self.expect_op(".")?;
        let hi = self.parse_expr()?;
        self.expect_op("{")?;
        let body = self.parse_stmts_until_close()?;
        self.expect_op("}")?;
        Ok(Stmt::For { var, hi, body, span: self.sp(start) })
    }

    // ── expressions ──────────────────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> OResult<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_and()?;
        while self.eat_op("||") {
            let right = self.parse_and()?;
            left = Expr::BinOp { op: "or".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_comparison()?;
        while self.eat_op("&&") {
            let right = self.parse_comparison()?;
            left = Expr::BinOp { op: "and".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
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
        if self.eat_op("!") {
            let e = self.parse_unary()?;
            return Ok(Expr::UnaryOp { op: "not".to_string(), expr: Box::new(e), span: self.sp(start) });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut e = self.parse_atom()?;
        loop {
            if self.eat_op(".") {
                let name = self.expect_ident()?;
                e = Expr::Attr { obj: Box::new(e), name, span: self.sp(start) };
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
                let v: f64 = t.value.trim_end_matches('f').parse().unwrap_or(0.0);
                return Ok(Expr::Float { v, span: self.sp(start) });
            }
            TokKind::Bool => {
                self.advance();
                return Ok(Expr::Bool { v: t.value == "true", span: self.sp(start) });
            }
            TokKind::Ident => {
                self.advance();
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
                    return Ok(Expr::Call { func: t.value, args, span: self.sp(start) });
                }
                return Ok(Expr::Ident { name: t.value, span: self.sp(start) });
            }
            _ => {}
        }
        if self.eat_op("(") {
            let e = self.parse_expr()?;
            self.expect_op(")")?;
            return Ok(e);
        }
        Err(self.err(format!("expected an expression but found '{}'", self.cur_desc())))
    }
}
