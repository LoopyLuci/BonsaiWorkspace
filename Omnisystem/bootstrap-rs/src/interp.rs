//! Tree-walking interpreter — the bootstrap Omni runtime.
//!
//! Control flow (return/break/continue and the `?` operator) propagates
//! through `Flow`; runtime errors carry spans for caret diagnostics.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::diag::{OmniError, Phase, Span};
use crate::values::*;

pub enum Flow {
    Return(Value),
    /// `break [value]` — the optional label targets a specific enclosing loop
    /// (`break 'outer`); `None` targets the innermost loop.
    Break(Value, Option<String>),
    /// `continue ['label]` — see `Break` for label semantics.
    Continue(Option<String>),
    Error(Box<OmniError>),
}

pub type EResult = Result<Value, Flow>;

pub struct Interp {
    pub file: String,
    pub structs: HashMap<String, Rc<StructItem>>,
    pub enums: HashMap<String, Rc<EnumItem>>,
    pub methods: HashMap<String, HashMap<String, Rc<FnItem>>>,
    pub fns: HashMap<String, Rc<FnItem>>,
    pub enum_variants: HashMap<String, String>,
    pub assoc_consts: HashMap<String, Value>,
    pub globals: Env,
    pub out: String,
    /// Impl targets of the methods currently executing — resolves `Self::x`
    /// inside static methods (no `self` binding in scope).
    pub type_stack: Vec<String>,
    /// The declared error type name (`E` in `Result<_, E>`) of the innermost
    /// function call currently executing, if its return type declares one.
    /// Consulted by `?` to auto-convert a mismatched error type via `From`.
    pub err_type_stack: Vec<Option<String>>,
}

