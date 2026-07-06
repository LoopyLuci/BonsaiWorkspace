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
    /// Guards `Ident { .. }` struct-literal parsing (see `parse_atom`'s
    /// `Ident` case) so an `if`/`while`/`for` condition or range bound
    /// ending in a bare identifier (`if in_bar { .. }`, `for i in
    /// point_count { .. }`) doesn't swallow the construct's own
    /// block-opening `{` as the start of a struct literal. Saved/restored
    /// (not unconditionally reset) around each guarded parse, since a
    /// condition can itself contain a closure whose body legitimately
    /// allows struct literals.
    allow_struct_lit: bool,
}

impl<'a> Parser<'a> {
    pub fn new(toks: Vec<Token>, file: &'a str) -> Self {
        Parser { toks, p: 0, file, allow_struct_lit: true }
    }

    fn cur(&self) -> &Token {
        &self.toks[self.p.min(self.toks.len() - 1)]
    }
    fn next_tok(&self) -> &Token {
        &self.toks[(self.p + 1).min(self.toks.len() - 1)]
    }
    /// `parse_expr`, but with struct-literal parsing disabled for the
    /// duration of the call — see `allow_struct_lit`'s doc comment.
    fn parse_cond_expr(&mut self) -> OResult<Expr> {
        let prev = self.allow_struct_lit;
        self.allow_struct_lit = false;
        let e = self.parse_expr();
        self.allow_struct_lit = prev;
        e
    }
    /// A plain (non-reserved) word used as a soft keyword — `var`, `while`,
    /// `const`, `struct`, `impl`, `mod`, `pub`, `use` — none of which are
    /// in Helix's `KEYWORDS`, so they lex as ordinary `Ident` tokens and
    /// are dispatched on their text instead.
    fn eat_ident_word(&mut self, v: &str) -> bool {
        if self.cur().kind == TokKind::Ident && self.cur().value == v {
            self.advance();
            true
        } else {
            false
        }
    }
    fn is_ident_word(&self, v: &str) -> bool {
        self.cur().kind == TokKind::Ident && self.cur().value == v
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
            } else if self.is_ident_word("pipeline") {
                self.advance();
                self.parse_pipeline_discard()?;
            } else if self.is_ident_word("struct") {
                self.advance();
                self.expect_ident()?; // struct name
                self.skip_struct_type_body()?;
            } else if self.is_ident_word("impl") {
                self.advance();
                self.parse_impl_discard()?;
            } else if (self.is_ident_word("pub") && self.next_tok().value == "mod") || self.is_ident_word("mod") {
                self.eat_ident_word("pub");
                self.eat_ident_word("mod");
                self.parse_mod_discard()?;
            } else {
                script.push(self.parse_stmt()?);
            }
        }
        Ok(Module { fns, kernels, shaders, script })
    }

    /// The WGSL-dialect `pipeline NAME { .. }` construct — an entirely
    /// different shape from `kernel`/`shader` (metadata properties,
    /// `inputs`/`outputs`/`uniforms`/`bindings`/`shared` typed field
    /// blocks, and either a `code { .. }` or `compute @workgroup(N) { .. }`
    /// body). Parsed (so it must be valid) and discarded entirely — this
    /// bootstrap has no real GPU pipeline/binding-layout concept to store
    /// it as.
    fn parse_pipeline_discard(&mut self) -> OResult<()> {
        self.expect_ident()?; // pipeline name
        self.expect_op("{")?;
        while !self.is_op("}") && !self.at_eof() {
            if self.is_kw("compute") {
                self.advance();
                self.skip_attrs()?; // `@workgroup(64)`
                self.expect_op("{")?;
                self.parse_stmts_until_close()?;
                self.expect_op("}")?;
                continue;
            }
            if self.cur().kind == TokKind::Ident {
                match self.cur().value.as_str() {
                    "inputs" | "outputs" | "uniforms" | "shared" => {
                        self.advance();
                        self.parse_type_field_block()?;
                        continue;
                    }
                    "bindings" => {
                        self.advance();
                        self.parse_bindings_block()?;
                        continue;
                    }
                    "code" => {
                        self.advance();
                        self.expect_op("{")?;
                        self.parse_stmts_until_close()?;
                        self.expect_op("}")?;
                        continue;
                    }
                    "attachments" => {
                        self.advance();
                        self.expect_op("{")?;
                        while !self.is_op("}") && !self.at_eof() {
                            self.expect_ident()?;
                            self.expect_op(":")?;
                            self.parse_expr()?;
                        }
                        self.expect_op("}")?;
                        continue;
                    }
                    _ => {}
                }
            }
            // Flat metadata property: `target: compute`, `topology:
            // triangle_strip`, `workgroup_size: [16, 16, 1]`,
            // `vertex_shader: FullscreenVertex`, ...
            self.expect_ident()?;
            self.expect_op(":")?;
            self.parse_expr()?;
        }
        self.expect_op("}")?;
        Ok(())
    }

    /// `inputs { .. }` / `outputs { .. }` / `uniforms { .. }` / `shared { .. }`
    /// — `name : type @attr(..)*` fields, discarded.
    fn parse_type_field_block(&mut self) -> OResult<()> {
        self.expect_op("{")?;
        while !self.is_op("}") && !self.at_eof() {
            self.expect_ident()?;
            self.expect_op(":")?;
            self.skip_type_ignored()?;
            self.skip_attrs()?;
        }
        self.expect_op("}")?;
        Ok(())
    }

    /// `bindings { @group(0) @binding(0) storage_buffer name: type .. }` —
    /// attributes come *before* the qualifier+name here, unlike
    /// `inputs`/`outputs`/`uniforms` where they trail the type.
    fn parse_bindings_block(&mut self) -> OResult<()> {
        self.expect_op("{")?;
        while !self.is_op("}") && !self.at_eof() {
            self.skip_attrs()?;
            self.expect_ident()?; // qualifier: storage_buffer / uniform_buffer
            self.expect_ident()?; // field name
            self.expect_op(":")?;
            self.skip_type_ignored()?;
        }
        self.expect_op("}")?;
        Ok(())
    }

    /// Zero or more `@ident[(args)]` attribute decorations
    /// (`@binding(0)`, `@builtin(position)`, `@uniform`).
    fn skip_attrs(&mut self) -> OResult<()> {
        while self.eat_op("@") {
            self.expect_ident()?;
            if self.eat_op("(") {
                while !self.is_op(")") && !self.at_eof() {
                    self.parse_expr()?;
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op(")")?;
            }
        }
        Ok(())
    }

    /// A type reference: `f32`, `buffer<u32>`, `array<vec4<f32>>`,
    /// `Vec<f32>`, `&[u8]`, `&self`'s referent, or an inline
    /// `struct { field: type, .. }` — parsed (so it must be valid) and
    /// discarded; this bootstrap has no static type system to check
    /// against.
    fn skip_type_ignored(&mut self) -> OResult<()> {
        self.eat_op("&");
        self.eat_ident_word("mut");
        if self.eat_op("[") {
            self.skip_type_ignored()?;
            if self.eat_op(";") {
                self.parse_expr()?;
            }
            self.expect_op("]")?;
            return Ok(());
        }
        if self.eat_ident_word("struct") {
            return self.skip_struct_type_body();
        }
        self.expect_ident()?; // base type name (possibly a `path::to::Type`)
        while self.eat_op("::") {
            self.expect_ident()?;
        }
        if self.eat_op("<") {
            loop {
                if self.cur().kind == TokKind::Int {
                    self.advance();
                } else {
                    self.skip_type_ignored()?;
                }
                if self.eat_op(",") {
                    continue;
                }
                break;
            }
            self.eat_close_angle()?;
        }
        Ok(())
    }

    /// Consumes a single `>` closing one generic-argument nesting level.
    /// This lexer fuses adjacent `>` characters into a `>>` token (needed
    /// for the bitwise right-shift operator), so a doubly-nested close
    /// like `array<vec4<f32>>` must "shrink" a `>>` token to a lone `>` in
    /// place rather than fully consuming it, leaving the remainder for the
    /// enclosing generic level to close — same technique used for `::<>`
    /// turbofish-closing in the other bootstraps this session.
    fn eat_close_angle(&mut self) -> OResult<()> {
        if self.eat_op(">") {
            return Ok(());
        }
        if self.is_op(">>") {
            let i = self.p.min(self.toks.len() - 1);
            self.toks[i].value = ">".to_string();
            return Ok(());
        }
        Err(self.err(format!("expected '>' to close a generic type but found '{}'", self.cur_desc())))
    }

    fn skip_struct_type_body(&mut self) -> OResult<()> {
        self.expect_op("{")?;
        while !self.is_op("}") && !self.at_eof() {
            self.expect_ident()?;
            self.expect_op(":")?;
            self.skip_type_ignored()?;
            self.eat_op(",");
        }
        self.expect_op("}")?;
        Ok(())
    }

    /// `impl NAME { [pub] fn method(..) { .. } .. }` — the Rust-dialect
    /// GPU-context glue code's method-definition block; each method is
    /// parsed via the same `parse_fn` used for top-level `fn`s (now
    /// tolerant of typed/referenced params and a `-> Type` return arrow)
    /// but the result is discarded rather than stored, since these are
    /// methods on a type this bootstrap doesn't model, not callable
    /// top-level functions.
    fn parse_impl_discard(&mut self) -> OResult<()> {
        self.expect_ident()?; // impl target type name
        self.expect_op("{")?;
        while !self.is_op("}") && !self.at_eof() {
            self.eat_ident_word("pub");
            self.parse_fn()?;
        }
        self.expect_op("}")?;
        Ok(())
    }

    /// `[pub] mod NAME { [pub] use path::items [pub] fn .. }` — discarded
    /// entirely, same rationale as `impl`.
    fn parse_mod_discard(&mut self) -> OResult<()> {
        self.expect_ident()?; // mod name
        self.expect_op("{")?;
        while !self.is_op("}") && !self.at_eof() {
            self.eat_ident_word("pub");
            if self.eat_ident_word("use") {
                self.expect_ident()?;
                while self.eat_op("::") {
                    self.expect_ident()?;
                }
                continue;
            }
            if self.is_kw("fn") {
                self.parse_fn()?;
                continue;
            }
            return Err(self.err(format!("expected 'use' or 'fn' inside 'mod' but found '{}'", self.cur_desc())));
        }
        self.expect_op("}")?;
        Ok(())
    }

    fn parse_params(&mut self) -> OResult<Vec<String>> {
        self.expect_op("(")?;
        let mut params = Vec::new();
        while !self.is_op(")") && !self.at_eof() {
            if self.eat_op("&") {
                self.eat_ident_word("mut");
                params.push(self.expect_ident()?); // typically `self`
            } else {
                let name = self.expect_ident()?;
                if self.eat_op(":") {
                    self.skip_type_ignored()?;
                }
                params.push(name);
            }
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
        if self.eat_op("->") {
            self.skip_type_ignored()?;
        }
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
        if self.is_op("(") {
            // Native form: `shader vertex Name(params) { stmts }`.
            let params = self.parse_params()?;
            self.expect_op("{")?;
            let body = self.parse_stmts_until_close()?;
            self.expect_op("}")?;
            return Ok(ShaderDef { name, stage, params, body, span: self.sp(start) });
        }
        // WGSL-dialect form: `shader vertex Name { inputs{} outputs{}
        // [uniforms{}] code{ stmts } }` — no explicit param list; the
        // shader's `code` block reads implicit input fields off `in.*`
        // instead (`in` lexes as a `Keyword`, accepted as a bare atom by
        // `parse_atom`'s dedicated case, since it isn't the `for .. in ..`
        // usage here).
        self.expect_op("{")?;
        let mut body = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            if self.cur().kind == TokKind::Ident {
                match self.cur().value.as_str() {
                    "inputs" | "outputs" | "uniforms" => {
                        self.advance();
                        self.parse_type_field_block()?;
                        continue;
                    }
                    "code" => {
                        self.advance();
                        self.expect_op("{")?;
                        body = self.parse_stmts_until_close()?;
                        self.expect_op("}")?;
                        continue;
                    }
                    _ => {}
                }
            }
            return Err(self.err(format!("expected 'inputs', 'outputs', 'uniforms', or 'code' but found '{}'", self.cur_desc())));
        }
        self.expect_op("}")?;
        Ok(ShaderDef { name, stage, params: Vec::new(), body, span: self.sp(start) })
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
        if self.eat_kw("let") || self.eat_ident_word("const") {
            let name = self.expect_ident()?;
            if self.eat_op(":") {
                self.skip_type_ignored()?;
            }
            self.expect_op("=")?;
            let value = self.parse_expr()?;
            return Ok(Stmt::Let { name, value, span: self.sp(start) });
        }
        if self.is_ident_word("var") {
            self.advance();
            let name = self.expect_ident()?;
            if self.eat_op(":") {
                self.skip_type_ignored()?;
            }
            let value = if self.eat_op("=") { Some(self.parse_expr()?) } else { None };
            return Ok(Stmt::Var { name, value, span: self.sp(start) });
        }
        if self.is_kw("if") {
            return self.parse_if();
        }
        if self.is_kw("for") {
            return self.parse_for();
        }
        if self.is_ident_word("while") {
            self.advance();
            let cond = self.parse_cond_expr()?;
            self.expect_op("{")?;
            let body = self.parse_stmts_until_close()?;
            self.expect_op("}")?;
            return Ok(Stmt::While { cond, body, span: self.sp(start) });
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
        for (tok, op) in [("+=", "+"), ("-=", "-"), ("*=", "*"), ("/=", "/")] {
            if self.eat_op(tok) {
                let rhs = self.parse_expr()?;
                let value = Expr::BinOp { op: op.to_string(), left: Box::new(e.clone()), right: Box::new(rhs), span: self.sp(start) };
                return Ok(Stmt::Assign { target: e, value, span: self.sp(start) });
            }
        }
        Ok(Stmt::Expr(e))
    }

    fn parse_if(&mut self) -> OResult<Stmt> {
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

    /// `for i in 0..N { ... }` — parsed generally (`hi` is any expression),
    /// but the *interpreter* rejects a non-literal `hi` with a clear error;
    /// see `ast.rs`'s doc comment for why that check belongs there.
    ///
    /// Also accepts two WGSL/Rust-dialect forms sharing the `for` keyword:
    /// `for i in range(..)` (an arbitrary iterable call, not a `LO..HI`
    /// range — `hi` becomes that whole call expression, evaluated as an
    /// ordinary value, not specially) and the C-style three-clause
    /// `for (init; cond; step) { .. }` (see `Stmt::CFor`).
    fn parse_for(&mut self) -> OResult<Stmt> {
        let start = self.start();
        self.expect_kw("for")?;
        if self.is_op("(") {
            return self.parse_c_style_for(start);
        }
        let var = self.expect_ident()?;
        self.expect_kw("in")?;
        // Lower bound is always 0 in this bootstrap's range-for and is
        // simply discarded — parsed via `parse_atom` (no postfix chaining),
        // NOT `parse_expr`, because `parse_expr` would greedily consume the
        // first '.' of '..' as swizzle/attribute-access on the literal.
        // Guarded the same way as `parse_cond_expr` (struct-lit disabled)
        // since a non-range iterable (see the `else` branch below) can be
        // a bare identifier immediately followed by the loop's own `{`.
        let prev = self.allow_struct_lit;
        self.allow_struct_lit = false;
        let lo = self.parse_atom();
        self.allow_struct_lit = prev;
        let lo = lo?;
        let hi = if self.eat_op("..") {
            self.parse_cond_expr()?
        } else {
            // Not a `LO..HI` range at all — e.g. `for i in range(lane, n, 256u)`.
            // `lo` (already a full call expression, since `parse_atom`
            // parses a bare-Ident-with-`(args)` as a `Call`) *is* the whole
            // iterable; kept as `hi` since this bootstrap only ever
            // inspects `hi`'s literal-ness, never actually iterates a
          // dynamic range for real (this construct only appears in
            // parsed-then-discarded `pipeline`/`shader` code blocks).
            lo
        };
        self.expect_op("{")?;
        let body = self.parse_stmts_until_close()?;
        self.expect_op("}")?;
        Ok(Stmt::For { var, hi, body, span: self.sp(start) })
    }

    /// `for (init; cond; step) { .. }`.
    fn parse_c_style_for(&mut self, start: Pos) -> OResult<Stmt> {
        self.expect_op("(")?;
        let init = Box::new(self.parse_stmt()?);
        self.eat_op(";");
        let cond = self.parse_cond_expr()?;
        self.expect_op(";")?;
        let step = Box::new(self.parse_stmt()?);
        self.expect_op(")")?;
        self.expect_op("{")?;
        let body = self.parse_stmts_until_close()?;
        self.expect_op("}")?;
        Ok(Stmt::CFor { init, cond, step, body, span: self.sp(start) })
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
        let mut left = self.parse_bitwise()?;
        while self.eat_op("&&") {
            let right = self.parse_bitwise()?;
            left = Expr::BinOp { op: "and".to_string(), left: Box::new(left), right: Box::new(right), span: self.sp(start) };
        }
        Ok(left)
    }

    /// Bitwise `|`/`^`/`&` and shift `<<`/`>>` — the Rust-dialect
    /// GPU-binding glue code (`gpu::BufferUsage::Storage | ..::MapRead`,
    /// `(BASIS ^ byte).wrapping_mul(..)`, `byte << 8`). Precedence among
    /// these doesn't need to be exactly right (this bootstrap never
    /// actually evaluates them meaningfully — see `interp.rs`), so they
    /// share one level rather than the usual four separate C-precedence
    /// tiers.
    fn parse_bitwise(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut left = self.parse_comparison()?;
        loop {
            if self.is_op("|") || self.is_op("^") || self.is_op("&") || self.is_op("<<") || self.is_op(">>") {
                let op = self.advance().value;
                let right = self.parse_comparison()?;
                left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), span: self.sp(start) };
            } else {
                break;
            }
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
        // Rust reference-taking (`&packed`, `&mut x`) — a no-op here (this
        // bootstrap has no reference/pointer values); safe unconditionally
        // at this fresh-operand position.
        if self.eat_op("&") {
            self.eat_ident_word("mut");
            return self.parse_unary();
        }
        self.parse_cast()
    }

    /// `expr as Type` — a no-op cast (this bootstrap's numeric model is
    /// uniformly f64/i64, not width-typed).
    fn parse_cast(&mut self) -> OResult<Expr> {
        let e = self.parse_postfix()?;
        while self.is_ident_word("as") {
            self.advance();
            self.skip_type_ignored()?;
        }
        Ok(e)
    }

    fn parse_postfix(&mut self) -> OResult<Expr> {
        let start = self.start();
        let mut e = self.parse_atom()?;
        loop {
            if self.eat_op(".") {
                let name = self.expect_ident()?;
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
                    e = Expr::MethodCall { obj: Box::new(e), method: name, args, span: self.sp(start) };
                } else {
                    e = Expr::Attr { obj: Box::new(e), name, span: self.sp(start) };
                }
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
            // Reserved words also used as bare values elsewhere in the
            // WGSL/design-token dialect: `in` (`in.position`, WGSL's
            // implicit shader-stage-IO struct) and `compute` (`target:
            // compute`, a pipeline's kind, distinct from `shader compute
            // Name`). Unambiguous here since every native-syntax use of
            // these words is consumed at its own dedicated call site
            // (`expect_kw("in")`/`eat_kw("compute")`) before expression
            // parsing is ever reached.
            TokKind::Keyword if t.value == "in" || t.value == "compute" => {
                self.advance();
                return Ok(Expr::Ident { name: t.value, span: self.sp(start) });
            }
            TokKind::Ident => {
                self.advance();
                // Rust-dialect namespaced path (`gpu::create_buffer`,
                // `EmbeddingDotProduct::create`) folded into one atomic
                // identifier — `::` lexes as a single token (see
                // `lexer.rs`), so this never conflicts with a plain `:` in
                // `name: type`/`name: value` position.
                let mut name = t.value;
                while self.eat_op("::") {
                    name.push_str("::");
                    name.push_str(&self.expect_ident()?);
                }
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
                    return Ok(Expr::Call { func: name, args, span: self.sp(start) });
                }
                // Rust struct literal (`GpuHarnessContext { field: value, .. }`)
                // — guarded by `allow_struct_lit` so an `if`/`while`/`for`
                // condition or range bound ending in a bare identifier
                // doesn't swallow the construct's own block-opening `{`.
                if self.allow_struct_lit && self.is_op("{") {
                    return self.parse_brace_field_values(start);
                }
                return Ok(Expr::Ident { name, span: self.sp(start) });
            }
            _ => {}
        }
        // Anonymous `struct { field: value, .. }` literal (used for inline
        // uniform-buffer params, e.g. `let params = struct { n: 1u32 }`) —
        // the `struct` keyword itself is just skipped; the brace-bodied
        // field-value list is identical to a named struct literal's.
        if self.is_ident_word("struct") {
            self.advance();
            return self.parse_brace_field_values(start);
        }
        // Rust closure (`|c| expr` / `|c| { stmts }` / `|| expr`) — safe to
        // dual-purpose `|`/`||` here since `parse_atom` is only ever
        // reached at a fresh-operand position, never where a bitwise/
        // logical-or infix operator would appear.
        if self.is_op("|") || self.is_op("||") {
            return self.parse_closure(start);
        }
        // `if cond { expr } else { expr }` used as an expression (e.g. the
        // RHS of a `let`/assignment), distinct from `Stmt::If` (a
        // statement, handled by `parse_if`, which is always checked first
        // at statement-start position and so never reaches here).
        if self.is_kw("if") {
            return self.parse_if_expr(start);
        }
        if self.eat_op("(") {
            let e = self.parse_expr()?;
            self.expect_op(")")?;
            return Ok(e);
        }
        // Bare `[a, b, c]` array literal (`workgroup_size: [16, 16, 1]`,
        // `gpu::create_bind_group(&[..])`).
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
        // Bare `{ name: value, .. }` map literal (only ever reached in a
        // known-safe, non-condition value position, e.g.
        // `attachments { color: { format: .., blend: .. } }`'s inner
        // value) — same field-value shape as a struct literal.
        if self.is_op("{") {
            return self.parse_brace_field_values(start);
        }
        Err(self.err(format!("expected an expression but found '{}'", self.cur_desc())))
    }

    /// `{ name: value, .. }` — shared by named struct literals, the
    /// anonymous `struct { .. }` form, and bare nested-map property
    /// values; field names are discarded, only the values are kept (same
    /// modeling tradeoff used by the other bootstraps' struct-literal
    /// support this session).
    fn parse_brace_field_values(&mut self, start: Pos) -> OResult<Expr> {
        self.expect_op("{")?;
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
        Ok(Expr::List { elems, span: self.sp(start) })
    }

    fn parse_closure(&mut self, start: Pos) -> OResult<Expr> {
        let params = if self.eat_op("||") {
            Vec::new()
        } else {
            self.expect_op("|")?;
            let mut params = Vec::new();
            while !self.is_op("|") && !self.at_eof() {
                params.push(self.expect_ident()?);
                if !self.eat_op(",") {
                    break;
                }
            }
            self.expect_op("|")?;
            params
        };
        let body = if self.eat_op("{") {
            let stmts = self.parse_stmts_until_close()?;
            self.expect_op("}")?;
            let bspan = self.sp(start);
            stmts
                .into_iter()
                .rev()
                .find_map(|s| match s {
                    Stmt::Expr(e) => Some(e),
                    Stmt::Return { value, .. } => value,
                    _ => None,
                })
                .unwrap_or(Expr::Int { v: 0, span: bspan })
        } else {
            self.parse_expr()?
        };
        Ok(Expr::Lambda { params, body: Box::new(body), span: self.sp(start) })
    }

    fn parse_if_expr(&mut self, start: Pos) -> OResult<Expr> {
        self.expect_kw("if")?;
        let cond = self.parse_cond_expr()?;
        let then_ = self.parse_brace_trailing_expr()?;
        self.expect_kw("else")?;
        let orelse = if self.is_kw("if") {
            self.parse_if_expr(self.start())?
        } else {
            self.parse_brace_trailing_expr()?
        };
        Ok(Expr::IfExpr { cond: Box::new(cond), then_: Box::new(then_), orelse: Box::new(orelse), span: self.sp(start) })
    }

    /// `{ stmts }`, reduced to its trailing expression (or `0` if the
    /// block ends in a non-expression statement) — the `if`-as-expression
    /// and closure-block-body shared shape.
    fn parse_brace_trailing_expr(&mut self) -> OResult<Expr> {
        let bstart = self.start();
        self.expect_op("{")?;
        let stmts = self.parse_stmts_until_close()?;
        self.expect_op("}")?;
        let bspan = self.sp(bstart);
        Ok(stmts
            .into_iter()
            .rev()
            .find_map(|s| match s {
                Stmt::Expr(e) => Some(e),
                Stmt::Return { value, .. } => value,
                _ => None,
            })
            .unwrap_or(Expr::Int { v: 0, span: bspan }))
    }
}
