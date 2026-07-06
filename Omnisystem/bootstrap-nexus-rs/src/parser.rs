//! Nexus parser — recursive descent over a purely declarative grammar (no
//! statements/control-flow/functions at all; see `ast.rs`).

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
    /// Some property names collide with keywords (`direction` isn't, but a
    /// user box could plausibly want a property literally named `in` etc.);
    /// property-name position accepts either an Ident or a Keyword token's
    /// text, since Nexus's keyword set is small and property names are
    /// otherwise unconstrained.
    fn expect_name(&mut self) -> OResult<String> {
        if matches!(self.cur().kind, TokKind::Ident | TokKind::Keyword) {
            Ok(self.advance().value)
        } else {
            Err(self.err(format!("expected a name but found '{}'", self.cur_desc())))
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
        let mut boxes = Vec::new();
        let mut layouts = Vec::new();
        let mut constraints = Vec::new();
        while !self.at_eof() {
            if self.is_kw("box") {
                boxes.push(self.parse_box()?);
            } else if self.is_kw("layout") {
                layouts.push(self.parse_layout()?);
            } else if self.is_kw("constrain") {
                constraints.push(self.parse_constraint()?);
            } else if self.cur().kind == TokKind::Ident
                && matches!(self.cur().value.as_str(), "tokens" | "breakpoints" | "scrollbar")
            {
                // The responsive-design-token dialect's flat declaration
                // blocks (`tokens NAME { .. }`, `breakpoints NAME { .. }`)
                // and its `hover`-capable `scrollbar NAME { .. }` block all
                // share the same name+body shape as a native `layout`, so
                // they're folded into `layouts` too — never actually
                // solved as real geometry (only `check`ed), just parsed.
                self.advance();
                layouts.push(self.parse_named_css_block()?);
            } else if self.cur().kind == TokKind::Ident && self.cur().value == "keyframes" {
                self.advance();
                self.parse_keyframes_block()?;
            } else {
                return Err(self.err(format!("expected 'box', 'layout', or 'constrain' but found '{}'", self.cur_desc())));
            }
        }
        Ok(Module { boxes, layouts, constraints })
    }

    fn parse_box(&mut self) -> OResult<BoxDef> {
        let start = self.start();
        self.expect_kw("box")?;
        let name = self.expect_ident()?;
        self.expect_op("{")?;
        let mut props = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            let pname = self.expect_name()?;
            self.expect_op(":")?;
            let val = self.parse_expr()?;
            props.push((pname, val));
        }
        self.expect_op("}")?;
        Ok(BoxDef { name, props, span: self.sp(start) })
    }

    fn parse_layout(&mut self) -> OResult<LayoutDef> {
        let start = self.start();
        self.expect_kw("layout")?;
        let name = self.expect_ident()?;
        let (props, direction, children) = self.parse_layout_body()?;
        Ok(LayoutDef { name, props, direction, children, span: self.sp(start) })
    }

    /// `tokens NAME { .. }` / `breakpoints NAME { .. }` / `scrollbar NAME { .. }`
    /// — same name+body shape as `layout`, minus the leading keyword (the
    /// caller already consumed the dispatching ident).
    fn parse_named_css_block(&mut self) -> OResult<LayoutDef> {
        let start = self.start();
        let name = self.parse_css_name()?;
        let (props, direction, children) = self.parse_layout_body()?;
        Ok(LayoutDef { name, props, direction, children, span: self.sp(start) })
    }

    /// `keyframes NAME { 0% { .. } 50% { .. } from { .. } to { .. } }` —
    /// parsed (so it must be valid) and discarded; this bootstrap models
    /// no animation-keyframe concept.
    fn parse_keyframes_block(&mut self) -> OResult<()> {
        self.parse_css_name()?; // name (may be hyphenated: `toast-in`)
        self.expect_op("{")?;
        while !self.is_op("}") && !self.at_eof() {
            if self.cur().kind == TokKind::Ident && matches!(self.cur().value.as_str(), "from" | "to") {
                self.advance();
            } else {
                self.parse_expr()?; // percentage, e.g. `0%`/`50%`/`100%` (Num followed by '%')
                self.expect_op("%")?;
            }
            self.parse_layout_body()?;
        }
        self.expect_op("}")?;
        Ok(())
    }

    /// The design-token dialect's `{ .. }` block body: ordinary
    /// `name: value [;]` declarations, the native `direction:`/`children:`
    /// special forms, nested `@breakpoint { .. }` / `variant NAME { .. }` /
    /// `hover { .. }` state blocks, and nested `container .sel { .. }` /
    /// `region .sel { .. }` / `item .sel { .. }` / `flow .sel { .. }`
    /// selector blocks — all parsed (so they must be valid) but only the
    /// direct property declarations are kept; nested blocks are recursively
    /// parsed and discarded, matching this bootstrap's "parse-only, no new
    /// evaluation semantics" scope for the responsive-layout dialect.
    fn parse_layout_body(&mut self) -> OResult<(Vec<(String, Expr)>, Direction, Vec<String>)> {
        self.expect_op("{")?;
        let mut props = Vec::new();
        let mut direction = Direction::Row;
        let mut children = Vec::new();
        while !self.is_op("}") && !self.at_eof() {
            if self.is_kw("direction") {
                self.advance();
                self.expect_op(":")?;
                if self.eat_kw_ident_like("row") {
                    direction = Direction::Row;
                } else if self.eat_kw_ident_like("column") {
                    direction = Direction::Column;
                } else {
                    // The design-token dialect also uses `direction:` for
                    // non-flow-axis meanings (e.g. a `flow` block's
                    // connector direction: `direction: horizontal`) — not
                    // every `direction:` describes this layout's own
                    // row/column axis, so fall back to an ordinary
                    // property instead of erroring.
                    let val = self.parse_expr()?;
                    props.push(("direction".to_string(), val));
                }
                self.eat_op(";");
                continue;
            }
            if self.is_kw("children") {
                self.advance();
                self.expect_op(":")?;
                self.expect_op("[")?;
                while !self.is_op("]") && !self.at_eof() {
                    children.push(self.expect_ident()?);
                    self.eat_op(","); // lexer already treats ',' as whitespace, kept for clarity/no-op safety
                }
                self.expect_op("]")?;
                self.eat_op(";");
                continue;
            }
            if self.eat_op("@") {
                self.expect_name()?; // breakpoint name (compact/tablet/../sm/md/lg/xl)
                self.parse_layout_body()?;
                self.eat_op(";");
                continue;
            }
            if self.cur().kind == TokKind::Ident && self.cur().value == "variant" {
                self.advance();
                self.expect_name()?; // variant/state name
                self.parse_layout_body()?;
                self.eat_op(";");
                continue;
            }
            if self.cur().kind == TokKind::Ident && self.cur().value == "hover" {
                self.advance();
                self.parse_layout_body()?;
                self.eat_op(";");
                continue;
            }
            if self.cur().kind == TokKind::Ident
                && matches!(self.cur().value.as_str(), "container" | "region" | "item" | "flow")
                && self.next_tok().kind == TokKind::Op
                && self.next_tok().value == "."
            {
                self.advance();
                self.expect_op(".")?;
                self.parse_css_name()?; // class-like selector name (may be hyphenated)
                self.parse_layout_body()?;
                self.eat_op(";");
                continue;
            }
            let pname = self.parse_css_name()?;
            self.expect_op(":")?;
            let val = self.parse_expr()?;
            props.push((pname, val));
            self.eat_op(";");
        }
        self.expect_op("}")?;
        Ok((props, direction, children))
    }

    /// `row`/`column` lex as Keyword tokens (they're in `KEYWORDS`); this
    /// checks for that specific keyword text.
    fn eat_kw_ident_like(&mut self, v: &str) -> bool {
        if self.cur().kind == TokKind::Keyword && self.cur().value == v {
            self.advance();
            true
        } else {
            false
        }
    }

    fn parse_constraint(&mut self) -> OResult<ConstraintStmt> {
        let start = self.start();
        self.expect_kw("constrain")?;
        let left = self.parse_expr()?;
        let op = if self.eat_op("==") {
            "==".to_string()
        } else if self.eat_op(">=") {
            ">=".to_string()
        } else if self.eat_op("<=") {
            "<=".to_string()
        } else {
            return Err(self.err("expected '==', '>=', or '<=' in constraint"));
        };
        let right = self.parse_expr()?;
        Ok(ConstraintStmt { left, op, right, span: self.sp(start) })
    }

    // ── expressions ──────────────────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> OResult<Expr> {
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
            if self.is_op("*") || self.is_op("/") {
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
            TokKind::Int | TokKind::Float => {
                self.advance();
                let end = self.toks[self.p.saturating_sub(1)].span.end;
                // A unit suffix fused directly onto the number with no
                // separating whitespace (`100vh`, `640px`) — this is a
                // CSS-style dimension, not a real numeric layout equation
                // term, so keep the combined text rather than the bare
                // f64. Adjacency (not just "next token is an Ident") is
                // checked via span position so an unrelated following
                // property name is never mistaken for a unit suffix.
                if self.cur().kind == TokKind::Ident && self.cur().span.start == end {
                    let unit = self.advance().value;
                    return Ok(Expr::Str { v: format!("{}{unit}", t.value), span: self.sp(start) });
                }
                return Ok(Expr::Num { v: t.value.parse().unwrap_or(0.0), span: self.sp(start) });
            }
            TokKind::Str => {
                self.advance();
                return Ok(Expr::Str { v: t.value, span: self.sp(start) });
            }
            TokKind::Ident => {
                self.advance();
                if self.eat_op(".") {
                    let prop = self.expect_name()?;
                    return Ok(Expr::PropRef { obj: t.value, prop, span: self.sp(start) });
                }
                // Bare CSS-style keyword value (`flex`, `pointer`,
                // `space-between`) — hyphen-folded the same way as a
                // property name (see `parse_css_name`).
                let mut word = t.value;
                while self.is_op("-") {
                    self.advance();
                    word.push('-');
                    word.push_str(&self.expect_name()?);
                }
                return Ok(Expr::Str { v: word, span: self.sp(start) });
            }
            TokKind::Keyword if t.value == "parent" => {
                self.advance();
                self.expect_op(".")?;
                let prop = self.expect_name()?;
                return Ok(Expr::PropRef { obj: "parent".to_string(), prop, span: self.sp(start) });
            }
            // `row`/`column` (and other reserved words) used as a bare
            // value outside the `direction:` special form, e.g.
            // `flex_direction: column` — same hyphen-folded-word fallback
            // as a plain Ident value.
            TokKind::Keyword => {
                self.advance();
                let mut word = t.value;
                while self.is_op("-") {
                    self.advance();
                    word.push('-');
                    word.push_str(&self.expect_name()?);
                }
                return Ok(Expr::Str { v: word, span: self.sp(start) });
            }
            _ => {}
        }
        if self.eat_op("(") {
            let e = self.parse_expr()?;
            self.expect_op(")")?;
            return Ok(e);
        }
        // Bare `{token}` reference (the design-token dialect's
        // interpolation shorthand, e.g. `background: {color_panel}`) —
        // unambiguous here since `parse_atom` is only ever reached in
        // value position, never where a block's own body could start.
        if self.eat_op("{") {
            let name = self.expect_name()?;
            self.expect_op("}")?;
            return Ok(Expr::Str { v: format!("{{{name}}}"), span: self.sp(start) });
        }
        Err(self.err(format!("expected a number or 'Box.prop' reference but found '{}'", self.cur_desc())))
    }

    /// Property/selector name, hyphen-folded (`font-family`, `z-index`,
    /// `.mode-btn`) — the design-token dialect's CSS-style naming
    /// convention layered on top of Nexus's native single-word names.
    fn parse_css_name(&mut self) -> OResult<String> {
        let mut s = self.expect_name()?;
        while self.is_op("-") {
            self.advance();
            s.push('-');
            s.push_str(&self.expect_name()?);
        }
        Ok(s)
    }
}
