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
    /// Disabled while parsing an `if`/`case`/`for`/`match` condition-or-
    /// subject expression, so it doesn't swallow the construct's own
    /// block-opening `{` as a struct-literal start — see
    /// `parse_atom_expr`'s `TokKind::Ident` case.
    allow_struct_lit: bool,
}

impl<'a> Parser<'a> {
    pub fn new(toks: Vec<Token>, file: &'a str) -> Self {
        Parser { toks, p: 0, file, allow_struct_lit: true }
    }

    /// `parse_expr`, but with struct-literal parsing disabled for the
    /// duration of the call (save/restore, since this nests — a condition
    /// can contain a closure whose body legitimately allows struct
    /// literals). See `allow_struct_lit`'s doc comment.
    fn parse_cond_expr(&mut self) -> OResult<Expr> {
        let prev = self.allow_struct_lit;
        self.allow_struct_lit = false;
        let e = self.parse_expr();
        self.allow_struct_lit = prev;
        e
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
        } else if self.cur().kind == TokKind::Keyword {
            // Several spec files use words this bootstrap reserves as
            // native keywords (`after`, `mut`, ...) as plain Rust-dialect
            // variable/field names. Every native-syntax use of a keyword is
            // matched via `is_kw`/`eat_kw` at its own dedicated call site
            // before falling through here, so accepting any keyword as an
            // identifier at an identifier-required position is safe.
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
        // Rust-syntax dialect item (see `parse_rust_fn`/`parse_use`'s doc
        // comments): `use`, `struct`/`pub struct` (skipped — no place in
        // this AST for a flat data-record type), `impl Name { fn .. }`
        // (methods folded into an `ActorDef` so at least their bodies are
        // validated), free `[pub] fn` (also Rust-shaped: typed params,
        // `-> Type`, brace body).
        let start = self.start();
        if self.is_kw("use") {
            self.parse_use()?;
            return Ok(Item::TopStmt(Stmt::Expr(Expr::Nil { span: self.sp(start) })));
        }
        let after_pub = if self.is_kw("pub") { Some(self.next_tok().value.clone()) } else { None };
        if self.is_kw("struct") || after_pub.as_deref() == Some("struct") {
            self.eat_kw("pub");
            self.skip_struct_or_enum()?;
            return Ok(Item::TopStmt(Stmt::Expr(Expr::Nil { span: self.sp(start) })));
        }
        if self.is_kw("impl") {
            return Ok(Item::ActorDef(self.parse_rust_impl()?));
        }
        // `pub actor Name { field: Type, .. }` (Rust-shaped fields, no
        // `do`/`end`) — the omni-integration dialect's actor-as-struct
        // shape. Fields are skipped (this bootstrap's `ActorDef` has no
        // field-storage model beyond what `impl` methods build up via
        // `self.x = ..` assignments); a same-named `impl` block elsewhere
        // supplies the real methods as a separate `ActorDef`.
        if self.is_kw("actor") && self.next_tok().kind == TokKind::Ident {
            let save = self.p;
            self.advance(); // 'actor'
            let name = self.expect_ident()?;
            if self.is_op("{") {
                self.advance();
                let mut depth = 1i32;
                while depth > 0 && !self.at_eof() {
                    if self.is_op("{") {
                        depth += 1;
                    } else if self.is_op("}") {
                        depth -= 1;
                    }
                    self.advance();
                }
                return Ok(Item::ActorDef(ActorDef { name, fns: Vec::new(), receives: Vec::new(), span: self.sp(start) }));
            }
            self.p = save; // not the Rust-shaped form — fall through to native `actor .. do .. end`
        }
        if after_pub.as_deref() == Some("actor") {
            self.eat_kw("pub");
            self.advance(); // 'actor'
            let name = self.expect_ident()?;
            self.expect_op("{")?;
            let mut depth = 1i32;
            while depth > 0 && !self.at_eof() {
                if self.is_op("{") {
                    depth += 1;
                } else if self.is_op("}") {
                    depth -= 1;
                }
                self.advance();
            }
            return Ok(Item::ActorDef(ActorDef { name, fns: Vec::new(), receives: Vec::new(), span: self.sp(start) }));
        }
        if self.is_kw("actor") {
            return Ok(Item::ActorDef(self.parse_actor()?));
        }
        if self.is_kw("fn") || after_pub.as_deref() == Some("fn") {
            return Ok(Item::FnClause(self.parse_rust_fn()?));
        }
        Ok(Item::TopStmt(self.parse_stmt()?))
    }

