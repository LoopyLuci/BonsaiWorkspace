//! Titan parser — recursive descent for items/statements/patterns/types and a
//! Pratt (precedence-climbing) expression parser.
//!
//! Titan treats `;` as optional; statements separate at expression boundaries.
//! Nested generic closers (`>>`) are split in place. Struct literals are
//! disallowed directly in `if`/`while`/`for`/`match` heads.

use crate::ast::*;
use crate::diag::{OResult, OmniError, Phase, Pos, Span};
use crate::lexer::{Lexer, TokKind, Token};

fn bin_prec(op: &str) -> Option<u8> {
    Some(match op {
        "||" => 1,
        "&&" => 2,
        "==" | "!=" | "<" | ">" | "<=" | ">=" => 3,
        "|" => 4,
        "^" => 5,
        "&" => 6,
        "<<" | ">>" => 7,
        "+" | "-" => 8,
        "*" | "/" | "%" => 9,
        _ => return None,
    })
}

const ASSIGN_OPS: &[&str] = &["=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>="];

pub struct Parser<'a> {
    toks: Vec<Token>,
    p: usize,
    file: &'a str,
    struct_lit_ok: bool,
    debug: bool,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str, file: &'a str) -> OResult<Self> {
        let toks = Lexer::new(src, file).tokenize()?;
        Ok(Parser { toks, p: 0, file, struct_lit_ok: true, debug: std::env::var("OMNI_DEBUG").is_ok() })
    }

    // ── token helpers ────────────────────────────────────────────────────────
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
    fn is(&self, v: &str) -> bool {
        let t = self.cur();
        matches!(t.kind, TokKind::Op | TokKind::Keyword) && t.value == v
    }
    fn is_kw(&self, v: &str) -> bool {
        let t = self.cur();
        t.kind == TokKind::Keyword && t.value == v
    }
    fn eat(&mut self, v: &str) -> bool {
        if self.is(v) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn expect(&mut self, v: &str) -> OResult<Token> {
        if self.is(v) {
            Ok(self.advance())
        } else {
            Err(self.err(format!("expected '{v}' but found '{}'", self.cur_desc()), self.cur().span))
        }
    }
    fn cur_desc(&self) -> String {
        let t = self.cur();
        if t.value.is_empty() { format!("{:?}", t.kind).to_lowercase() } else { t.value.clone() }
    }
    fn expect_ident(&mut self) -> OResult<String> {
        let t = self.cur().clone();
        if t.kind == TokKind::Ident || (t.kind == TokKind::Keyword && (t.value == "self" || t.value == "Self")) {
            self.advance();
            Ok(t.value)
        } else {
            Err(self.err(format!("expected identifier but found '{}'", self.cur_desc()), t.span))
        }
    }
    fn err(&self, msg: impl Into<String>, span: Span) -> OmniError {
        OmniError::new(Phase::Parse, msg, span, self.file)
    }
    fn start(&self) -> Pos {
        self.cur().span.start
    }
    fn sp(&self, start: Pos) -> Span {
        let prev = &self.toks[self.p.saturating_sub(1).min(self.toks.len() - 1)];
        Span { start, end: prev.span.end }
    }
    fn skip_semis(&mut self) {
        while self.eat(";") {}
    }

    // Angle-close handling: `>>`/`>=`/`>>=` may begin with a generic closer.
    fn is_gt(&self) -> bool {
        let t = self.cur();
        t.kind == TokKind::Op && matches!(t.value.as_str(), ">" | ">>" | ">=" | ">>=")
    }
    fn expect_gt(&mut self) -> OResult<()> {
        let (kind, val, span) = { let t = self.cur(); (t.kind, t.value.clone(), t.span) };
        if kind == TokKind::Op && val == ">" {
            self.advance();
            return Ok(());
        }
        if kind == TokKind::Op && matches!(val.as_str(), ">>" | ">=" | ">>=") {
            self.toks[self.p].value = val[1..].to_string();
            return Ok(());
        }
        Err(self.err(format!("expected '>' but found '{}'", self.cur_desc()), span))
    }

    // ── program & items ──────────────────────────────────────────────────────
    pub fn parse_program(&mut self) -> OResult<Program> {
        let mut items = Vec::new();
        self.skip_semis();
        while !self.at_eof() {
            let it = self.parse_item()?;
            if self.debug {
                eprintln!("item: {}", item_name(&it));
            }
            items.push(it);
            self.skip_semis();
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> OResult<Item> {
        self.skip_attributes()?; // #[derive(...)] precedes `pub`
        let pub_ = self.eat("pub");
        if pub_ && self.is("(") {
            self.skip_balanced("(", ")")?;
        }
        self.skip_attributes()?;

        // `actor Name { fields; message M(..) -> T { body } }` — Titan's actor
        // construct. Modeled as a struct plus an impl whose methods are the
        // message handlers (each takes &self implicitly).
        if self.cur().kind == TokKind::Ident && self.cur().value == "actor" && self.next_tok().kind == TokKind::Ident {
            return self.parse_actor();
        }

        if self.is_kw("use") {
            return self.parse_use();
        }
        if self.is_kw("struct") {
            return self.parse_struct();
        }
        if self.is_kw("enum") {
            return self.parse_enum();
        }
        if self.is_kw("impl") {
            return self.parse_impl();
        }
        if self.is_kw("trait") {
            return self.parse_trait();
        }
        if self.is_kw("fn") || self.is_kw("async") {
            return Ok(Item::Fn(self.parse_fn()?));
        }
        if self.is_kw("const") || self.is_kw("static") {
            return Ok(Item::Const(self.parse_const()?));
        }
        if self.is_kw("mod") {
            return self.parse_mod();
        }
        if self.is_kw("type") {
            self.advance();
            self.expect_ident()?;
            if self.is("<") {
                self.skip_balanced("<", ">")?;
            }
            self.expect("=")?;
            self.parse_type()?;
            self.eat(";");
            return Ok(Item::Use);
        }
        Err(self.err(
            format!("expected an item (fn/struct/enum/impl/use/...) but found '{}'", self.cur_desc()),
            self.cur().span,
        ))
    }

    /// `actor Name { field: T, message M(p: T) -> R { .. }, fn f(..) { .. } }`
    /// Lowered to a struct (the fields) + an impl (messages and fns as methods).
    fn parse_actor(&mut self) -> OResult<Item> {
        let start = self.start();
        self.advance(); // `actor`
        let name = self.expect_ident()?;
        self.parse_generics()?;
        self.expect("{")?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        self.skip_semis();
        while !self.is("}") && !self.at_eof() {
            self.skip_attributes()?;
            self.eat("pub");
            if self.cur().kind == TokKind::Ident && (self.cur().value == "message" || self.cur().value == "handler")
                && self.next_tok().kind == TokKind::Ident
            {
                self.advance(); // message/handler
                let mstart = self.start();
                let mname = self.expect_ident()?;
                self.expect("(")?;
                let mut params = Vec::new();
                while !self.is(")") && !self.at_eof() {
                    params.push(self.parse_param()?);
                    if !self.eat(",") {
                        break;
                    }
                }
                self.expect(")")?;
                let ret = if self.eat("->") {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                let body = Some(self.parse_block()?);
                methods.push(FnItem { name: mname, params, body, ret, span: self.sp(mstart) });
            } else if self.is_kw("fn") || self.is_kw("async") {
                methods.push(self.parse_fn()?);
            } else if self.cur().kind == TokKind::Ident && self.cur().value == "on" {
                // lifecycle hook: on start { .. }
                self.advance();
                let hstart = self.start();
                let hname = format!("on_{}", self.expect_ident()?);
                let body = Some(self.parse_block()?);
                methods.push(FnItem { name: hname, params: vec![], body, ret: None, span: self.sp(hstart) });
            } else {
                // field: name: Type [= default]
                let fname = self.expect_ident()?;
                self.expect(":")?;
                self.parse_type()?;
                if self.eat("=") {
                    self.parse_expr()?;
                }
                fields.push(Field { name: fname });
                self.eat(",");
            }
            self.skip_semis();
        }
        self.expect("}")?;
        let span = self.sp(start);
        Ok(Item::Mod(ModItem {
            name: name.clone(),
            items: vec![
                Item::Struct(StructItem { name: name.clone(), fields, span }),
                Item::Impl(ImplItem { target: name, methods, consts: vec![], span }),
            ],
        }))
    }

    fn skip_attributes(&mut self) -> OResult<()> {
        while self.is("#") {
            self.advance();
            self.eat("!");
            if self.is("[") {
                self.skip_balanced("[", "]")?;
            }
        }
        Ok(())
    }

    fn skip_balanced(&mut self, open: &str, close: &str) -> OResult<()> {
        self.expect(open)?;
        let mut depth = 1;
        while depth > 0 && !self.at_eof() {
            if self.is(open) {
                depth += 1;
            } else if self.is(close) {
                depth -= 1;
            }
            self.advance();
        }
        Ok(())
    }

    fn parse_use(&mut self) -> OResult<Item> {
        self.expect("use")?;
        self.expect_ident()?;
        while self.eat("::") {
            if self.is("{") {
                self.advance();
                while !self.is("}") && !self.at_eof() {
                    self.expect_ident()?;
                    if self.eat("as") {
                        self.expect_ident()?;
                    }
                    if !self.eat(",") {
                        break;
                    }
                }
                self.expect("}")?;
                break;
            } else if self.is("*") {
                self.advance();
                break;
            } else {
                self.expect_ident()?;
            }
        }
        self.eat(";");
        Ok(Item::Use)
    }

    fn parse_generics(&mut self) -> OResult<()> {
        if !self.is("<") {
            return Ok(());
        }
        self.advance();
        while !self.is_gt() && !self.at_eof() {
            if self.cur().kind == TokKind::Lifetime {
                self.advance();
            } else {
                self.eat("const");
                self.expect_ident()?;
            }
            if self.eat(":") {
                self.parse_bounds()?;
            }
            if self.eat("=") {
                self.parse_type()?;
            }
            if !self.eat(",") {
                break;
            }
        }
        self.expect_gt()
    }

    fn parse_bounds(&mut self) -> OResult<()> {
        loop {
            if self.cur().kind == TokKind::Lifetime {
                self.advance();
            } else {
                self.parse_type()?;
            }
            if !self.eat("+") {
                return Ok(());
            }
        }
    }

    fn parse_where(&mut self) -> OResult<()> {
        if !self.is_kw("where") {
            return Ok(());
        }
        self.advance();
        while !self.is("{") && !self.is(";") && !self.at_eof() {
            self.advance();
        }
        Ok(())
    }

    fn parse_struct(&mut self) -> OResult<Item> {
        let start = self.start();
        self.expect("struct")?;
        let name = self.expect_ident()?;
        self.parse_generics()?;
        self.parse_where()?;
        let mut fields = Vec::new();
        if self.eat("(") {
            while !self.is(")") && !self.at_eof() {
                self.eat("pub");
                self.parse_type()?;
                if !self.eat(",") {
                    break;
                }
            }
            self.expect(")")?;
            self.eat(";");
        } else if self.eat("{") {
            while !self.is("}") && !self.at_eof() {
                self.skip_attributes()?;
                let fpub = self.eat("pub");
                if fpub && self.is("(") {
                    self.skip_balanced("(", ")")?;
                }
                let fname = self.expect_ident()?;
                self.expect(":")?;
                self.parse_type()?;
                if self.eat("=") {
                    self.parse_expr()?; // field default value
                }
                fields.push(Field { name: fname });
                if !self.eat(",") {
                    break;
                }
            }
            self.expect("}")?;
        } else {
            self.eat(";");
        }
        Ok(Item::Struct(StructItem { name, fields, span: self.sp(start) }))
    }

    fn parse_enum(&mut self) -> OResult<Item> {
        let start = self.start();
        self.expect("enum")?;
        let name = self.expect_ident()?;
        self.parse_generics()?;
        self.parse_where()?;
        self.expect("{")?;
        let mut variants = Vec::new();
        while !self.is("}") && !self.at_eof() {
            self.skip_attributes()?;
            let vname = self.expect_ident()?;
            let mut arity = 0;
            let mut struct_fields = Vec::new();
            if self.eat("(") {
                while !self.is(")") && !self.at_eof() {
                    self.parse_type()?;
                    arity += 1;
                    if !self.eat(",") {
                        break;
                    }
                }
                self.expect(")")?;
            } else if self.eat("{") {
                while !self.is("}") && !self.at_eof() {
                    let fname = self.expect_ident()?;
                    self.expect(":")?;
                    self.parse_type()?;
                    struct_fields.push(fname);
                    if !self.eat(",") {
                        break;
                    }
                }
                self.expect("}")?;
            } else if self.eat("=") {
                self.parse_expr()?;
            }
            variants.push(EnumVariant { name: vname, arity, struct_fields });
            if !self.eat(",") {
                break;
            }
        }
        self.expect("}")?;
        Ok(Item::Enum(EnumItem { name, variants, span: self.sp(start) }))
    }

    fn parse_impl(&mut self) -> OResult<Item> {
        let start = self.start();
        self.expect("impl")?;
        self.parse_generics()?;
        let first = self.parse_type()?;
        let mut target = first.name.clone();
        if self.eat("for") {
            target = self.parse_type()?.name;
        }
        self.parse_where()?;
        self.expect("{")?;
        let mut methods = Vec::new();
        let mut consts = Vec::new();
        self.skip_semis();
        while !self.is("}") && !self.at_eof() {
            self.skip_attributes()?;
            let mpub = self.eat("pub");
            if mpub && self.is("(") {
                self.skip_balanced("(", ")")?;
            }
            if self.is_kw("const") || self.is_kw("static") {
                consts.push(self.parse_const()?);
                self.skip_semis();
                continue;
            }
            if self.is_kw("type") {
                self.advance();
                self.expect_ident()?;
                if self.is("<") {
                    self.skip_balanced("<", ">")?;
                }
                if self.eat("=") {
                    self.parse_type()?;
                }
                self.eat(";");
                continue;
            }
            methods.push(self.parse_fn()?);
            self.skip_semis();
        }
        self.expect("}")?;
        Ok(Item::Impl(ImplItem { target, methods, consts, span: self.sp(start) }))
    }

    fn parse_trait(&mut self) -> OResult<Item> {
        self.expect("trait")?;
        let name = self.expect_ident()?;
        self.parse_generics()?;
        if self.eat(":") {
            self.parse_bounds()?;
        }
        self.parse_where()?;
        self.expect("{")?;
        let mut methods = Vec::new();
        while !self.is("}") && !self.at_eof() {
            self.skip_attributes()?;
            self.eat("pub");
            if self.is_kw("type") || self.is_kw("const") {
                while !self.is(";") && !self.is("}") && !self.at_eof() {
                    self.advance();
                }
                self.eat(";");
                continue;
            }
            methods.push(self.parse_fn()?);
            self.skip_semis();
        }
        self.expect("}")?;
        Ok(Item::Trait(TraitItem { name, methods }))
    }

    fn parse_fn(&mut self) -> OResult<FnItem> {
        let start = self.start();
        self.eat("async");
        self.expect("fn")?;
        let name = self.expect_ident()?;
        self.parse_generics()?;
        self.expect("(")?;
        let mut params = Vec::new();
        while !self.is(")") && !self.at_eof() {
            params.push(self.parse_param()?);
            if !self.eat(",") {
                break;
            }
        }
        self.expect(")")?;
        let ret = if self.eat("->") {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.parse_where()?;
        let body = if self.is("{") {
            Some(self.parse_block()?)
        } else {
            self.eat(";");
            None
        };
        Ok(FnItem { name, params, body, ret, span: self.sp(start) })
    }

    fn parse_param(&mut self) -> OResult<Param> {
        self.skip_attributes()?;
        if self.is("&") {
            self.advance();
            if self.cur().kind == TokKind::Lifetime {
                self.advance();
            }
            self.eat("mut");
        }
        if self.is_kw("self") {
            self.advance();
            return Ok(Param { name: "self".into(), is_self: true });
        }
        self.eat("mut");
        let name = self.expect_ident()?;
        if self.eat(":") {
            self.parse_type()?;
        }
        Ok(Param { name, is_self: false })
    }

    fn parse_const(&mut self) -> OResult<ConstItem> {
        let start = self.start();
        self.advance(); // const / static
        self.eat("mut");
        let name = self.expect_ident()?;
        if self.eat(":") {
            self.parse_type()?;
        }
        self.expect("=")?;
        let value = self.parse_expr()?;
        self.eat(";");
        Ok(ConstItem { name, value, span: self.sp(start) })
    }

    fn parse_mod(&mut self) -> OResult<Item> {
        self.expect("mod")?;
        let name = self.expect_ident()?;
        let mut items = Vec::new();
        if self.eat("{") {
            self.skip_semis();
            while !self.is("}") && !self.at_eof() {
                items.push(self.parse_item()?);
                self.skip_semis();
            }
            self.expect("}")?;
        } else {
            self.eat(";");
        }
        Ok(Item::Mod(ModItem { name, items }))
    }

    // ── types ────────────────────────────────────────────────────────────────
    fn parse_type(&mut self) -> OResult<TypeRef> {
        while self.is("&") {
            self.advance();
            if self.cur().kind == TokKind::Lifetime {
                self.advance();
            }
            self.eat("mut");
        }
        // raw pointer: *mut T / *const T
        if self.is("*") {
            self.advance();
            self.eat("mut");
            self.eat("const");
            let inner = self.parse_type()?;
            return Ok(TypeRef { name: "ptr".into(), args: vec![inner] });
        }
        self.eat("dyn");
        if self.is("(") {
            self.advance();
            let mut args = Vec::new();
            while !self.is(")") && !self.at_eof() {
                args.push(self.parse_type()?);
                if !self.eat(",") {
                    break;
                }
            }
            self.expect(")")?;
            return Ok(TypeRef { name: "tuple".into(), args });
        }
        if self.is("[") {
            self.advance();
            let el = self.parse_type()?;
            if self.eat(";") {
                self.parse_expr()?;
            }
            self.expect("]")?;
            return Ok(TypeRef { name: "array".into(), args: vec![el] });
        }
        if self.is_kw("fn") {
            self.advance();
            if self.is("(") {
                self.skip_balanced("(", ")")?;
            }
            if self.eat("->") {
                self.parse_type()?;
            }
            return Ok(TypeRef { name: "fn".into(), args: vec![] });
        }
        if self.is_kw("impl") {
            self.advance();
        }
        let mut name = self.expect_ident()?;
        while self.eat("::") {
            name = self.expect_ident()?;
        }
        let mut args = Vec::new();
        if self.is("<") {
            self.advance();
            while !self.is_gt() && !self.at_eof() {
                if self.cur().kind == TokKind::Lifetime {
                    self.advance();
                } else if self.cur().kind == TokKind::Ident
                    && self.next_tok().kind == TokKind::Op
                    && self.next_tok().value == "="
                {
                    // associated type binding: Item = T
                    self.advance();
                    self.advance();
                    self.parse_type()?;
                } else {
                    args.push(self.parse_type()?);
                }
                if !self.eat(",") {
                    break;
                }
            }
            self.expect_gt()?;
        } else if self.is("(") {
            // Fn(A, B) -> C
            self.skip_balanced("(", ")")?;
            if self.eat("->") {
                self.parse_type()?;
            }
        }
        Ok(TypeRef { name, args })
    }

    // ── blocks & statements ─────────────────────────────────────────────────
    fn parse_block(&mut self) -> OResult<Block> {
        let start = self.start();
        self.expect("{")?;
        let mut stmts = Vec::new();
        self.skip_semis();
        while !self.is("}") && !self.at_eof() {
            stmts.push(self.parse_stmt()?);
            self.skip_semis();
        }
        self.expect("}")?;
        Ok(Block { stmts, span: self.sp(start) })
    }

    fn parse_stmt(&mut self) -> OResult<Stmt> {
        let start = self.start();
        if self.is_kw("let") {
            self.advance();
            let pat = self.parse_pattern()?;
            if self.eat(":") {
                self.parse_type()?;
            }
            let init = if self.eat("=") { Some(self.parse_expr()?) } else { None };
            if self.is_kw("else") {
                self.advance();
                self.parse_block()?;
            }
            self.eat(";");
            return Ok(Stmt::Let { pat, init, span: self.sp(start) });
        }
        if self.is_item_start() {
            let item = self.parse_item()?;
            return Ok(Stmt::Item(Box::new(item)));
        }
        let expr = self.parse_expr()?;
        let semi = self.eat(";");
        Ok(Stmt::Expr { expr, semi })
    }

    fn is_item_start(&self) -> bool {
        if self.is_kw("fn") || self.is_kw("struct") || self.is_kw("enum") || self.is_kw("impl")
            || self.is_kw("use") || self.is_kw("mod") || self.is_kw("trait") || self.is_kw("static")
        {
            return true;
        }
        if self.is_kw("const") {
            return self.next_tok().kind == TokKind::Ident;
        }
        self.is_kw("pub")
    }

    // ── patterns ─────────────────────────────────────────────────────────────
    /// True if the token after a range `..`/`..=` begins an upper-bound
    /// expression (literal or unary minus), vs. an open range where the next
    /// token ends the pattern (`=>`, `|`, `)`, `if`, etc.).
    fn range_pat_has_upper(&self) -> bool {
        let k = self.cur().kind;
        matches!(k, TokKind::Int | TokKind::Float | TokKind::Str | TokKind::Char | TokKind::Bool)
            || (k == TokKind::Op && self.cur().value == "-")
    }

    fn parse_pattern(&mut self) -> OResult<Pattern> {
        let first = self.parse_pattern_primary()?;
        if self.is("|") {
            let mut alts = vec![first];
            while self.eat("|") {
                alts.push(self.parse_pattern_primary()?);
            }
            return Ok(Pattern::Or(alts));
        }
        Ok(first)
    }

    fn parse_pattern_primary(&mut self) -> OResult<Pattern> {
        let start = self.start();
        if self.is("&") {
            self.advance();
            self.eat("mut");
            return Ok(Pattern::Ref(Box::new(self.parse_pattern_primary()?)));
        }
        if self.cur().kind == TokKind::Ident && self.cur().value == "_" {
            self.advance();
            return Ok(Pattern::Wild);
        }
        if self.is("(") {
            self.advance();
            let mut elems = Vec::new();
            while !self.is(")") && !self.at_eof() {
                elems.push(self.parse_pattern()?);
                if !self.eat(",") {
                    break;
                }
            }
            self.expect(")")?;
            return Ok(Pattern::Tuple(elems));
        }
        {
            let k = self.cur().kind;
            if matches!(k, TokKind::Int | TokKind::Float | TokKind::Str | TokKind::Char | TokKind::Bool)
                || (k == TokKind::Op && self.cur().value == "-")
            {
                let value = self.parse_unary()?;
                // Range pattern: `lo..hi` / `lo..=hi`.
                if self.is("..=") || self.is("..") {
                    let inclusive = self.eat("..=");
                    if !inclusive { self.expect("..")?; }
                    let hi = if self.range_pat_has_upper() {
                        Some(Box::new(self.parse_unary()?))
                    } else {
                        None
                    };
                    return Ok(Pattern::Range { lo: Some(Box::new(value)), hi, inclusive, span: self.sp(start) });
                }
                return Ok(Pattern::Lit(Box::new(value)));
            }
            // Open-low range pattern: `..=hi` / `..hi`.
            if self.is("..=") || self.is("..") {
                let inclusive = self.eat("..=");
                if !inclusive { self.expect("..")?; }
                let hi = Some(Box::new(self.parse_unary()?));
                return Ok(Pattern::Range { lo: None, hi, inclusive, span: self.sp(start) });
            }
        }
        if self.is_kw("mut") {
            self.advance();
            let name = self.expect_ident()?;
            return Ok(Pattern::Bind { name, span: self.sp(start) });
        }
        let mut path = vec![self.expect_ident()?];
        while self.eat("::") {
            path.push(self.expect_ident()?);
        }
        if self.is("(") {
            self.advance();
            let mut elems = Vec::new();
            while !self.is(")") && !self.at_eof() {
                elems.push(self.parse_pattern()?);
                if !self.eat(",") {
                    break;
                }
            }
            self.expect(")")?;
            return Ok(Pattern::Enum { path, elems, span: self.sp(start) });
        }
        if self.is("{") {
            self.advance();
            let mut fields = Vec::new();
            while !self.is("}") && !self.at_eof() {
                if self.eat("..") {
                    break;
                }
                let fname = self.expect_ident()?;
                let pat = if self.eat(":") {
                    self.parse_pattern()?
                } else {
                    Pattern::Bind { name: fname.clone(), span: self.cur().span }
                };
                fields.push((fname, pat));
                if !self.eat(",") {
                    break;
                }
            }
            self.expect("}")?;
            return Ok(Pattern::Struct { path, fields, span: self.sp(start) });
        }
        if path.len() == 1 && path[0].chars().next().is_some_and(|c| c.is_lowercase() || c == '_') {
            return Ok(Pattern::Bind { name: path.remove(0), span: self.sp(start) });
        }
        Ok(Pattern::Path { path, span: self.sp(start) })
    }

    // ── expressions (Pratt) ──────────────────────────────────────────────────
    pub fn parse_expr(&mut self) -> OResult<Expr> {
        self.parse_assign()
    }

    fn parse_assign(&mut self) -> OResult<Expr> {
        let start = self.start();
        let left = self.parse_range()?;
        let (kind, val) = { let t = self.cur(); (t.kind, t.value.clone()) };
        if kind == TokKind::Op && ASSIGN_OPS.contains(&val.as_str()) {
            self.advance();
            let value = self.parse_assign()?;
            return Ok(Expr::Assign { op: val, target: Box::new(left), value: Box::new(value), span: self.sp(start) });
        }
        Ok(left)
    }

    fn parse_range(&mut self) -> OResult<Expr> {
        let start = self.start();
        if self.is("..") || self.is("..=") {
            let inclusive = self.cur().value == "..=";
            self.advance();
            let to = if self.is_expr_start() { Some(Box::new(self.parse_binary(0)?)) } else { None };
            return Ok(Expr::Range { from: None, to, inclusive, span: self.sp(start) });
        }
        let left = self.parse_binary(0)?;
        if self.is("..") || self.is("..=") {
            let inclusive = self.cur().value == "..=";
            self.advance();
            let to = if self.is_expr_start() { Some(Box::new(self.parse_binary(0)?)) } else { None };
            return Ok(Expr::Range { from: Some(Box::new(left)), to, inclusive, span: self.sp(start) });
        }
        Ok(left)
    }

    fn parse_binary(&mut self, min_prec: u8) -> OResult<Expr> {
        let mut left = self.parse_cast()?;
        loop {
            let (kind, val) = { let t = self.cur(); (t.kind, t.value.clone()) };
            if kind != TokKind::Op {
                break;
            }
            let Some(prec) = bin_prec(&val) else { break };
            if prec < min_prec {
                break;
            }
            self.advance();
            let right = self.parse_binary(prec + 1)?;
            let span = Span { start: left.span().start, end: right.span().end };
            left = Expr::Binary { op: val, left: Box::new(left), right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_cast(&mut self) -> OResult<Expr> {
        let mut e = self.parse_unary()?;
        while self.is_kw("as") {
            self.advance();
            let ty = self.parse_type()?;
            let span = e.span();
            e = Expr::Cast { expr: Box::new(e), ty, span };
        }
        Ok(e)
    }

    fn parse_unary(&mut self) -> OResult<Expr> {
        let start = self.start();
        if self.is("-") || self.is("!") {
            let op = self.advance().value;
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary { op, operand: Box::new(operand), span: self.sp(start) });
        }
        if self.is("*") {
            self.advance();
            return self.parse_unary(); // deref is a no-op in the dynamic seed
        }
        if self.is("&") {
            self.advance();
            self.eat("mut");
            return self.parse_unary(); // ref is a no-op in the dynamic seed
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> OResult<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            if self.is(".") {
                self.advance();
                if self.cur().kind == TokKind::Int {
                    let idx = self.advance();
                    let span = Span { start: e.span().start, end: idx.span.end };
                    e = Expr::Field { obj: Box::new(e), name: idx.value, span };
                    continue;
                }
                if self.is_kw("await") {
                    self.advance();
                    continue;
                }
                let name = self.expect_ident()?;
                if self.is("::") {
                    self.advance();
                    self.expect("<")?;
                    self.skip_type_args();
                }
                if self.is("(") {
                    let args = self.parse_args()?;
                    let span = self.sp(e.span().start);
                    e = Expr::Method { recv: Box::new(e), name, args, span };
                } else {
                    let span = self.sp(e.span().start);
                    e = Expr::Field { obj: Box::new(e), name, span };
                }
            } else if self.is("(") {
                let args = self.parse_args()?;
                let span = self.sp(e.span().start);
                e = Expr::Call { callee: Box::new(e), args, span };
            } else if self.is("[") {
                self.advance();
                let index = self.parse_expr()?;
                self.expect("]")?;
                let span = self.sp(e.span().start);
                e = Expr::Index { obj: Box::new(e), index: Box::new(index), span };
            } else if self.is("?") {
                self.advance();
                let span = self.sp(e.span().start);
                e = Expr::Try { expr: Box::new(e), span };
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn skip_type_args(&mut self) {
        let mut depth = 1;
        while depth > 0 && !self.at_eof() {
            if self.is("<") {
                depth += 1;
            } else if self.is(">") {
                depth -= 1;
            } else if self.is(">>") {
                depth -= 2;
            }
            self.advance();
        }
    }

    fn parse_args(&mut self) -> OResult<Vec<Expr>> {
        self.expect("(")?;
        let mut args = Vec::new();
        while !self.is(")") && !self.at_eof() {
            args.push(self.parse_expr()?);
            if !self.eat(",") {
                break;
            }
        }
        self.expect(")")?;
        Ok(args)
    }

    fn is_expr_start(&self) -> bool {
        let t = self.cur();
        match t.kind {
            TokKind::Int | TokKind::Float | TokKind::Str | TokKind::Char | TokKind::Bool | TokKind::Ident => true,
            TokKind::Keyword => matches!(
                t.value.as_str(),
                "self" | "Self" | "if" | "match" | "while" | "for" | "loop" | "return" | "break" | "continue" | "move"
            ),
            TokKind::Op => matches!(t.value.as_str(), "(" | "[" | "{" | "-" | "!" | "*" | "&" | "|" | "||" | ".." | "..="),
            _ => false,
        }
    }

    fn no_struct<T>(&mut self, f: impl FnOnce(&mut Self) -> OResult<T>) -> OResult<T> {
        let save = self.struct_lit_ok;
        self.struct_lit_ok = false;
        let r = f(self);
        self.struct_lit_ok = save;
        r
    }

    fn parse_primary(&mut self) -> OResult<Expr> {
        let start = self.start();
        let t = self.cur().clone();

        match t.kind {
            TokKind::Int => {
                self.advance();
                let v: i64 = t.value.parse().unwrap_or(0);
                return Ok(Expr::Int { v, span: self.sp(start) });
            }
            TokKind::Float => {
                self.advance();
                let v: f64 = t.value.parse().unwrap_or(0.0);
                return Ok(Expr::Float { v, span: self.sp(start) });
            }
            TokKind::Str => {
                self.advance();
                return Ok(Expr::Str { v: t.value, span: self.sp(start) });
            }
            TokKind::Char => {
                self.advance();
                return Ok(Expr::Char { v: t.value.chars().next().unwrap_or('\0'), span: self.sp(start) });
            }
            TokKind::Bool => {
                self.advance();
                return Ok(Expr::Bool { v: t.value == "true", span: self.sp(start) });
            }
            _ => {}
        }

        if self.is_kw("if") {
            return self.parse_if();
        }
        if self.is_kw("match") {
            return self.parse_match();
        }
        // Optional loop label: `'outer: for … / while … / loop …`.
        if self.cur().kind == TokKind::Lifetime && self.next_tok().kind == TokKind::Op && self.next_tok().value == ":" {
            let label = self.advance().value; // 'outer
            self.expect(":")?;
            if self.is_kw("while") { return self.parse_while(Some(label)); }
            if self.is_kw("for") { return self.parse_for(Some(label)); }
            if self.is_kw("loop") {
                self.advance();
                let body = self.parse_block()?;
                return Ok(Expr::Loop { body, label: Some(label), span: self.sp(start) });
            }
            return Err(self.err("loop label must be followed by `for`, `while`, or `loop`", self.cur().span));
        }
        if self.is_kw("while") {
            return self.parse_while(None);
        }
        if self.is_kw("for") {
            return self.parse_for(None);
        }
        if self.is_kw("loop") {
            self.advance();
            let body = self.parse_block()?;
            return Ok(Expr::Loop { body, label: None, span: self.sp(start) });
        }
        if self.is("{") {
            let block = self.parse_block()?;
            return Ok(Expr::BlockE { block });
        }
        if self.is_kw("return") {
            self.advance();
            let value = if self.is_expr_start() { Some(Box::new(self.parse_expr()?)) } else { None };
            return Ok(Expr::Return { value, span: self.sp(start) });
        }
        if self.is_kw("break") {
            self.advance();
            let label = if self.cur().kind == TokKind::Lifetime { Some(self.advance().value) } else { None };
            let value = if self.is_expr_start() { Some(Box::new(self.parse_expr()?)) } else { None };
            return Ok(Expr::Break { value, label, span: self.sp(start) });
        }
        if self.is_kw("continue") {
            self.advance();
            let label = if self.cur().kind == TokKind::Lifetime { Some(self.advance().value) } else { None };
            return Ok(Expr::Continue { label, span: self.sp(start) });
        }
        if self.is_kw("move") {
            self.advance();
            return self.parse_closure(start);
        }
        if self.is("|") || self.is("||") {
            return self.parse_closure(start);
        }

        if self.is("(") {
            self.advance();
            if self.is(")") {
                self.advance();
                return Ok(Expr::Tuple { elems: vec![], span: self.sp(start) });
            }
            let first = self.parse_expr()?;
            if self.is(",") {
                let mut elems = vec![first];
                while self.eat(",") {
                    if self.is(")") {
                        break;
                    }
                    elems.push(self.parse_expr()?);
                }
                self.expect(")")?;
                return Ok(Expr::Tuple { elems, span: self.sp(start) });
            }
            self.expect(")")?;
            return Ok(first);
        }

        if self.is("[") {
            self.advance();
            let mut elems = Vec::new();
            let mut repeat = None;
            if !self.is("]") {
                let first = self.parse_expr()?;
                if self.eat(";") {
                    elems.push(first);
                    repeat = Some(Box::new(self.parse_expr()?));
                } else {
                    elems.push(first);
                    while self.eat(",") {
                        if self.is("]") {
                            break;
                        }
                        elems.push(self.parse_expr()?);
                    }
                }
            }
            self.expect("]")?;
            return Ok(Expr::Array { elems, repeat, span: self.sp(start) });
        }

        if t.kind == TokKind::Ident || (t.kind == TokKind::Keyword && (t.value == "self" || t.value == "Self")) {
            let mut segs = vec![self.advance().value];
            if self.is("!") {
                self.advance();
                return self.parse_macro(segs.remove(0), start);
            }
            while self.is("::") {
                self.advance();
                if self.is("<") {
                    self.advance();
                    self.skip_type_args();
                    continue;
                }
                segs.push(self.expect_ident()?);
            }
            if self.is("{") && self.struct_lit_ok {
                return self.parse_struct_lit(segs, start);
            }
            return Ok(Expr::Path { segs, span: self.sp(start) });
        }

        Err(self.err(format!("expected an expression but found '{}'", self.cur_desc()), t.span))
    }

    fn parse_struct_lit(&mut self, path: Vec<String>, start: Pos) -> OResult<Expr> {
        self.expect("{")?;
        let mut fields = Vec::new();
        let mut spread = None;
        while !self.is("}") && !self.at_eof() {
            if self.eat("..") {
                spread = Some(Box::new(self.parse_expr()?));
                break;
            }
            let name = self.expect_ident()?;
            let value = if self.eat(":") {
                self.parse_expr()?
            } else {
                Expr::Path { segs: vec![name.clone()], span: self.cur().span }
            };
            fields.push((name, value));
            if !self.eat(",") {
                break;
            }
        }
        self.expect("}")?;
        Ok(Expr::StructLit { path, fields, spread, span: self.sp(start) })
    }

    fn parse_macro(&mut self, name: String, start: Pos) -> OResult<Expr> {
        let (open, close) = if self.is("[") {
            ("[", "]")
        } else if self.is("{") {
            ("{", "}")
        } else {
            ("(", ")")
        };
        self.expect(open)?;
        let mut args = Vec::new();
        let mut repeat = None;
        if !self.is(close) {
            args.push(self.parse_expr()?);
            if self.eat(";") {
                repeat = Some(Box::new(self.parse_expr()?));
            } else {
                while self.eat(",") {
                    if self.is(close) {
                        break;
                    }
                    args.push(self.parse_expr()?);
                }
            }
        }
        self.expect(close)?;
        Ok(Expr::Macro { name, args, repeat, span: self.sp(start) })
    }

    fn parse_closure(&mut self, start: Pos) -> OResult<Expr> {
        let mut params = Vec::new();
        if self.eat("||") {
            // no params
        } else {
            self.expect("|")?;
            while !self.is("|") && !self.at_eof() {
                self.eat("mut");
                self.eat("&");
                let name = self.expect_ident()?;
                if self.eat(":") {
                    self.parse_type()?;
                }
                params.push(name);
                if !self.eat(",") {
                    break;
                }
            }
            self.expect("|")?;
        }
        if self.eat("->") {
            self.parse_type()?;
        }
        let body = if self.is("{") {
            let block = self.parse_block()?;
            Expr::BlockE { block }
        } else {
            self.parse_expr()?
        };
        Ok(Expr::Closure { params, body: Box::new(body), span: self.sp(start) })
    }

    fn parse_if(&mut self) -> OResult<Expr> {
        let start = self.start();
        self.expect("if")?;
        let (let_pat, cond) = if self.eat("let") {
            let pat = self.parse_pattern()?;
            self.expect("=")?;
            let c = self.no_struct(|s| s.parse_expr())?;
            (Some(Box::new(pat)), c)
        } else {
            (None, self.no_struct(|s| s.parse_expr())?)
        };
        let then = self.parse_block()?;
        let els = if self.eat("else") {
            if self.is_kw("if") {
                Some(Box::new(self.parse_if()?))
            } else {
                let b = self.parse_block()?;
                Some(Box::new(Expr::BlockE { block: b }))
            }
        } else {
            None
        };
        Ok(Expr::If { let_pat, cond: Box::new(cond), then, els, span: self.sp(start) })
    }

    fn parse_match(&mut self) -> OResult<Expr> {
        let start = self.start();
        self.expect("match")?;
        let scrut = self.no_struct(|s| s.parse_expr())?;
        self.expect("{")?;
        let mut arms = Vec::new();
        while !self.is("}") && !self.at_eof() {
            let pat = self.parse_pattern()?;
            let guard = if self.eat("if") { Some(self.parse_expr()?) } else { None };
            self.expect("=>")?;
            let body = self.parse_expr()?;
            arms.push(MatchArm { pat, guard, body });
            self.eat(",");
        }
        self.expect("}")?;
        Ok(Expr::Match { scrut: Box::new(scrut), arms, span: self.sp(start) })
    }

    fn parse_while(&mut self, label: Option<String>) -> OResult<Expr> {
        let start = self.start();
        self.expect("while")?;
        let (let_pat, cond) = if self.eat("let") {
            let pat = self.parse_pattern()?;
            self.expect("=")?;
            let c = self.no_struct(|s| s.parse_expr())?;
            (Some(Box::new(pat)), c)
        } else {
            (None, self.no_struct(|s| s.parse_expr())?)
        };
        let body = self.parse_block()?;
        Ok(Expr::While { let_pat, cond: Box::new(cond), body, label, span: self.sp(start) })
    }

    fn parse_for(&mut self, label: Option<String>) -> OResult<Expr> {
        let start = self.start();
        self.expect("for")?;
        let pat = self.parse_pattern()?;
        self.expect("in")?;
        let iter = self.no_struct(|s| s.parse_expr())?;
        let body = self.parse_block()?;
        Ok(Expr::For { pat, iter: Box::new(iter), body, label, span: self.sp(start) })
    }
}

fn item_name(it: &Item) -> String {
    match it {
        Item::Use => "use".into(),
        Item::Struct(s) => format!("struct {}", s.name),
        Item::Enum(e) => format!("enum {}", e.name),
        Item::Impl(i) => format!("impl {}", i.target),
        Item::Trait(t) => format!("trait {}", t.name),
        Item::Fn(f) => format!("fn {}", f.name),
        Item::Const(c) => format!("const {}", c.name),
        Item::Mod(m) => format!("mod {}", m.name),
    }
}

pub fn parse(src: &str, file: &str) -> OResult<Program> {
    Parser::new(src, file)?.parse_program()
}
