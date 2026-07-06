//! Axiom parser — recursive descent. Quantifier and state-literal bounds
//! are parsed via a dedicated signed-integer-literal path (`parse_int_lit`),
//! never the general expression parser — Axiom requires explicit finite
//! bounds to verify over, so there's no "computed bound" case to support,
//! and this sidesteps the `0..n` postfix-ambiguity class of bug entirely
//! (unlike Helix, `..` is its own single token here — see `lexer.rs`).

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
    /// Handles an optional leading `-` itself: negative literals (`-5`)
    /// always lex as two tokens (`Op("-")`, `Int("5")`) — the lexer never
    /// merges them (see `lexer.rs::prev_allows_unary`'s honest note that
    /// it's a documented no-op, not the "dedicated signed-int path" an
    /// earlier draft of that comment incorrectly implied).
    fn expect_int_lit(&mut self) -> OResult<i64> {
        let neg = self.eat_op("-");
        if self.cur().kind == TokKind::Int {
            let t = self.advance();
            let n: i64 = t.value.parse().unwrap_or(0);
            Ok(if neg { -n } else { n })
        } else {
            Err(self.err(format!("expected an integer literal but found '{}'", self.cur_desc())))
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
        let mut axioms = Vec::new();
        let mut theorems = Vec::new();
        let mut invariants = Vec::new();
        while !self.at_eof() {
            if self.is_kw("axiom") {
                axioms.push(self.parse_axiom()?);
            } else if self.is_kw("theorem") {
                theorems.push(self.parse_theorem()?);
            } else if self.is_kw("invariant") {
                invariants.push(self.parse_invariant()?);
            } else {
                return Err(self.err(format!("expected 'axiom', 'theorem', or 'invariant' but found '{}'", self.cur_desc())));
            }
        }
        Ok(Module { axioms, theorems, invariants })
    }

    fn parse_axiom(&mut self) -> OResult<AxiomDef> {
        let start = self.start();
        self.expect_kw("axiom")?;
        let name = self.expect_ident()?;
        self.expect_op("{")?;
        let body = self.parse_expr()?;
        self.expect_op("}")?;
        Ok(AxiomDef { name, body, span: self.sp(start) })
    }

    fn parse_theorem(&mut self) -> OResult<TheoremDef> {
        let start = self.start();
        self.expect_kw("theorem")?;
        let name = self.expect_ident()?;
        let mut foralls = Vec::new();
        if self.eat_kw("forall") {
            loop {
                let var = self.expect_ident()?;
                self.expect_kw("in")?;
                let lo = self.expect_int_lit()?;
                self.expect_op("..")?;
                let hi = self.expect_int_lit()?;
                foralls.push(QuantBinding { var, lo, hi });
                if !self.eat_op(",") {
                    break;
                }
            }
        }
        self.expect_op("{")?;
        let body = self.parse_expr()?;
        self.expect_op("}")?;
        Ok(TheoremDef { name, foralls, body, span: self.sp(start) })
    }

    fn parse_invariant(&mut self) -> OResult<InvariantDef> {
        let start = self.start();
        self.expect_kw("invariant")?;
        let name = self.expect_ident()?;
        self.expect_kw("over")?;
        self.expect_kw("states")?;
        self.expect_op("(")?; // uses parens to keep the state-list visually distinct from a body block
        let mut states = Vec::new();
        while !self.is_op(")") && !self.at_eof() {
            states.push(self.parse_state_lit()?);
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        self.expect_op("{")?;
        let body = self.parse_expr()?;
        self.expect_op("}")?;
        Ok(InvariantDef { name, states, body, span: self.sp(start) })
    }

    fn parse_state_lit(&mut self) -> OResult<StateLit> {
        self.expect_op("{")?;
        let mut fields = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            let name = self.expect_ident()?;
            self.expect_op(":")?;
            let v = self.expect_int_lit()?;
            fields.push((name, v));
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op("}")?;
        Ok(fields)
    }

    // ── expressions ──────────────────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> OResult<Expr> {
        self.parse_implies()
    }

    fn parse_implies(&mut self) -> OResult<Expr> {
        let start = self.start();
        let left = self.parse_or()?;
        if self.eat_op("=>") {
            let right = self.parse_implies()?; // right-associative, matching logical convention
            return Ok(Expr::BinOp { op: "=>".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) });
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
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> OResult<Expr> {
        let start = self.start();
        let t = self.cur().clone();
        match t.kind {
            TokKind::Int => {
                self.advance();
                return Ok(Expr::Int { v: t.value.parse().unwrap_or(0), span: self.sp(start) });
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
        if self.eat_op("(") {
            let e = self.parse_expr()?;
            self.expect_op(")")?;
            return Ok(e);
        }
        Err(self.err(format!("expected an expression but found '{}'", self.cur_desc())))
    }
}
