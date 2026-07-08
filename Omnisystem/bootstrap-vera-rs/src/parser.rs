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
    /// Disabled while parsing an `if`/`for`/`match` condition-or-subject
    /// expression so it doesn't swallow the construct's own `{` as a
    /// struct-literal start (`if config {` is the `if`'s block, not
    /// `config { .. }` instantiation) — mirrors the same-named flag in the
    /// Sylva bootstrap.
    allow_struct_lit: bool,
}

impl<'a> Parser<'a> {
    pub fn new(toks: Vec<Token>, file: &'a str) -> Self {
        Parser { toks, p: 0, file, allow_struct_lit: true }
    }

    /// `parse_expr`, but with struct-literal parsing disabled for the
    /// duration of the call — for `if`/`for`/`match`'s condition/subject
    /// position, where a following `{` is that construct's own block, not
    /// a struct literal (see `allow_struct_lit`'s doc comment). Save/
    /// restore (not unconditional reset) since this nests: a condition can
    /// itself contain a closure whose body legitimately allows struct
    /// literals.
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
            } else if self.cur().kind == TokKind::Ident && self.cur().value == "use" {
                self.parse_use()?;
            } else if self.cur().kind == TokKind::Ident && (self.cur().value == "struct" || self.cur().value == "enum") {
                self.skip_struct_or_enum()?;
            } else if self.is_kw("fn") {
                self.skip_toplevel_fn()?;
            } else {
                script.push(self.parse_stmt()?);
            }
        }
        Ok(Module { components, script })
    }

    /// `struct Name { field: Type, .. }` / `enum Name { A, B, .. }` at
    /// module level — Rust-style data declarations some omni-integration
    /// specs use alongside `component`s. Vera's AST has no place to put
    /// these (components are the only structural unit), so they're parsed
    /// (to stay lex/parse-correct) and entirely discarded.
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

    /// A free (non-component-method) `fn name(..) [-> Type] { body }` at
    /// module level — parsed (params/return-type/body all consumed
    /// correctly) and discarded; only `component` methods are modeled by
    /// this AST.
    fn skip_toplevel_fn(&mut self) -> OResult<()> {
        self.advance(); // "fn"
        self.expect_ident()?;
        self.expect_op("(")?;
        let mut depth = 1i32;
        while depth > 0 && !self.at_eof() {
            if self.is_op("(") {
                depth += 1;
            } else if self.is_op(")") {
                depth -= 1;
            }
            self.advance();
        }
        if self.eat_op("->") {
            // Return type: any identifier path/generic run up to the body's `{`.
            while !self.is_op("{") && !self.at_eof() {
                self.advance();
            }
        }
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

    /// `use path::{a, b, c}` / `use path::item` — Rust-style grouped
    /// import, used verbatim by some omni-integration specs regardless of
    /// this bootstrap's own (nonexistent) module system. Parsed and
    /// entirely discarded — this bootstrap has no cross-file resolution to
    /// hook it into.
    fn parse_use(&mut self) -> OResult<()> {
        self.advance(); // "use"
        self.expect_ident()?;
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
            self.expect_ident()?;
        }
        Ok(())
    }

    fn parse_component(&mut self) -> OResult<ComponentDef> {
        let start = self.start();
        self.expect_kw("component")?;
        let name = self.expect_ident()?;
        // Two component shapes: native `component Name(prop, prop2) { .. }`
        // (props as a parenthesized parameter list), and the
        // omni-integration dialect's `component Name { props { field:
        // Type, .. } .. }` (props as a typed field block, parsed alongside
        // `state`/`render`/etc. below rather than up front here).
        let mut props = Vec::new();
        if self.eat_op("(") {
            while !self.is_op(")") && !self.at_eof() {
                props.push(self.expect_ident()?);
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op(")")?;
        }
        self.expect_op("{")?;
        let mut state = Vec::new();
        let mut computed = Vec::new();
        let mut methods = Vec::new();
        let mut render = Vec::new();
        let mut saw_render = false;
        while !self.is_op("}") && !self.at_eof() {
            if self.cur().kind == TokKind::Ident && self.cur().value == "props" && self.toks.get(self.p + 1).is_some_and(|t| t.value == "{") {
                self.advance();
                self.expect_op("{")?;
                while !self.is_op("}") && !self.at_eof() {
                    props.push(self.expect_ident()?);
                    self.expect_op(":")?;
                    self.skip_type_ignored()?;
                    if self.eat_op("=") {
                        self.parse_expr()?; // default value — discarded (see `ComponentDef.props: Vec<String>`)
                    }
                    self.eat_op(",");
                }
                self.expect_op("}")?;
            } else if self.is_kw("state") && self.toks.get(self.p + 1).is_some_and(|t| t.value == "{") {
                self.advance();
                self.expect_op("{")?;
                while !self.is_op("}") && !self.at_eof() {
                    let n = self.expect_ident()?;
                    self.expect_op(":")?;
                    self.skip_type_ignored()?;
                    let v = if self.eat_op("=") { self.parse_expr()? } else { Expr::Int { v: 0, span: self.sp(start) } };
                    state.push((n, v));
                    self.eat_op(",");
                }
                self.expect_op("}")?;
            } else if self.eat_kw("state") {
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
            } else if self.cur().kind == TokKind::Ident && self.cur().value == "style" && self.toks.get(self.p + 1).is_some_and(|t| t.value == "{") {
                // `style { ".selector" { prop: val, .. } .. }` — CSS-like
                // metadata (hyphenated property names like `border-top`
                // wouldn't even lex as one identifier). Not modeled by this
                // AST at all; skipped by brace-depth alone, without trying
                // to make sense of its contents.
                self.advance();
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
            } else if self.cur().kind == TokKind::Ident && self.toks.get(self.p + 1).is_some_and(|t| t.value == "(") {
                // Method shorthand (`on_kill() { .. }`, no `fn` keyword).
                methods.push(self.parse_fn_def_noheader()?);
            } else {
                return Err(self.err(format!("expected 'props'/'state'/'computed'/'fn'/'render'/a method in component body, found '{}'", self.cur_desc())));
            }
        }
        self.expect_op("}")?;
        if !saw_render {
            return Err(self.err(format!("component '{name}' has no render block")));
        }
        Ok(ComponentDef { name, props, state, computed, methods, render, span: self.sp(start) })
    }

    /// A field's type annotation in a `props`/`state` block: an identifier
    /// path, optionally with one level of `<...>` generics. Discarded —
    /// this bootstrap models everything dynamically.
    fn skip_type_ignored(&mut self) -> OResult<()> {
        self.eat_op("&");
        if self.cur().kind == TokKind::Ident && self.cur().value == "mut" {
            self.advance();
        }
        // `fn(Type, ..) -> Type` — a callback-prop type (`on_submit: fn(String) -> ()`).
        if self.is_kw("fn") {
            self.advance();
            self.expect_op("(")?;
            while !self.is_op(")") && !self.at_eof() {
                self.skip_type_ignored()?;
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op(")")?;
            if self.eat_op("->") {
                self.skip_type_ignored()?;
            }
            return Ok(());
        }
        // `()` unit / `(A, B)` tuple type.
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

    /// Like `parse_fn_def`, but for the method-shorthand form (`name(..) {
    /// .. }`, no leading `fn` keyword) used by the omni-integration
    /// dialect's `on_kill()`-style event handlers.
    fn parse_fn_def_noheader(&mut self) -> OResult<FnDef> {
        let start = self.start();
        let name = self.expect_ident()?;
        self.expect_op("(")?;
        let mut params = Vec::new();
        while !self.is_op(")") && !self.at_eof() {
            params.push(self.expect_ident()?);
            if self.eat_op(":") {
                self.skip_type_ignored()?; // typed param (`t: String`) — type discarded
            }
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
            // Optional `;` between statements (Rust-flavored specs use it
            // as a separator; this bootstrap's own grammar has no
            // statement terminator at all, so it's just skipped).
            self.eat_op(";");
        }
        Ok(out)
    }

    fn parse_stmt(&mut self) -> OResult<Stmt> {
        let start = self.start();
        if self.is_kw("if") {
            return self.parse_if_stmt();
        }
        if self.eat_kw("let") {
            self.eat_op("&");
            let name = self.expect_ident()?;
            if self.eat_op(":") {
                self.skip_type_ignored()?;
            }
            self.expect_op("=")?;
            let value = self.parse_expr()?;
            return Ok(Stmt::Assign { name, value, span: self.sp(start) });
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
        // General assignment target (`self.field = expr`, `obj[i] = expr`)
        // — see `ast::Stmt::AssignTarget`'s doc comment for what's modeled.
        let e = self.parse_expr()?;
        if matches!(e, Expr::Attr { .. }) && matches!(self.cur().value.as_str(), "=" | "+=" | "-=") && self.cur().kind == TokKind::Op {
            let aug = self.advance().value;
            let rhs = self.parse_expr()?;
            let value = if aug == "=" {
                rhs
            } else {
                let op = aug.trim_end_matches('=').to_string();
                Expr::BinOp { op, left: Box::new(e.clone()), right: Box::new(rhs), span: self.sp(start) }
            };
            return Ok(Stmt::AssignTarget { target: e, value, span: self.sp(start) });
        }
        Ok(Stmt::Expr(e))
    }

    fn parse_if_stmt(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.expect_kw("if")?;
        let cond = self.parse_cond_expr()?;
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
        // Rust reference/dereference (`&self.messages`, `*x`) — a no-op
        // prefix here (this bootstrap has no reference/pointer values).
        if self.is_op("&") {
            self.advance();
            // "mut" isn't a Vera keyword (lexes as a plain Ident) — checked
            // by value directly rather than via `eat_kw`.
            if self.cur().kind == TokKind::Ident && self.cur().value == "mut" {
                self.advance();
            }
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
            } else if self.is_op("!") && self.next_tok().kind == TokKind::Op && self.next_tok().value == "(" {
                // Rust macro call (`format!(..)`) — treated as an ordinary call.
                self.advance(); // '!'
                self.advance(); // '('
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
                // `obj[i]` / `obj[lo..hi]` / `obj[..hi]` / `obj[lo..]` /
                // `obj[..]` — this bootstrap's `Expr` has no
                // indexing/slicing node at all, so the index/range is
                // parsed (so it must be valid) and discarded, yielding the
                // object unchanged. A parse-level-only simplification, same
                // tradeoff as elsewhere in this dialect extension.
                if !self.is_op("..") && !self.is_op("]") {
                    self.parse_expr()?;
                }
                if self.eat_op("..") && !self.is_op("]") {
                    self.parse_expr()?;
                }
                self.expect_op("]")?;
            } else {
                break;
            }
        }
        Ok(e)
    }

    /// The value a `{ .. }` block "evaluates to" when used where an
    /// expression is expected (`Expr::IfExpr`'s branches): its last
    /// `Stmt::Expr`/`Stmt::Assign`'s value, or a `Bool(false)` placeholder
    /// if it has none.
    fn block_trailing_expr(stmts: Vec<Stmt>, fallback_span: Span) -> Expr {
        stmts
            .into_iter()
            .rev()
            .find_map(|s| match s {
                Stmt::Expr(e) => Some(e),
                Stmt::Assign { value, .. } | Stmt::AssignTarget { value, .. } => Some(value),
                Stmt::Return { value, .. } => value,
                _ => None,
            })
            .unwrap_or(Expr::Bool { v: false, span: fallback_span })
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
                // Namespaced path (`HashMap::new`) — folded into one
                // atomic identifier; this bootstrap has no module system
                // to resolve it against, but it needs to parse.
                let mut name = t.value;
                while self.is_op(":") && self.next_tok().kind == TokKind::Op && self.next_tok().value == ":" {
                    self.advance();
                    self.advance();
                    name.push_str("::");
                    name.push_str(&self.expect_ident()?);
                }
                // Rust struct literal (`ChatMessage { field: val, .. }`) —
                // gated by `allow_struct_lit` (see its doc comment). This
                // bootstrap's `Expr` has no record/struct value node, so
                // it's modeled as `Expr::List` of just the field values
                // (field names discarded) — good enough to parse and
                // evaluate the values for side effects, not to actually
                // construct a usable object.
                if self.allow_struct_lit && self.is_op("{") {
                    self.advance();
                    let mut elems = Vec::new();
                    while !self.is_op("}") && !self.at_eof() {
                        self.expect_ident()?;
                        self.expect_op(":")?;
                        elems.push(self.parse_expr()?);
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
        // `if cond { expr } else { expr }` in expression position (e.g. an
        // attribute value: `class=if self.disabled { "a" } else { "b" }`).
        if self.is_kw("if") {
            self.advance();
            let cond = self.parse_cond_expr()?;
            self.expect_op("{")?;
            let then_span = self.sp(start);
            let then_ = Self::block_trailing_expr(self.parse_stmts_until_close()?, then_span);
            self.expect_op("}")?;
            self.expect_kw("else")?;
            self.expect_op("{")?;
            let else_span = self.sp(start);
            let else_ = Self::block_trailing_expr(self.parse_stmts_until_close()?, else_span);
            self.expect_op("}")?;
            return Ok(Expr::IfExpr { cond: Box::new(cond), then_: Box::new(then_), else_: Box::new(else_), span: self.sp(start) });
        }
        // `match subject { Pat(bindings) => expr, .. }` in expression
        // position (see `ast::Expr::MatchExpr`'s doc comment).
        if self.is_kw("match") {
            self.advance();
            let subject = self.parse_cond_expr()?;
            self.expect_op("{")?;
            let mut arms = Vec::new();
            while !self.is_op("}") && !self.at_eof() {
                let mut pat_name = self.expect_ident()?;
                while self.is_op(":") && self.next_tok().kind == TokKind::Op && self.next_tok().value == ":" {
                    self.advance();
                    self.advance();
                    pat_name = self.expect_ident()?;
                }
                let mut bindings = Vec::new();
                if self.eat_op("(") {
                    while !self.is_op(")") && !self.at_eof() {
                        bindings.push(self.expect_ident()?);
                        if !self.eat_op(",") {
                            break;
                        }
                    }
                    self.expect_op(")")?;
                }
                self.expect_op("=>")?;
                let arm_body = self.parse_expr()?;
                arms.push((pat_name, bindings, arm_body));
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op("}")?;
            return Ok(Expr::MatchExpr { subject: Box::new(subject), arms, span: self.sp(start) });
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
            // `|| { stmt* }` (multi-statement, possibly empty) or `|| expr`
            // — zero-arg shorthand handler (common for onClick).
            if self.is_op("{") {
                self.advance();
                let body = self.parse_stmts_until_close()?;
                self.expect_op("}")?;
                return Ok(Expr::Lambda { params: vec![], body, span: self.sp(start) });
            }
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
            // `|params| { stmt* }` — a multi-statement closure body (as
            // opposed to the single-expression form below).
            if self.is_op("{") {
                self.advance();
                let body = self.parse_stmts_until_close()?;
                self.expect_op("}")?;
                return Ok(Expr::Lambda { params, body, span: self.sp(start) });
            }
            // A closure body that's a markup element (`.map(|a| <Node .../>)`)
            // — unambiguous here (a fresh closure-body position), unlike
            // general expression context; see `ast::Expr::Markup`'s doc
            // comment.
            let e = if self.is_op("<") {
                let mstart = self.start();
                let node = self.parse_element()?;
                Expr::Markup(Box::new(node), self.sp(mstart))
            } else {
                self.parse_expr()?
            };
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
        // Rust-macro-flavored markup (an alternate dialect some
        // omni-integration specs use throughout instead of JSX-style
        // `<Tag>`): `tag(attr=val, on_event=|e| {..}, ..) { children }` /
        // `tag(attr=val)` (no children).
        if self.cur().kind == TokKind::Ident && self.next_tok().kind == TokKind::Op && self.next_tok().value == "(" {
            return self.parse_call_style_element();
        }
        if self.cur().kind == TokKind::Str {
            let t = self.advance();
            return Ok(Node::Text(t.value));
        }
        // Bare `if cond { .. } [else { .. }]` / `for x in xs { .. }` used
        // directly as a node (as opposed to `{if ..}`/`{for ..}`, which
        // are the same thing wrapped in an interpolation-style `{ }` —
        // both shapes appear across the omni-integration specs).
        if self.is_kw("if") {
            return self.parse_node_if_bare();
        }
        if self.is_kw("for") {
            return self.parse_node_for_bare();
        }
        if self.is_kw("match") {
            return self.parse_node_match();
        }
        // `let name = expr;` appearing directly among children — see
        // `ast::Node::Let`'s doc comment.
        if self.is_kw("let") {
            self.advance();
            self.eat_op("&");
            let name = self.expect_ident()?;
            if self.eat_op(":") {
                self.skip_type_ignored()?;
            }
            self.expect_op("=")?;
            let value = self.parse_expr()?;
            self.eat_op(";");
            return Ok(Node::Let { name, value });
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
        // Bare inline text (`<span>parallel ≤ {n}</span>` — "parallel ≤ " is
        // unquoted). Not a real word-boundary/whitespace-preserving
        // reconstruction (this parser only sees the already-tokenized
        // stream, not raw source spans) — consecutive non-markup tokens up
        // to the next `<`/`{`/`}` are joined with single spaces, which is
        // fine for this bar (parses cleanly; doesn't have to re-render
        // pixel-identical whitespace).
        if !self.is_op("<") && !self.is_op("{") && !self.is_op("}") && !self.at_eof() {
            let mut words = vec![self.advance().value];
            while !self.is_op("<") && !self.is_op("{") && !self.is_op("}") && !self.at_eof() {
                words.push(self.advance().value);
            }
            return Ok(Node::Text(words.join(" ")));
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
                    // Unwrapped literal (e.g. `options=["a", "b"]`) — not
                    // just the `{expr}`/string forms above.
                    self.parse_expr()?
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

    /// `tag(attr=val, ..) [{ children }]` — see `parse_node`'s doc comment.
    /// Attribute values are ordinary expressions (closures included), not
    /// JSX's `{expr}`-wrapped form.
    fn parse_call_style_element(&mut self) -> OResult<Node> {
        let start = self.start();
        let tag = self.expect_ident()?;
        self.expect_op("(")?;
        let mut attrs = Vec::new();
        let mut positional = 0usize;
        while !self.is_op(")") && !self.at_eof() {
            // `name=value` kwarg-style attr vs. a positional arg
            // (`icon("paperclip")` — really more "helper function call"
            // than "element with attrs", but modeled the same way here;
            // synthetic `_0`/`_1`/.. names since `Node::Element.attrs` is
            // name-keyed).
            if self.cur().kind == TokKind::Ident && self.next_tok().kind == TokKind::Op && self.next_tok().value == "=" {
                let aname = self.advance().value;
                self.advance(); // '='
                attrs.push((aname, self.parse_expr()?));
            } else {
                attrs.push((format!("_{positional}"), self.parse_expr()?));
                positional += 1;
            }
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        let children = if self.eat_op("{") {
            let c = self.parse_nodes_until_close()?;
            self.expect_op("}")?;
            c
        } else {
            Vec::new()
        };
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
        let node = self.parse_node_if_bare()?;
        self.expect_op("}")?; // closes the outer `{if ...}` wrapper
        Ok(node)
    }

    fn parse_node_for(&mut self) -> OResult<Node> {
        let node = self.parse_node_for_bare()?;
        self.expect_op("}")?; // closes the outer `{for ...}` wrapper
        Ok(node)
    }

    /// `if cond { .. } [else { .. }]` as a node, with no outer `{ }`
    /// wrapper to close (see `parse_node_if`, which adds that for the
    /// `{if ..}` interpolation shape).
    fn parse_node_if_bare(&mut self) -> OResult<Node> {
        self.expect_kw("if")?;
        // `if let Some(x) = expr { .. }` — reduced to evaluating `expr` for
        // truthiness (`Some`/`None`/`Ok`/`Err` aren't modeled as distinct
        // values), same tradeoff as the Sylva bootstrap's `if let` support.
        let cond = if self.eat_kw("let") {
            self.expect_ident()?;
            if self.eat_op("(") {
                self.expect_ident()?;
                self.expect_op(")")?;
            }
            self.expect_op("=")?;
            self.parse_cond_expr()?
        } else {
            self.parse_cond_expr()?
        };
        self.expect_op("{")?;
        let then_branch = self.parse_nodes_until_close()?;
        self.expect_op("}")?;
        let mut else_branch = Vec::new();
        if self.eat_kw("else") {
            self.expect_op("{")?;
            else_branch = self.parse_nodes_until_close()?;
            self.expect_op("}")?;
        }
        Ok(Node::If { cond, then_branch, else_branch })
    }

    /// `for x in xs { .. }` as a node, with no outer `{ }` wrapper to close
    /// (see `parse_node_for`).
    fn parse_node_for_bare(&mut self) -> OResult<Node> {
        self.expect_kw("for")?;
        let var = self.expect_ident()?;
        self.expect_kw("in")?;
        let iter = self.parse_cond_expr()?;
        self.expect_op("{")?;
        let body = self.parse_nodes_until_close()?;
        self.expect_op("}")?;
        Ok(Node::For { var, iter, body })
    }

    /// `match subject { Pat(bindings) => { nodes } .. }` as a node — see
    /// `ast::Node::Match`'s doc comment for what's modeled.
    fn parse_node_match(&mut self) -> OResult<Node> {
        self.expect_kw("match")?;
        let subject = self.parse_cond_expr()?;
        self.expect_op("{")?;
        let mut arms = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            // `MessageRole::User` — only the last segment is kept as the
            // effective pattern name (this bootstrap's Match has no real
            // enum-variant resolution to check the prefix against anyway).
            let mut pat_name = self.expect_ident()?;
            while self.is_op(":") && self.next_tok().kind == TokKind::Op && self.next_tok().value == ":" {
                self.advance();
                self.advance();
                pat_name = self.expect_ident()?;
            }
            let mut bindings = Vec::new();
            if self.eat_op("(") {
                while !self.is_op(")") && !self.at_eof() {
                    bindings.push(self.expect_ident()?);
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op(")")?;
            }
            self.expect_op("=>")?;
            self.expect_op("{")?;
            let nodes = self.parse_nodes_until_close()?;
            self.expect_op("}")?;
            arms.push((pat_name, bindings, nodes));
            self.eat_op(",");
        }
        self.expect_op("}")?;
        Ok(Node::Match { subject, arms })
    }
}