    /// `use path::{a, b, c}` / `use path::item` — parsed and entirely
    /// discarded (this bootstrap has no cross-file module resolution).
    fn parse_use(&mut self) -> OResult<()> {
        self.advance(); // "use"
        self.expect_ident()?;
        loop {
            // `::{` grouped import, or `::item` / `::Item` single import —
            // the second `:` combines with the following identifier into
            // one `Atom` token (see `lexer.rs::lex_atom`), same quirk
            // documented in `parse_atom_expr`'s path-folding.
            if self.is_op(":") && self.next_tok().kind == TokKind::Op && self.next_tok().value == ":" {
                self.advance();
                self.advance();
            } else if self.is_op(":") && self.next_tok().kind == TokKind::Atom {
                self.advance();
                // The atom token itself stands in for `:segment`; if it's
                // immediately followed by `{`, that's the grouped-import
                // brace (rare in practice — these specs always use `use
                // path::{..}` where the brace follows a plain `::`).
                if self.is_op("{") {
                    self.advance();
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
                self.advance(); // the Atom token (a single segment)
            } else if self.eat_op("{") {
                self.expect_ident()?;
                while self.eat_op(",") {
                    if self.is_op("}") {
                        break;
                    }
                    self.expect_ident()?;
                }
                self.expect_op("}")?;
                break;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// `struct Name { field: Type, .. }` / `enum Name { A, B, .. }` at
    /// module level. Skipped entirely by brace-depth (this AST has no
    /// data-record/enum type to put it in).
    fn skip_struct_or_enum(&mut self) -> OResult<()> {
        self.advance(); // "struct" | "enum"
        self.expect_ident()?;
        self.expect_op("{")?;
        let mut depth = 1i32;
        while depth > 0 && !self.at_eof() {
            if self.is_op("{") {
                depth += 1;
            } else if self.is_op("}") {
                depth -= 1;
            }
            self.advance();
        }
        Ok(())
    }

    /// A Rust-style type annotation — parsed and discarded. Handles `&`/
    /// `&mut`, `(A, B)`/`()` tuples, `[T]`/`[T; N]` slices/arrays, and one
    /// level of `<...>` generics.
    fn skip_type_ignored(&mut self) -> OResult<()> {
        self.eat_op("&");
        if self.is_kw("mut") {
            self.advance();
        }
        if self.eat_op("[") {
            self.skip_type_ignored()?;
            if self.eat_op(";") {
                self.advance();
            }
            self.expect_op("]")?;
            return Ok(());
        }
        if self.eat_op("(") {
            while !self.is_op(")") && !self.at_eof() {
                self.skip_type_ignored()?;
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op(")")?;
            return Ok(());
        }
        self.expect_ident()?;
        if self.eat_op("<") {
            loop {
                self.skip_type_ignored()?;
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op(">")?;
        }
        Ok(())
    }

    /// `impl Name { [pub] fn ... }*` — folded into an `ActorDef` whose
    /// `fns` are the impl block's methods (see `Item::ActorDef`'s call
    /// site in `parse_item` for why a same-named `actor`/`struct`
    /// declaration elsewhere isn't merged with this one).
    fn parse_rust_impl(&mut self) -> OResult<ActorDef> {
        let start = self.start();
        self.advance(); // "impl"
        let name = self.expect_ident()?;
        self.expect_op("{")?;
        let mut fns = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            fns.push(self.parse_rust_fn()?);
        }
        self.expect_op("}")?;
        Ok(ActorDef { name, fns, receives: Vec::new(), span: self.sp(start) })
    }

    /// `[pub] fn name([&[mut] self,] [name: Type],*) [-> Type] { rust_stmts }`.
    fn parse_rust_fn(&mut self) -> OResult<FnClause> {
        let start = self.start();
        self.eat_kw("pub");
        self.advance(); // "fn"
        let name = self.expect_ident()?;
        self.expect_op("(")?;
        let mut params = Vec::new();
        while !self.is_op(")") && !self.at_eof() {
            self.eat_op("&");
            if self.is_kw("mut") {
                self.advance();
            }
            let pname = self.expect_ident()?;
            if self.eat_op(":") {
                self.skip_type_ignored()?;
            }
            params.push(Pattern::Bind(pname));
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        if self.eat_op("->") {
            self.skip_type_ignored()?;
        }
        let body = self.parse_rust_block()?;
        Ok(FnClause { name, params, guard: None, body, is_private: false, span: self.sp(start) })
    }

    /// `{ stmts }` for the Rust-syntax dialect — unlike native Aether's
    /// `do .. end`, no explicit statement terminator is required between
    /// entries (a trailing `;`, if present, is just consumed like
    /// whitespace).
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
        if self.is_kw("fn") || (self.is_kw("pub") && self.next_tok().value == "fn") {
            // A nested free fn — not modeled as a distinct statement kind
            // by this AST; parsed (so it must be valid) and discarded.
            self.parse_rust_fn()?;
            return Ok(Stmt::Expr(Expr::Nil { span: self.sp(start) }));
        }
        if self.eat_kw("let") {
            self.eat_op("&");
            if self.is_kw("mut") {
                self.advance();
            }
            // `if let Some(x) = ..` inside a `let` isn't valid Rust, so a
            // bare pattern here is always a plain binding name, possibly
            // followed by a type annotation.
            let name = self.expect_ident()?;
            if self.eat_op(":") {
                self.skip_type_ignored()?;
            }
            self.expect_op("=")?;
            let value = self.parse_expr()?;
            return Ok(Stmt::Assign { name, value, span: self.sp(start) });
        }
        if self.is_kw("if") {
            return self.parse_rust_if();
        }
        if self.is_kw("for") {
            return self.parse_rust_for();
        }
        if self.is_kw("match") {
            return self.parse_rust_match_stmt();
        }
        if self.is_kw("return") {
            self.advance();
            let value = if self.is_op("}") || self.is_op(";") { None } else { Some(self.parse_expr()?) };
            return Ok(Stmt::Return { value, span: self.sp(start) });
        }
        // Expression statement or assignment (`name = expr`,
        // `self.field = expr` — the latter's target is evaluated but the
        // write itself is a no-op, same tradeoff as the Vera bootstrap's
        // `Stmt::AssignTarget`; this AST's `Stmt::Assign` only models a
        // plain-name target).
        let e = self.parse_expr()?;
        if self.is_op("=") && !self.is_op("==") {
            self.advance();
            let value = self.parse_expr()?;
            if let Expr::Ident { name, .. } = e {
                return Ok(Stmt::Assign { name, value, span: self.sp(start) });
            }
            return Ok(Stmt::Expr(value));
        }
        Ok(Stmt::Expr(e))
    }

    /// `if [let PAT =] cond { stmts } [else if .. / else { stmts }]`.
    fn parse_rust_if(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.advance(); // "if"
        if self.eat_kw("let") {
            self.expect_ident()?;
            if self.eat_op("(") {
                self.expect_ident()?;
                self.expect_op(")")?;
            }
            self.expect_op("=")?;
        }
        let cond = self.parse_cond_expr()?;
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

    /// `for x in iter { stmts }`.
    fn parse_rust_for(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.advance(); // "for"
        self.eat_op("&");
        if self.is_kw("mut") {
            self.advance();
        }
        let var = self.expect_ident()?;
        self.expect_kw("in")?;
        let iter = self.parse_cond_expr()?;
        let body = self.parse_rust_block()?;
        Ok(Stmt::For { var, iter, body, span: self.sp(start) })
    }

    /// `match subject { Pat(bindings) => stmt_or_block, .. }` used as a
    /// statement. Reduced to a plain identifier/wildcard `Pattern` per arm
    /// (no real enum-variant matching — see `ast::Pattern`).
    fn parse_rust_match_stmt(&mut self) -> OResult<Stmt> {
        let start = self.start();
        let (scrut, arms) = self.parse_rust_match_arms()?;
        Ok(Stmt::Case { scrut, arms, span: self.sp(start) })
    }

    fn parse_rust_match_arms(&mut self) -> OResult<(Expr, Vec<CaseArm>)> {
        self.advance(); // "match"
        let scrut = self.parse_cond_expr()?;
        self.expect_op("{")?;
        let mut arms = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            // `Name` / `Name(a, b)` / `_` / `"literal"` pattern, with
            // `MessageRole::User`-style paths folded to their last segment
            // (this bootstrap's `Pattern` has no enum-variant matching to
            // check a prefix against anyway).
            let pat_start = self.start();
            let pattern = if self.cur().kind == TokKind::Str {
                let v = self.advance().value;
                Pattern::Lit(Expr::Str { v, span: self.sp(pat_start) })
            } else if self.cur().kind == TokKind::Int {
                let v: i64 = self.advance().value.parse().unwrap_or(0);
                Pattern::Lit(Expr::Int { v, span: self.sp(pat_start) })
            } else {
                let mut pat_name = self.expect_ident()?;
                while self.is_op(":") && self.next_tok().kind == TokKind::Atom {
                    self.advance();
                    pat_name = self.advance().value;
                }
                if pat_name == "_" {
                    Pattern::Wild
                } else if self.eat_op("(") {
                    while !self.is_op(")") && !self.at_eof() {
                        self.expect_ident()?;
                        if !self.eat_op(",") {
                            break;
                        }
                    }
                    self.expect_op(")")?;
                    Pattern::Bind(pat_name)
                } else {
                    Pattern::Bind(pat_name)
                }
            };
            self.expect_op("=>")?;
            let body = if self.is_op("{") {
                self.parse_rust_block()?
            } else {
                vec![Stmt::Expr(self.parse_expr()?)]
            };
            arms.push(CaseArm { pattern, guard: None, body });
            self.eat_op(",");
        }
        self.expect_op("}")?;
        Ok((scrut, arms))
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
        // Rust reference/dereference (`&self.agents`, `&mut x`, `*x`) — a
        // no-op prefix here (this bootstrap has no reference/pointer
        // values); safe unconditionally at this fresh-operand position.
        if self.eat_op("&") {
            self.eat_kw("mut");
            return self.parse_unary();
        }
        if self.is_op("*") {
            self.advance();
            return self.parse_unary();
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut e = self.parse_atom_expr()?;
        loop {
            if self.eat_op("?") {
                // Rust try-operator — a no-op here (this bootstrap has no
                // distinct Result value to propagate); just drops the `?`.
            } else if self.eat_op(".") {
                let name = self.expect_ident()?;
                e = Expr::Attr { obj: Box::new(e), name, span: self.sp(start) };
                // Rust turbofish (`.collect::<Vec<_>>()`) — types are
                // irrelevant to this bootstrap's untyped values; skip them.
                if self.is_op(":") && self.next_tok().kind == TokKind::Op && self.next_tok().value == ":" {
                    self.advance();
                    self.advance();
                    self.skip_turbofish_args()?;
                }
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
            } else if self.is_op("!") && self.next_tok().kind == TokKind::Op && self.next_tok().value == "(" {
                // Rust macro call (`format!(..)`) — treated as an ordinary call.
                self.advance();
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
            } else if self.is_op("!") && self.next_tok().kind == TokKind::Op && self.next_tok().value == "[" {
                // `vec![...]` — evaluated as a list literal directly
                // (discarding the macro-name expression, e.g. `vec`).
                self.advance();
                self.advance();
                let mut elems = Vec::new();
                while !self.is_op("]") && !self.at_eof() {
                    elems.push(self.parse_expr()?);
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op("]")?;
                e = Expr::List { elems, span: self.sp(start) };
            } else if self.is_op("[")
                && matches!(&e, Expr::Ident { name, .. } if name.ends_with('!'))
            {
                // `vec![...]` — this lexer fuses a trailing `!` onto the
                // preceding identifier (Elixir-style `save!`/`is_empty?`
                // convention), so `vec!` arrives as one Ident token and the
                // `!`+`[` case above never actually matches; handle the
                // fused form here instead, as a list literal (comma-
                // separated elements), not an index expression.
                self.advance();
                let mut elems = Vec::new();
                while !self.is_op("]") && !self.at_eof() {
                    elems.push(self.parse_expr()?);
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op("]")?;
                e = Expr::List { elems, span: self.sp(start) };
            } else if self.eat_op("[") {
                // Plain index (`a[i]`) or a Rust slice (`a[lo..]` /
                // `a[..hi]` / `a[lo..hi]`) — slicing isn't modeled by this
                // AST's `Expr::Index`, so only the lower bound (or `0` if
                // open-ended) is kept; the upper bound is parsed (so it
                // must be valid) and discarded.
                let idx = if self.is_op("..") {
                    Expr::Int { v: 0, span: self.sp(start) }
                } else {
                    self.parse_expr()?
                };
                if self.eat_op("..") && !self.is_op("]") {
                    self.parse_expr()?;
                }
                self.expect_op("]")?;
                e = Expr::Index { obj: Box::new(e), index: Box::new(idx), span: self.sp(start) };
            } else {
                break;
            }
        }
        Ok(e)
    }

    /// Skips a turbofish `<...>` generic-argument list (already past the
    /// `::`); Aether's `>` never combines into `>>`, so nested nesting like
    /// `Vec<_>>` lexes as plain `<`/`_`/`>`/`>` tokens and a simple depth
    /// counter is enough.
    fn skip_turbofish_args(&mut self) -> OResult<()> {
        self.expect_op("<")?;
        let mut depth = 1;
        while depth > 0 && !self.at_eof() {
            if self.eat_op("<") {
                depth += 1;
            } else if self.eat_op(">") {
                depth -= 1;
            } else {
                self.advance();
            }
        }
        Ok(())
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
            // A handful of spec files use words this bootstrap reserves as
            // native keywords (e.g. `after`, from Erlang/OTP's receive-
            // timeout clause) as plain Rust-dialect variable names instead.
            // Every native-syntax use of these is matched via `is_kw` at
            // its own dedicated call site before expression parsing ever
            // reaches here, so treating them as plain identifiers in
            // expression position is safe.
            TokKind::Keyword if t.value == "after" || t.value == "end" => {
                self.advance();
                return Ok(Expr::Ident { name: t.value, span: self.sp(start) });
            }
            TokKind::Ident => {
                self.advance();
                // Namespaced path (`HashMap::new`, `MessageRole::User`) —
                // folded into one atomic identifier. Note: because this
                // lexer treats `:` immediately followed by an identifier
                // character as the START OF AN ATOM (`:foo`, see
                // `lexer.rs::lex_atom`), the second `:` of a `::` here
                // combines with the following segment into one `Atom`
                // token (value already without its leading `:`), not two
                // separate `Op(":")` tokens.
                let mut name = t.value;
                while self.is_op(":") && self.next_tok().kind == TokKind::Atom {
                    self.advance(); // the first ':'
                    let seg = self.advance().value; // the Atom token (segment, no leading ':')
                    name.push_str("::");
                    name.push_str(&seg);
                }
                // Rust struct literal (`Value { key: .., value: .. }`) —
                // gated the same way as the Sylva/Vera bootstraps' same-
                // named flag, to avoid swallowing an `if`/`case`/`for`
                // condition's own block-opening `{`.
                if self.allow_struct_lit && self.is_op("{") {
                    self.advance();
                    let mut elems = Vec::new();
                    while !self.is_op("}") && !self.at_eof() {
                        if self.eat_op("..") {
                            // Struct-update spread (`..base`) — the base's
                            // fields aren't modeled by this AST's plain
                            // field-value list; evaluate for side effects
                            // only and drop it.
                            self.parse_expr()?;
                            break;
                        }
                        let field_start = self.start();
                        let fname = self.expect_ident()?;
                        if self.eat_op(":") {
                            elems.push(self.parse_expr()?);
                        } else {
                            // Field-init shorthand (`content,` == `content: content`).
                            elems.push(Expr::Ident { name: fname, span: self.sp(field_start) });
                        }
                        if !self.eat_op(",") {
                            break;
                        }
                    }
                    self.expect_op("}")?;
                    return Ok(Expr::List { elems, span: self.sp(start) });
                }
                return Ok(Expr::Ident { name, span: self.sp(start) });
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
        // Rust-syntax `match subject { Pat => expr, .. }` used in
        // expression position — the brace-delimited sibling of native
        // Aether's `case .. do .. end` (`parse_case_body`, just above).
        if self.is_kw("match") {
            let (scrut, arms) = self.parse_rust_match_arms()?;
            return Ok(Expr::Case { scrut: Box::new(scrut), arms, span: self.sp(start) });
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
        if self.is_kw("fn") {
            return self.parse_lambda();
        }
        // Rust closure: `|x| expr` / `|x| { stmt* }` / `|| expr`.
        if self.is_op("|") || self.is_op("||") {
            let params = if self.eat_op("||") {
                Vec::new()
            } else {
                self.expect_op("|")?;
                let mut params = Vec::new();
                while !self.is_op("|") && !self.at_eof() {
                    params.push(Pattern::Bind(self.expect_ident()?));
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op("|")?;
                params
            };
            let body = if self.is_op("{") {
                let stmts = self.parse_rust_block()?;
                let bspan = self.sp(start);
                stmts
                    .into_iter()
                    .rev()
                    .find_map(|s| match s {
                        Stmt::Expr(e) => Some(e),
                        Stmt::Return { value, .. } => value,
                        _ => None,
                    })
                    .unwrap_or(Expr::Nil { span: bspan })
            } else {
                self.parse_expr()?
            };
            return Ok(Expr::Lambda { params, body: Box::new(body), span: self.sp(start) });
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