impl Interp {
    pub fn new(file: &str) -> Self {
        Interp {
            file: file.to_string(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            methods: HashMap::new(),
            fns: HashMap::new(),
            enum_variants: HashMap::new(),
            assoc_consts: HashMap::new(),
            globals: Env::new(),
            out: String::new(),
            type_stack: Vec::new(),
            err_type_stack: Vec::new(),
        }
    }

    pub fn rt(&self, msg: impl Into<String>, span: Span) -> Flow {
        Flow::Error(Box::new(OmniError::new(Phase::Runtime, msg, span, &self.file)))
    }

    pub fn print(&mut self, s: &str) {
        self.out.push_str(s);
    }

    // ── registration ─────────────────────────────────────────────────────────
    pub fn register(&mut self, items: &[Item]) -> Result<(), Box<OmniError>> {
        for item in items {
            self.register_item(item)?;
        }
        Ok(())
    }

    fn register_item(&mut self, item: &Item) -> Result<(), Box<OmniError>> {
        match item {
            Item::Struct(s) => {
                self.structs.insert(s.name.clone(), Rc::new(s.clone()));
            }
            Item::Enum(e) => {
                for v in &e.variants {
                    self.enum_variants.insert(v.name.clone(), e.name.clone());
                }
                self.enums.insert(e.name.clone(), Rc::new(e.clone()));
            }
            Item::Fn(f) => {
                self.fns.insert(f.name.clone(), Rc::new(f.clone()));
            }
            Item::Impl(imp) => {
                let table = self.methods.entry(imp.target.clone()).or_default();
                for m in &imp.methods {
                    table.insert(m.name.clone(), Rc::new(m.clone()));
                }
                for c in &imp.consts {
                    let v = self.eval(&c.value, &self.globals.clone()).map_err(flow_to_err)?;
                    self.assoc_consts.insert(format!("{}::{}", imp.target, c.name), v);
                }
            }
            Item::Const(c) => {
                let v = self.eval(&c.value, &self.globals.clone()).map_err(flow_to_err)?;
                self.globals.set(&c.name, v);
            }
            Item::Mod(m) => {
                for it in &m.items {
                    self.register_item(it)?;
                }
            }
            Item::Use | Item::Trait(_) => {}
        }
        Ok(())
    }

    // ── entry point ──────────────────────────────────────────────────────────
    pub fn run_main(&mut self) -> Result<i32, Box<OmniError>> {
        let Some(main) = self.fns.get("main").cloned() else {
            return Err(Box::new(
                OmniError::new(Phase::Runtime, "no `main` function found", Span::point(1, 1), &self.file)
                    .with_help("add `pub fn main() { ... }`"),
            ));
        };
        match self.call_fn(&main, vec![], None) {
            Ok(v) | Err(Flow::Return(v)) => Ok(match v {
                Value::Int(n) => n as i32,
                Value::Enum { enum_name, variant, .. } if &*enum_name == "Result" => {
                    if &*variant == "Ok" { 0 } else { 1 }
                }
                _ => 0,
            }),
            Err(Flow::Error(e)) => Err(e),
            Err(_) => Ok(0),
        }
    }

    // ── invocation ───────────────────────────────────────────────────────────
    pub fn call_fn(&mut self, f: &Rc<FnItem>, args: Vec<Value>, self_val: Option<Value>) -> EResult {
        let env = self.globals.child();
        let mut ai = 0;
        for p in &f.params {
            if p.is_self {
                if let Some(sv) = &self_val {
                    env.set("self", sv.clone());
                }
                continue;
            }
            env.set(&p.name, args.get(ai).cloned().unwrap_or(Value::Unit));
            ai += 1;
        }
        let Some(body) = &f.body else {
            return Err(self.rt(format!("function `{}` has no body", f.name), f.span));
        };
        // `Result<_, E>` return type declares the error type `?` should
        // convert propagated errors into (via `From`) inside this call.
        let err_ty = f.ret.as_ref().filter(|t| t.name == "Result").and_then(|t| t.args.get(1)).map(|t| t.name.clone());
        self.err_type_stack.push(err_ty);
        let result = match self.eval_block(body, &env) {
            Err(Flow::Return(v)) => Ok(v),
            other => other,
        };
        self.err_type_stack.pop();
        result
    }

    /// Implements `?`'s Rust-idiomatic error conversion: if the enclosing
    /// function declares `-> Result<_, E>` and the propagated error's type
    /// differs from `E`, look for `impl From<SourceErr> for E { fn from(..) }`
    /// and apply it — matching Rust's automatic `?` + `From` conversion so
    /// callers don't need `.map_err(...)` at every function boundary.
    ///
    /// Limitation: the method table is keyed by `(target_type, method_name)`
    /// only, not by source-parameter type, so if `E` has more than one `impl
    /// From<X> for E` the most-recently-registered `from` wins rather than
    /// dispatching on the actual source type. Fine for the common one-`From`
    /// case; multi-source `From` needs a richer method table to do properly.
    fn convert_propagated_err(&mut self, err_val: Value) -> Value {
        let Some(Some(target_ty)) = self.err_type_stack.last().cloned() else {
            return err(err_val);
        };
        if err_val.type_name() == Some(target_ty.as_str()) {
            return err(err_val);
        }
        let Some(from_fn) = self.methods.get(&target_ty).and_then(|m| m.get("from")).cloned() else {
            // No conversion registered — propagate the original error value
            // rather than hard-erroring, since this interpreter is dynamically
            // typed and can't statically require the impl to exist.
            return err(err_val);
        };
        match self.call_fn(&from_fn, vec![err_val.clone()], None) {
            Ok(converted) => err(converted),
            Err(_) => err(err_val),
        }
    }

    pub fn apply(&mut self, f: &Value, args: Vec<Value>, span: Span) -> EResult {
        match f {
            Value::Closure { params, body, env } => {
                let scope = env.child();
                for (i, p) in params.iter().enumerate() {
                    scope.set(p, args.get(i).cloned().unwrap_or(Value::Unit));
                }
                match self.eval(body, &scope) {
                    Err(Flow::Return(v)) => Ok(v),
                    other => other,
                }
            }
            Value::Fn { decl, self_val, owner } => {
                if let Some(o) = owner {
                    self.type_stack.push(o.to_string());
                }
                let r = self.call_fn(&decl.clone(), args, self_val.as_deref().cloned());
                if owner.is_some() {
                    self.type_stack.pop();
                }
                r
            }
            Value::Builtin(name) => {
                let segs: Vec<String> = name.split("::").map(str::to_string).collect();
                crate::builtins::static_builtin(&segs, &args)
                    .ok_or_else(|| self.rt(format!("unknown builtin `{name}`"), span))
            }
            other => Err(self.rt(format!("value is not callable: {}", other.display()), span)),
        }
    }

    // ── blocks & statements ──────────────────────────────────────────────────
    pub fn eval_block(&mut self, block: &Block, parent: &Env) -> EResult {
        let env = parent.child();
        for s in &block.stmts {
            if let Stmt::Item(item) = s {
                self.register_item(item).map_err(Flow::Error)?;
            }
        }
        let mut last = Value::Unit;
        for s in &block.stmts {
            match s {
                Stmt::Let { pat, init, span } => {
                    let v = match init {
                        Some(e) => self.eval(e, &env)?,
                        None => Value::Unit,
                    };
                    if !self.match_pattern(pat, &v, &env)? {
                        return Err(self.rt(format!("pattern does not match value {}", v.debug()), *span));
                    }
                    last = Value::Unit;
                }
                Stmt::Expr { expr, semi } => {
                    last = self.eval(expr, &env)?;
                    if *semi {
                        last = Value::Unit;
                    }
                }
                Stmt::Item(_) => last = Value::Unit,
            }
        }
        Ok(last)
    }

    // ── expressions ──────────────────────────────────────────────────────────
    pub fn eval(&mut self, e: &Expr, env: &Env) -> EResult {
        match e {
            Expr::Int { v, .. } => Ok(Value::Int(*v)),
            Expr::Float { v, .. } => Ok(Value::Float(*v)),
            Expr::Str { v, .. } => Ok(vstr(v.clone())),
            Expr::Char { v, .. } => Ok(Value::Char(*v)),
            Expr::Bool { v, .. } => Ok(Value::Bool(*v)),
            Expr::Path { segs, span } => self.eval_path(segs, *span, env),
            Expr::Field { obj, name, span } => {
                let o = self.eval(obj, env)?;
                self.get_field(&o, name, *span)
            }
            Expr::Index { obj, index, span } => {
                let o = self.eval(obj, env)?;
                let i = self.eval(index, env)?;
                self.get_index(&o, &i, *span)
            }
            Expr::Call { callee, args, span } => self.eval_call(callee, args, *span, env),
            Expr::Method { recv, name, args, span } => self.eval_method(recv, name, args, *span, env),
            Expr::Unary { op, operand, span } => {
                let v = self.eval(operand, env)?;
                match (op.as_str(), &v) {
                    ("-", Value::Int(n)) => Ok(Value::Int(-n)),
                    ("-", Value::Float(n)) => Ok(Value::Float(-n)),
                    ("!", Value::Bool(b)) => Ok(Value::Bool(!b)),
                    ("!", Value::Int(n)) => Ok(Value::Int(!n)),
                    _ => Err(self.rt(format!("cannot apply unary `{op}`"), *span)),
                }
            }
            Expr::Binary { op, left, right, span } => {
                if op == "&&" {
                    let l = self.eval(left, env)?;
                    if !l.truthy() {
                        return Ok(Value::Bool(false));
                    }
                    let r = self.eval(right, env)?;
                    return Ok(Value::Bool(r.truthy()));
                }
                if op == "||" {
                    let l = self.eval(left, env)?;
                    if l.truthy() {
                        return Ok(Value::Bool(true));
                    }
                    let r = self.eval(right, env)?;
                    return Ok(Value::Bool(r.truthy()));
                }
                let l = self.eval(left, env)?;
                let r = self.eval(right, env)?;
                self.binop(op, &l, &r, *span)
            }
            Expr::Assign { op, target, value, span } => {
                let mut v = self.eval(value, env)?;
                if op != "=" {
                    let cur = self.eval(target, env)?;
                    v = self.binop(&op[..op.len() - 1], &cur, &v, *span)?;
                }
                self.assign_to(target, v, env)?;
                Ok(Value::Unit)
            }
            Expr::Range { from, to, inclusive, span } => {
                let f = match from {
                    Some(x) => self.eval(x, env)?,
                    None => Value::Int(0),
                };
                let t = match to {
                    Some(x) => self.eval(x, env)?,
                    None => Value::Int(0),
                };
                match (f, t) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Range { from: a, to: b, inclusive: *inclusive }),
                    _ => Err(self.rt("range bounds must be integers", *span)),
                }
            }
            Expr::If { let_pat, cond, then, els, .. } => {
                if let Some(pat) = let_pat {
                    let v = self.eval(cond, env)?;
                    let scope = env.child();
                    if self.match_pattern(pat, &v, &scope)? {
                        return self.eval_block(then, &scope);
                    }
                    if let Some(e2) = els {
                        return self.eval(e2, env);
                    }
                    return Ok(Value::Unit);
                }
                if self.eval(cond, env)?.truthy() {
                    self.eval_block(then, env)
                } else if let Some(e2) = els {
                    self.eval(e2, env)
                } else {
                    Ok(Value::Unit)
                }
            }
            Expr::Match { scrut, arms, span } => {
                let v = self.eval(scrut, env)?;
                for arm in arms {
                    let scope = env.child();
                    if self.match_pattern(&arm.pat, &v, &scope)? {
                        if let Some(g) = &arm.guard {
                            if !self.eval(g, &scope)?.truthy() {
                                continue;
                            }
                        }
                        return self.eval(&arm.body, &scope);
                    }
                }
                Err(self
                    .rt(format!("no match arm covered value {}", v.debug()), *span)
                    .into_help("add a catch-all `_ => ...` arm"))
            }
            Expr::While { let_pat, cond, body, label, .. } => {
                loop {
                    let res = if let Some(pat) = let_pat {
                        let v = self.eval(cond, env)?;
                        let scope = env.child();
                        if !self.match_pattern(pat, &v, &scope)? {
                            break;
                        }
                        self.eval_block(body, &scope)
                    } else {
                        if !self.eval(cond, env)?.truthy() {
                            break;
                        }
                        self.eval_block(body, env)
                    };
                    match loop_flow(label, res) {
                        LoopAction::Break(_) => break,
                        LoopAction::Continue | LoopAction::Normal => continue,
                        LoopAction::Propagate(f) => return Err(f),
                    }
                }
                Ok(Value::Unit)
            }
            Expr::For { pat, iter, body, label, span } => {
                let it = self.eval(iter, env)?;
                let items = self.iter_values(&it, *span)?;
                for item in items {
                    let scope = env.child();
                    if !self.match_pattern(pat, &item, &scope)? {
                        return Err(self.rt("for-loop pattern does not match", *span));
                    }
                    match loop_flow(label, self.eval_block(body, &scope)) {
                        LoopAction::Break(_) => break,
                        LoopAction::Continue | LoopAction::Normal => continue,
                        LoopAction::Propagate(f) => return Err(f),
                    }
                }
                Ok(Value::Unit)
            }
            Expr::Loop { body, label, .. } => loop {
                match loop_flow(label, self.eval_block(body, env)) {
                    LoopAction::Break(v) => return Ok(v),
                    LoopAction::Continue | LoopAction::Normal => continue,
                    LoopAction::Propagate(f) => return Err(f),
                }
            },
            Expr::BlockE { block } => self.eval_block(block, env),
            Expr::Return { value, .. } => {
                let v = match value {
                    Some(e2) => self.eval(e2, env)?,
                    None => Value::Unit,
                };
                Err(Flow::Return(v))
            }
            Expr::Break { value, label, .. } => {
                let v = match value {
                    Some(e2) => self.eval(e2, env)?,
                    None => Value::Unit,
                };
                Err(Flow::Break(v, label.clone()))
            }
            Expr::Continue { label, .. } => Err(Flow::Continue(label.clone())),
            Expr::StructLit { path, fields, spread, .. } => {
                let name = path.last().cloned().unwrap_or_default();
                // enum struct-variant literal: Enum::Variant { .. }
                if path.len() > 1 && self.enums.contains_key(&path[0]) {
                    let mut payload = Vec::new();
                    for (_, fe) in fields {
                        payload.push(self.eval(fe, env)?);
                    }
                    return Ok(venum(&path[0], &name, payload));
                }
                let mut map = BTreeMap::new();
                if let Some(sp) = spread {
                    if let Value::Struct { fields: base, .. } = self.eval(sp, env)? {
                        for (k, v) in base.borrow().iter() {
                            map.insert(k.clone(), v.clone());
                        }
                    }
                }
                for (fname, fe) in fields {
                    let v = self.eval(fe, env)?;
                    map.insert(fname.clone(), v);
                }
                Ok(Value::Struct { name: Rc::from(name.as_str()), fields: Rc::new(RefCell::new(map)) })
            }
            Expr::Array { elems, repeat, .. } => {
                if let Some(rep) = repeat {
                    let val = if let Some(e0) = elems.first() { self.eval(e0, env)? } else { Value::Int(0) };
                    let n = match self.eval(rep, env)? {
                        Value::Int(n) => n.max(0) as usize,
                        _ => 0,
                    };
                    return Ok(vvec((0..n).map(|_| val.deep_clone()).collect()));
                }
                let mut items = Vec::new();
                for el in elems {
                    items.push(self.eval(el, env)?);
                }
                Ok(vvec(items))
            }
            Expr::Tuple { elems, .. } => {
                let mut items = Vec::new();
                for el in elems {
                    items.push(self.eval(el, env)?);
                }
                Ok(vtuple(items))
            }
            Expr::Closure { params, body, .. } => Ok(Value::Closure {
                params: Rc::new(params.clone()),
                body: Rc::new((**body).clone()),
                env: env.clone(),
            }),
            Expr::Try { expr, span } => {
                let v = self.eval(expr, env)?;
                if let Value::Enum { enum_name, variant, payload } = &v {
                    let inner = payload.borrow().first().cloned().unwrap_or(Value::Unit);
                    match (&**enum_name, &**variant) {
                        ("Result", "Ok") | ("Option", "Some") => return Ok(inner),
                        ("Result", "Err") => return Err(Flow::Return(self.convert_propagated_err(inner))),
                        ("Option", "None") => return Err(Flow::Return(none())),
                        _ => {}
                    }
                }
                Err(self.rt("the `?` operator can only be applied to Result or Option", *span))
            }
            Expr::Cast { expr, ty, .. } => {
                let v = self.eval(expr, env)?;
                let n = match &v {
                    Value::Int(n) => *n as f64,
                    Value::Float(f) => *f,
                    Value::Char(c) => *c as u32 as f64,
                    Value::Bool(b) => *b as i64 as f64,
                    _ => return Ok(v),
                };
                Ok(if ty.name.starts_with('f') {
                    Value::Float(n)
                } else if ty.name == "char" {
                    Value::Char(char::from_u32(n as u32).unwrap_or('\0'))
                } else if ty.name == "bool" {
                    Value::Bool(n != 0.0)
                } else {
                    Value::Int(n as i64)
                })
            }
            Expr::Macro { name, args, repeat, span } => self.eval_macro(name, args, repeat.as_deref(), *span, env),
        }
    }

    fn eval_path(&mut self, segs: &[String], span: Span, env: &Env) -> EResult {
        if segs.len() == 1 {
            let name = &segs[0];
            if let Some(v) = env.get(name) {
                return Ok(v);
            }
            if name == "None" {
                return Ok(none());
            }
            if let Some(f) = self.fns.get(name) {
                return Ok(Value::Fn { decl: f.clone(), self_val: None, owner: None });
            }
            if let Some(en) = self.enum_variants.get(name) {
                return Ok(venum(&en.clone(), name, vec![]));
            }
            return Err(self.rt(format!("unknown name `{name}`"), span));
        }
        // `Self::member` resolves through the runtime type of `self`, or the
        // impl target of the currently executing method (static context).
        let self_ty = if segs[0] == "Self" {
            env.get("self")
                .and_then(|v| v.type_name().map(str::to_string))
                .or_else(|| self.type_stack.last().cloned())
        } else {
            None
        };
        let ty = &self_ty.unwrap_or_else(|| segs[0].clone());
        let member = &segs[segs.len() - 1];
        if let Some(en) = self.enums.get(ty) {
            if en.variants.iter().any(|v| &v.name == member) {
                return Ok(venum(ty, member, vec![]));
            }
        }
        if ty == "Option" && member == "None" {
            return Ok(none());
        }
        if let Some(v) = self.assoc_consts.get(&format!("{ty}::{member}")) {
            return Ok(v.clone());
        }
        if let Some(m) = self.methods.get(ty).and_then(|t| t.get(member)) {
            return Ok(Value::Fn { decl: m.clone(), self_val: None, owner: Some(Rc::from(ty.as_str())) });
        }
        if let Some(v) = env.get(member) {
            return Ok(v);
        }
        // Defer to call time: `Vec::new` etc. used as a function value
        // (or_insert_with(Vec::new)). Unknown names error when applied.
        Ok(Value::Builtin(Rc::from(segs.join("::").as_str())))
    }

    fn get_field(&self, obj: &Value, name: &str, span: Span) -> EResult {
        match obj {
            Value::Struct { name: sn, fields } => fields
                .borrow()
                .get(name)
                .cloned()
                .ok_or_else(|| self.rt(format!("struct `{sn}` has no field `{name}`"), span)),
            Value::Tuple(items) => {
                let idx: usize = name.parse().unwrap_or(usize::MAX);
                items.borrow().get(idx).cloned().ok_or_else(|| self.rt(format!("tuple index {name} out of range"), span))
            }
            other => Err(self.rt(format!("cannot access field `{name}` on {}", other.display()), span)),
        }
    }

    fn get_index(&self, obj: &Value, idx: &Value, span: Span) -> EResult {
        match (obj, idx) {
            (Value::Vec(items), Value::Int(i)) => {
                let items = items.borrow();
                if *i < 0 || *i as usize >= items.len() {
                    return Err(self.rt(
                        format!("index out of bounds: len is {} but index is {i}", items.len()),
                        span,
                    ));
                }
                Ok(items[*i as usize].clone())
            }
            (Value::Map(m), k) => m
                .borrow()
                .get(&k.key())
                .map(|(_, v)| v.clone())
                .ok_or_else(|| self.rt("key not found", span)),
            (Value::Str(s), Value::Int(i)) => {
                let s = s.borrow();
                s.chars()
                    .nth(*i as usize)
                    .map(Value::Char)
                    .ok_or_else(|| self.rt("string index out of bounds", span))
            }
            _ => Err(self.rt("cannot index this value", span)),
        }
    }

    fn eval_call(&mut self, callee: &Expr, args: &[Expr], span: Span, env: &Env) -> EResult {
        if let Expr::Path { segs, span: pspan } = callee {
            let mut vals = Vec::new();
            for a in args {
                vals.push(self.eval(a, env)?);
            }
            if let Some(ctor) = self.try_enum_ctor(segs, &vals) {
                return Ok(ctor);
            }
            if let Some(callable) = self.path_callable(segs, env) {
                return self.apply(&callable, vals, span);
            }
            if let Some(b) = crate::builtins::static_builtin(segs, &vals) {
                return Ok(b);
            }
            return Err(self.rt(format!("unknown function `{}`", segs.join("::")), *pspan));
        }
        let f = self.eval(callee, env)?;
        let mut vals = Vec::new();
        for a in args {
            vals.push(self.eval(a, env)?);
        }
        self.apply(&f, vals, span)
    }

    fn try_enum_ctor(&self, segs: &[String], args: &[Value]) -> Option<Value> {
        let last = segs.last()?;
        let first_arg = || args.first().cloned().unwrap_or(Value::Unit);
        if segs.len() == 1 {
            match last.as_str() {
                "Some" => return Some(some(first_arg())),
                "Ok" => return Some(ok(first_arg())),
                "Err" => return Some(err(first_arg())),
                _ => {}
            }
            if let Some(en) = self.enum_variants.get(last) {
                return Some(venum(en, last, args.to_vec()));
            }
            return None;
        }
        let ty = &segs[0];
        if let Some(en) = self.enums.get(ty) {
            if en.variants.iter().any(|v| &v.name == last) {
                return Some(venum(ty, last, args.to_vec()));
            }
        }
        match (ty.as_str(), last.as_str()) {
            ("Option", "Some") => Some(some(first_arg())),
            ("Result", "Ok") => Some(ok(first_arg())),
            ("Result", "Err") => Some(err(first_arg())),
            _ => None,
        }
    }

    fn path_callable(&self, segs: &[String], env: &Env) -> Option<Value> {
        if segs.len() == 1 {
            if let Some(v) = env.get(&segs[0]) {
                if matches!(v, Value::Fn { .. } | Value::Closure { .. }) {
                    return Some(v);
                }
            }
            return self.fns.get(&segs[0]).map(|f| Value::Fn { decl: f.clone(), self_val: None, owner: None });
        }
        let self_ty = if segs[0] == "Self" {
            env.get("self")
                .and_then(|v| v.type_name().map(str::to_string))
                .or_else(|| self.type_stack.last().cloned())
        } else {
            None
        };
        let ty = self_ty.unwrap_or_else(|| segs[0].clone());
        let member = &segs[segs.len() - 1];
        self.methods
            .get(&ty)
            .and_then(|t| t.get(member))
            .map(|m| Value::Fn { decl: m.clone(), self_val: None, owner: Some(Rc::from(ty.as_str())) })
    }

    fn eval_method(&mut self, recv: &Expr, name: &str, args: &[Expr], span: Span, env: &Env) -> EResult {
        let r = self.eval(recv, env)?;
        let mut vals = Vec::new();
        for a in args {
            vals.push(self.eval(a, env)?);
        }
        if let Some(tn) = r.type_name().map(str::to_string) {
            if let Some(m) = self.methods.get(&tn).and_then(|t| t.get(name)).cloned() {
                self.type_stack.push(tn);
                let res = self.call_fn(&m, vals, Some(r));
                self.type_stack.pop();
                return res;
            }
        }
        match crate::builtins::call_builtin_method(self, &r, name, &vals, span) {
            Some(res) => res,
            None => Err(self
                .rt(
                    format!(
                        "no method `{name}` on {}",
                        r.type_name().map(|t| format!("`{t}`")).unwrap_or_else(|| "this value".into())
                    ),
                    span,
                )
                .into_help(format!("define `fn {name}(&self, ...)` in an impl block, or use a supported builtin"))),
        }
    }

    fn assign_to(&mut self, target: &Expr, value: Value, env: &Env) -> Result<(), Flow> {
        match target {
            Expr::Path { segs, .. } if segs.len() == 1 => {
                if !env.assign(&segs[0], value.clone()) {
                    env.set(&segs[0], value);
                }
                Ok(())
            }
            Expr::Field { obj, name, span } => {
                let o = self.eval(obj, env)?;
                match o {
                    Value::Struct { fields, .. } => {
                        fields.borrow_mut().insert(name.clone(), value);
                        Ok(())
                    }
                    Value::Tuple(items) => {
                        let idx: usize = name.parse().unwrap_or(usize::MAX);
                        let mut it = items.borrow_mut();
                        if idx < it.len() {
                            it[idx] = value;
                            Ok(())
                        } else {
                            Err(self.rt("tuple index out of range", *span))
                        }
                    }
                    _ => Err(self.rt("invalid assignment target", *span)),
                }
            }
            Expr::Index { obj, index, span } => {
                let o = self.eval(obj, env)?;
                let i = self.eval(index, env)?;
                match (&o, &i) {
                    (Value::Vec(items), Value::Int(n)) => {
                        let mut it = items.borrow_mut();
                        let idx = *n as usize;
                        if idx < it.len() {
                            it[idx] = value;
                            Ok(())
                        } else {
                            Err(self.rt(format!("index out of bounds: len is {} but index is {n}", it.len()), *span))
                        }
                    }
                    (Value::Map(m), k) => {
                        m.borrow_mut().insert(k.key(), (k.clone(), value));
                        Ok(())
                    }
                    _ => Err(self.rt("invalid index assignment", *span)),
                }
            }
            other => Err(self.rt("invalid assignment target", other.span())),
        }
    }

    pub fn binop(&self, op: &str, l: &Value, r: &Value, span: Span) -> EResult {
        match op {
            "==" => return Ok(Value::Bool(l.eq_val(r))),
            "!=" => return Ok(Value::Bool(!l.eq_val(r))),
            _ => {}
        }
        // string concatenation
        if op == "+" {
            if let (Value::Str(a), b) = (l, r) {
                return Ok(vstr(format!("{}{}", a.borrow(), b.display())));
            }
            if let (a, Value::Str(b)) = (l, r) {
                return Ok(vstr(format!("{}{}", a.display(), b.borrow())));
            }
        }
        let num = |v: &Value| -> Option<(f64, bool)> {
            match v {
                Value::Int(n) => Some((*n as f64, true)),
                Value::Float(f) => Some((*f, false)),
                _ => None,
            }
        };
        if let (Some((a, ai)), Some((b, bi))) = (num(l), num(r)) {
            let both_int = ai && bi;
            let wrap = |x: f64| if both_int { Value::Int(x as i64) } else { Value::Float(x) };
            return Ok(match op {
                "+" => wrap(a + b),
                "-" => wrap(a - b),
                "*" => wrap(a * b),
                "/" => {
                    if both_int {
                        if b == 0.0 {
                            return Err(self.rt("attempt to divide by zero", span));
                        }
                        Value::Int((a as i64) / (b as i64))
                    } else {
                        Value::Float(a / b)
                    }
                }
                "%" => {
                    if both_int {
                        if b == 0.0 {
                            return Err(self.rt("attempt to calculate remainder with a divisor of zero", span));
                        }
                        Value::Int((a as i64) % (b as i64))
                    } else {
                        Value::Float(a % b)
                    }
                }
                "<" => Value::Bool(a < b),
                ">" => Value::Bool(a > b),
                "<=" => Value::Bool(a <= b),
                ">=" => Value::Bool(a >= b),
                "&" => Value::Int((a as i64) & (b as i64)),
                "|" => Value::Int((a as i64) | (b as i64)),
                "^" => Value::Int((a as i64) ^ (b as i64)),
                "<<" => Value::Int((a as i64) << (b as i64)),
                ">>" => Value::Int((a as i64) >> (b as i64)),
                _ => return Err(self.rt(format!("unsupported operator `{op}`"), span)),
            });
        }
        if let (Value::Bool(a), Value::Bool(b)) = (l, r) {
            return Ok(match op {
                "&" => Value::Bool(*a && *b),
                "|" => Value::Bool(*a || *b),
                "^" => Value::Bool(a != b),
                _ => return Err(self.rt(format!("cannot apply `{op}` to bools"), span)),
            });
        }
        if let (Value::Str(a), Value::Str(b)) = (l, r) {
            let (a, b) = (a.borrow(), b.borrow());
            return Ok(match op {
                "<" => Value::Bool(*a < *b),
                ">" => Value::Bool(*a > *b),
                "<=" => Value::Bool(*a <= *b),
                ">=" => Value::Bool(*a >= *b),
                _ => return Err(self.rt(format!("cannot apply `{op}` to strings"), span)),
            });
        }
        Err(self.rt(format!("cannot apply `{op}` to these operand types"), span))
    }

    pub fn iter_values(&self, v: &Value, span: Span) -> Result<Vec<Value>, Flow> {
        match v {
            Value::Range { from, to, inclusive } => {
                let end = if *inclusive { *to } else { *to - 1 };
                Ok((*from..=end).map(Value::Int).collect())
            }
            Value::Vec(items) => Ok(items.borrow().clone()),
            Value::Set(s) => Ok(s.borrow().values().cloned().collect()),
            Value::Map(m) => Ok(m.borrow().values().map(|(k, val)| vtuple(vec![k.clone(), val.clone()])).collect()),
            Value::Str(s) => Ok(s.borrow().chars().map(Value::Char).collect()),
            Value::Enum { enum_name, variant, payload } if &**enum_name == "Option" => {
                if &**variant == "Some" {
                    Ok(vec![payload.borrow().first().cloned().unwrap_or(Value::Unit)])
                } else {
                    Ok(vec![])
                }
            }
            other => Err(self.rt(format!("`{}` is not iterable", other.display()), span)),
        }
    }

    // ── macros ───────────────────────────────────────────────────────────────
    fn eval_macro(&mut self, name: &str, args: &[Expr], repeat: Option<&Expr>, span: Span, env: &Env) -> EResult {
        match name {
            "println" => {
                let s = self.format_macro(args, env)?;
                self.print(&(s + "\n"));
                Ok(Value::Unit)
            }
            "print" => {
                let s = self.format_macro(args, env)?;
                self.print(&s);
                Ok(Value::Unit)
            }
            "eprintln" => {
                let s = self.format_macro(args, env)?;
                eprintln!("{s}");
                Ok(Value::Unit)
            }
            "format" => {
                let s = self.format_macro(args, env)?;
                Ok(vstr(s))
            }
            "vec" => {
                if let Some(rep) = repeat {
                    let el = if let Some(e0) = args.first() { self.eval(e0, env)? } else { Value::Int(0) };
                    let n = match self.eval(rep, env)? {
                        Value::Int(n) => n.max(0) as usize,
                        _ => 0,
                    };
                    return Ok(vvec((0..n).map(|_| el.deep_clone()).collect()));
                }
                let mut items = Vec::new();
                for a in args {
                    items.push(self.eval(a, env)?);
                }
                Ok(vvec(items))
            }
            "panic" => {
                let m = self.format_macro(args, env)?;
                Err(self.rt(format!("panicked: {}", if m.is_empty() { "explicit panic" } else { &m }), span))
            }
            "assert" => {
                let c = self.eval(&args[0], env)?;
                if !c.truthy() {
                    return Err(self.rt("assertion failed", span));
                }
                Ok(Value::Unit)
            }
            "assert_eq" => {
                let a = self.eval(&args[0], env)?;
                let b = self.eval(&args[1], env)?;
                if !a.eq_val(&b) {
                    return Err(self.rt(
                        format!("assertion failed: `(left == right)`\n  left: {}\n right: {}", a.debug(), b.debug()),
                        span,
                    ));
                }
                Ok(Value::Unit)
            }
            "assert_ne" => {
                let a = self.eval(&args[0], env)?;
                let b = self.eval(&args[1], env)?;
                if a.eq_val(&b) {
                    return Err(self.rt("assertion failed: `(left != right)`", span));
                }
                Ok(Value::Unit)
            }
            "todo" | "unimplemented" => Err(self.rt("not yet implemented", span)),
            "dbg" => {
                let v = if let Some(a) = args.first() { self.eval(a, env)? } else { Value::Unit };
                let s = v.debug();
                self.print(&(s + "\n"));
                Ok(v)
            }
            _ => {
                for a in args {
                    self.eval(a, env)?;
                }
                Ok(Value::Unit)
            }
        }
    }

    /// format!/println! — supports {}, {:?}, {name}, positional {0}, and {{ }}.
    fn format_macro(&mut self, args: &[Expr], env: &Env) -> Result<String, Flow> {
        if args.is_empty() {
            return Ok(String::new());
        }
        let Expr::Str { v: fmt, .. } = &args[0] else {
            let mut parts = Vec::new();
            for a in args {
                parts.push(self.eval(a, env)?.display());
            }
            return Ok(parts.join(" "));
        };
        let mut rest = Vec::new();
        for a in &args[1..] {
            rest.push(self.eval(a, env)?);
        }
        let mut out = String::new();
        let mut ai = 0usize;
        let chars: Vec<char> = fmt.chars().collect();
        let mut i = 0usize;
        while i < chars.len() {
            let c = chars[i];
            if c == '{' && chars.get(i + 1) == Some(&'{') {
                out.push('{');
                i += 2;
                continue;
            }
            if c == '}' && chars.get(i + 1) == Some(&'}') {
                out.push('}');
                i += 2;
                continue;
            }
            if c == '{' {
                let close = chars[i..].iter().position(|&x| x == '}').map(|p| p + i);
                if let Some(end) = close {
                    let spec: String = chars[i + 1..end].iter().collect();
                    let dbg = spec.contains('?');
                    let name_or_idx = spec.split(':').next().unwrap_or("");
                    let val = if name_or_idx.is_empty() {
                        let v = rest.get(ai).cloned().unwrap_or(Value::Unit);
                        ai += 1;
                        v
                    } else if name_or_idx.chars().all(|c| c.is_ascii_digit()) {
                        rest.get(name_or_idx.parse::<usize>().unwrap_or(0)).cloned().unwrap_or(Value::Unit)
                    } else {
                        env.get(name_or_idx).unwrap_or(Value::Unit)
                    };
                    out.push_str(&if dbg { val.debug() } else { val.display() });
                    i = end + 1;
                    continue;
                }
            }
            out.push(c);
            i += 1;
        }
        Ok(out)
    }

    // ── pattern matching ─────────────────────────────────────────────────────
    // range_key is a free fn defined below the impl.

    pub fn match_pattern(&mut self, pat: &Pattern, v: &Value, env: &Env) -> Result<bool, Flow> {
        match pat {
            Pattern::Wild => Ok(true),
            Pattern::Bind { name, .. } => {
                env.set(name, v.clone());
                Ok(true)
            }
            Pattern::Ref(inner) => self.match_pattern(inner, v, env),
            Pattern::Lit(e) => {
                let lit = self.eval(e, env)?;
                Ok(lit.eq_val(v))
            }
            Pattern::Or(alts) => {
                for a in alts {
                    if self.match_pattern(a, v, env)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Pattern::Range { lo, hi, inclusive, .. } => {
                // Ordered scalar (int/float/char) range membership test.
                let Some(x) = range_key(v) else { return Ok(false) };
                if let Some(lo) = lo {
                    let lo = self.eval(lo, env)?;
                    match range_key(&lo) {
                        Some(l) if x < l => return Ok(false),
                        None => return Ok(false),
                        _ => {}
                    }
                }
                if let Some(hi) = hi {
                    let hi = self.eval(hi, env)?;
                    match range_key(&hi) {
                        Some(h) if *inclusive && x > h => return Ok(false),
                        Some(h) if !*inclusive && x >= h => return Ok(false),
                        None => return Ok(false),
                        _ => {}
                    }
                }
                Ok(true)
            }
            Pattern::Tuple(elems) => {
                let Value::Tuple(items) = v else { return Ok(false) };
                let items = items.borrow().clone();
                if items.len() != elems.len() {
                    return Ok(false);
                }
                for (p, item) in elems.iter().zip(items.iter()) {
                    if !self.match_pattern(p, item, env)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Pattern::Path { path, .. } => {
                let name = path.last().map(String::as_str).unwrap_or("");
                if let Value::Enum { variant, .. } = v {
                    return Ok(&**variant == name);
                }
                if let Some(c) = env.get(name) {
                    return Ok(c.eq_val(v));
                }
                Ok(false)
            }
            Pattern::Enum { path, elems, .. } => {
                let name = path.last().map(String::as_str).unwrap_or("");
                let Value::Enum { variant, payload, .. } = v else { return Ok(false) };
                if &**variant != name {
                    return Ok(false);
                }
                let payload = payload.borrow().clone();
                if elems.len() != payload.len() {
                    if elems.len() == 1 && !payload.is_empty() {
                        return self.match_pattern(&elems[0], &payload[0], env);
                    }
                    return Ok(false);
                }
                for (p, item) in elems.iter().zip(payload.iter()) {
                    if !self.match_pattern(p, item, env)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Pattern::Struct { fields, .. } => {
                let Value::Struct { fields: vf, .. } = v else { return Ok(false) };
                for (fname, fpat) in fields {
                    let Some(fv) = vf.borrow().get(fname).cloned() else { return Ok(false) };
                    if !self.match_pattern(fpat, &fv, env)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }
}

impl Flow {
    fn into_help(self, help: impl Into<String>) -> Flow {
        match self {
            Flow::Error(e) => Flow::Error(Box::new(e.with_help(help))),
            other => other,
        }
    }
}

fn flow_to_err(f: Flow) -> Box<OmniError> {
    match f {
        Flow::Error(e) => e,
        _ => Box::new(OmniError::new(Phase::Runtime, "control flow escaped item initializer", Span::point(1, 1), "")),
    }
}

/// Outcome of one loop-body iteration once labels are resolved against the
/// loop's own label.
enum LoopAction {
    Break(Value),
    Continue,
    Propagate(Flow),
    Normal,
}

/// Resolve a loop-body result against `my` (this loop's label). An unlabeled
/// break/continue always targets the innermost loop; a labeled one only acts
/// here if the label matches, otherwise it propagates outward.
fn loop_flow(my: &Option<String>, r: Result<Value, Flow>) -> LoopAction {
    match r {
        Ok(_) => LoopAction::Normal,
        Err(Flow::Break(v, None)) => LoopAction::Break(v),
        Err(Flow::Break(v, Some(l))) if my.as_deref() == Some(l.as_str()) => LoopAction::Break(v),
        Err(Flow::Continue(None)) => LoopAction::Continue,
        Err(Flow::Continue(Some(l))) if my.as_deref() == Some(l.as_str()) => LoopAction::Continue,
        Err(other) => LoopAction::Propagate(other),
    }
}

/// Maps int/float/char values to a common `f64` ordering key for range-pattern
/// membership tests. Returns `None` for non-scalar values (which never match a
/// range pattern).
fn range_key(v: &Value) -> Option<f64> {
    match v {
        Value::Int(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        Value::Char(c) => Some(*c as u32 as f64),
        _ => None,
    }
}
