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
        // Structured form (omni-integration dialect): one or more of
        // preconditions/postconditions/invariants/assertions blocks, in any
        // order. Legacy form: a single boolean expression.
        if self.is_kw("preconditions") || self.is_kw("postconditions") || self.is_kw("invariants") || self.is_kw("assertions") {
            let mut preconditions = Vec::new();
            let mut postconditions = Vec::new();
            let mut named_invariants = Vec::new();
            let mut assertions = Vec::new();
            while !self.is_op("}") && !self.at_eof() {
                if self.eat_kw("preconditions") {
                    self.expect_op("{")?;
                    while !self.is_op("}") && !self.at_eof() {
                        let field = self.expect_ident()?;
                        self.expect_op(":")?;
                        let ty = self.parse_type_name()?;
                        preconditions.push((field, ty));
                        if !self.eat_op(",") {
                            break;
                        }
                    }
                    self.expect_op("}")?;
                } else if self.eat_kw("postconditions") {
                    self.expect_op("{")?;
                    postconditions = self.parse_stmts_until_close()?;
                    self.expect_op("}")?;
                } else if self.eat_kw("invariants") {
                    self.expect_op("{")?;
                    while !self.is_op("}") && !self.at_eof() {
                        let iname = self.expect_ident()?;
                        self.expect_op(":")?;
                        let body = self.parse_expr()?;
                        named_invariants.push((iname, body));
                    }
                    self.expect_op("}")?;
                } else if self.eat_kw("assertions") {
                    self.expect_op("{")?;
                    assertions = self.parse_stmts_until_close()?;
                    self.expect_op("}")?;
                } else {
                    return Err(self.err(format!(
                        "expected 'preconditions', 'postconditions', 'invariants', or 'assertions' but found '{}'",
                        self.cur_desc()
                    )));
                }
            }
            self.expect_op("}")?;
            return Ok(TheoremDef {
                name,
                foralls,
                body: TheoremBody::Structured { preconditions, postconditions, named_invariants, assertions },
                span: self.sp(start),
            });
        }
        let body = self.parse_expr()?;
        self.expect_op("}")?;
        Ok(TheoremDef { name, foralls, body: TheoremBody::Simple(body), span: self.sp(start) })
    }

    /// A type name for a `preconditions` field: `String`, `i64`, or a single
    /// level of generic, `Vec<u8>` (never enforced — Axiom's checker doesn't
    /// touch structured theorems at all; see `TheoremBody`).
    fn parse_type_name(&mut self) -> OResult<String> {
        let mut name = self.expect_ident()?;
        if self.eat_op("<") {
            let inner = self.expect_ident()?;
            // '<' / '>' aren't matched-pair tokens in this lexer (they're
            // comparison operators); a bare '>' following an identifier
            // closes the generic here.
            if self.is_op(">") {
                self.advance();
            } else if self.is_op(">=") {
                // ">=" is lexed greedily from '>' '='; if source had `<T>=`
                // rather than `<T> =`. Not expected in practice, but split
                // defensively so it doesn't swallow a real '=' downstream.
                self.advance();
            }
            name = format!("{name}<{inner}>");
        }
        Ok(name)
    }

    fn parse_stmts_until_close(&mut self) -> OResult<Vec<Stmt>> {
        let mut out = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            out.push(self.parse_theorem_stmt()?);
        }
        Ok(out)
    }

    fn parse_theorem_stmt(&mut self) -> OResult<Stmt> {
        if self.eat_kw("assert") {
            let e = self.parse_expr()?;
            return Ok(Stmt::Assert(e));
        }
        if self.eat_kw("let") {
            let name = self.expect_ident()?;
            self.expect_op("=")?;
            let value = self.parse_expr()?;
            return Ok(Stmt::Let { name, value });
        }
        if self.is_kw("if") {
            self.advance();
            let cond = self.parse_expr()?;
            self.expect_op("{")?;
            let body = self.parse_stmts_until_close()?;
            self.expect_op("}")?;
            return Ok(Stmt::If { cond, body });
        }
        // `forall` can appear as either a statement with a brace body
        // (`forall v in 0..n { stmts }`) or, just like in `invariants`
        // blocks, as a bare expression (`forall v in xs => expr`) used as a
        // statement on its own — both shapes occur in the omni-integration
        // specs' `postconditions`/`assertions` blocks.
        if self.is_kw("forall") {
            let (start, vars, collection, guard) = self.parse_forall_head()?;
            if self.eat_op("{") {
                let body = self.parse_stmts_until_close()?;
                self.expect_op("}")?;
                return Ok(Stmt::ForallStmt { vars, collection: collection.map(|c| *c), guard: guard.map(|g| *g), body });
            }
            self.expect_op("=>")?;
            let body = self.parse_expr()?;
            return Ok(Stmt::Expr(Expr::ForallIn { vars, collection, guard, body: Box::new(body), span: self.sp(start) }));
        }
        Ok(Stmt::Expr(self.parse_expr()?))
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
        // `forall`/`exists` as an *expression* (as opposed to a theorem/
        // invariant header's quantifier binding, parsed separately in
        // `parse_theorem`/`parse_invariant`) generalize to an arbitrary
        // collection expression, not just a literal integer range — used
        // inside `invariants { name: forall x in COLLECTION => ... }` blocks.
        if self.is_kw("forall") {
            return self.parse_forall_expr();
        }
        if self.is_kw("exists") {
            return self.parse_exists_expr();
        }
        self.parse_implies()
    }

    /// `forall v1, v2, .. [in collection] [where guard] => body`. `collection`
    /// may itself be a `lo..hi` range with expression bounds (see
    /// `parse_range_or_expr`).
    /// Parses `forall v1, v2, .. [in collection] [where guard]` — the part
    /// shared between the expression form (`=> body`, `parse_forall_expr`,
    /// used in `invariants` blocks) and the statement form (`{ stmts }`,
    /// used in `postconditions`/`assertions` blocks — see
    /// `parse_theorem_stmt`, which also allows the expression form there
    /// since both appear in the omni-integration specs).
    fn parse_forall_head(&mut self) -> OResult<(Pos, Vec<String>, Option<Box<Expr>>, Option<Box<Expr>>)> {
        let start = self.start();
        self.expect_kw("forall")?;
        let mut vars = vec![self.expect_ident()?];
        while self.eat_op(",") {
            vars.push(self.expect_ident()?);
        }
        let collection = if self.eat_kw("in") {
            Some(Box::new(self.parse_range_or_expr()?))
        } else {
            None
        };
        let guard = if self.eat_kw("where") {
            Some(Box::new(self.parse_or()?))
        } else {
            None
        };
        Ok((start, vars, collection, guard))
    }

    fn parse_forall_expr(&mut self) -> OResult<Expr> {
        let (start, vars, collection, guard) = self.parse_forall_head()?;
        self.expect_op("=>")?;
        let body = self.parse_expr()?; // right-assoc: allows nested forall/exists
        Ok(Expr::ForallIn { vars, collection, guard, body: Box::new(body), span: self.sp(start) })
    }

    /// Parses `expr` or `expr..expr` (a range with arbitrary, not necessarily
    /// literal-integer, bounds) — used for a `forall`/`exists` binding's
    /// `in <...>` clause.
    fn parse_range_or_expr(&mut self) -> OResult<Expr> {
        let start = self.start();
        let lo = self.parse_or()?;
        if self.eat_op("..") {
            let hi = self.parse_or()?;
            return Ok(Expr::Range { lo: Box::new(lo), hi: Box::new(hi), span: self.sp(start) });
        }
        Ok(lo)
    }

    fn parse_exists_expr(&mut self) -> OResult<Expr> {
        let start = self.start();
        self.expect_kw("exists")?;
        let var = self.expect_ident()?;
        self.expect_kw("in")?;
        let collection = self.parse_range_or_expr()?;
        self.expect_kw("where")?;
        let cond = self.parse_expr()?;
        Ok(Expr::ExistsIn { var, collection: Box::new(collection), cond: Box::new(cond), span: self.sp(start) })
    }

    fn parse_implies(&mut self) -> OResult<Expr> {
        let start = self.start();
        let left = self.parse_or()?;
        if self.eat_op("=>") {
            // `self.parse_expr()`, not `self.parse_implies()`: the RHS of
            // `=>` is a full expression position, so a nested `forall`/
            // `exists` there (e.g. `after(x) => exists e in Y where ...`)
            // must still be recognized — `parse_expr` is what checks for
            // those; `parse_implies` itself doesn't.
            let right = self.parse_expr()?; // right-associative, matching logical convention
            return Ok(Expr::BinOp { op: "=>".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) });
        }
        if self.eat_op("<=>") {
            let right = self.parse_expr()?;
            return Ok(Expr::BinOp { op: "<=>".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) });
        }
        Ok(left)
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

    /// `.name`, `.name(args)` (method call — Axiom doesn't distinguish field
    /// access from a call at parse time), and `[index]`, chained.
    fn parse_postfix(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut e = self.parse_atom()?;
        loop {
            if self.eat_op(".") {
                let name = self.expect_ident()?;
                let (args, has_parens) = if self.eat_op("(") {
                    let mut args = Vec::new();
                    while !self.is_op(")") && !self.at_eof() {
                        args.push(self.parse_expr()?);
                        if !self.eat_op(",") {
                            break;
                        }
                    }
                    self.expect_op(")")?;
                    (args, true)
                } else {
                    (Vec::new(), false)
                };
                e = Expr::MethodCall { obj: Box::new(e), name, args, has_parens, span: self.sp(start) };
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
        // `|params| body` — a closure literal (e.g. inside `.all(|c| ...)`).
        if self.eat_op("|") {
            let mut params = Vec::new();
            while !self.is_op("|") && !self.at_eof() {
                params.push(self.expect_ident()?);
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op("|")?;
            let body = self.parse_expr()?;
            return Ok(Expr::Closure { params, body: Box::new(body), span: self.sp(start) });
        }
        if self.eat_op("(") {
            let e = self.parse_expr()?;
            self.expect_op(")")?;
            return Ok(e);
        }
        Err(self.err(format!("expected an expression but found '{}'", self.cur_desc())))
    }
}
