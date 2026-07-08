//! Sylva tree-walking interpreter.
//!
//! Structural differences from Titan's interpreter (deliberate — Sylva is a
//! genuinely distinct language, not a reskinned copy):
//!   - No `main()` entry point requirement: top-level statements execute
//!     top-to-bottom exactly like a real Python script.
//!   - Error handling is real exceptions (`raise`/`try`/`except`/`finally`),
//!     not `Result`/`?` — `Flow::Raise(Value)` carries the raised value.
//!   - Classes (`class Name(Base):`) with single inheritance + MRO walk,
//!     not structs+impl+traits.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::diag::{OmniError, Phase, Span};
use crate::values::*;

pub enum Flow {
    Return(Value),
    Break,
    Continue,
    /// A raised exception value — typically a `Str` message in this
    /// bootstrap subset (no built-in exception class hierarchy yet; user
    /// classes can still be raised and caught by class name via `except`).
    Raise(Value),
}

pub type EResult = Result<Value, Flow>;

pub struct Interp {
    pub file: String,
    pub globals: Env,
    pub out: String,
}

impl Interp {
    pub fn new(file: &str) -> Self {
        Interp { file: file.to_string(), globals: Scope::new(), out: String::new() }
    }

    /// Raises a runtime fault as a **catchable Python-style exception**
    /// (`Flow::Raise`) — division by zero, index-out-of-range, missing
    /// attributes, etc. are all ordinary exceptions in Python that
    /// `try`/`except` can catch, so this bootstrap models them the same way
    /// rather than making them uncatchable faults (which would betray
    /// Sylva's whole reason for having real exception handling in the first
    /// place).
    /// Binds one `for`/comprehension loop item to its target name list —
    /// a single name binds the whole item; multiple names unpack a `Tuple`
    /// (`for k, v in d.items():`), matching Python's iteration-unpacking.
    fn bind_for_target(&self, target: &[String], item: Value, scope: &Env, span: Span) -> Result<(), Flow> {
        if target.len() == 1 {
            scope.set(&target[0], item);
            return Ok(());
        }
        let Value::Tuple(elems) = &item else {
            return Err(self.rt(format!("cannot unpack non-tuple value into {} targets", target.len()), span));
        };
        if elems.len() != target.len() {
            return Err(self.rt(format!("too {} values to unpack (expected {})", if elems.len() > target.len() { "many" } else { "few" }, target.len()), span));
        }
        for (name, v) in target.iter().zip(elems.iter()) {
            scope.set(name, v.clone());
        }
        Ok(())
    }

    fn rt(&self, msg: impl Into<String>, _span: Span) -> Flow {
        Flow::Raise(Value::Str(std::rc::Rc::from(msg.into().as_str())))
    }

    pub fn print(&mut self, s: &str) {
        self.out.push_str(s);
    }

    /// Runs a module's statements top-to-bottom in the global scope — the
    /// real Python execution model, not a `register-then-call-main` model.
    pub fn run_module(&mut self, module: &Module) -> Result<i32, Box<OmniError>> {
        let env = self.globals.clone();
        for stmt in &module.body {
            match self.exec_stmt(stmt, &env) {
                Ok(_) => {}
                Err(Flow::Raise(v)) => {
                    return Err(Box::new(
                        OmniError::new(
                            Phase::Runtime,
                            format!("unhandled exception: {}", v.display()),
                            Span::point(1, 1),
                            &self.file,
                        )
                        .with_help("wrap the raising code in try/except to handle it"),
                    ));
                }
                Err(Flow::Return(_)) => return Err(Box::new(OmniError::new(Phase::Runtime, "'return' outside function", Span::point(1, 1), &self.file))),
                Err(Flow::Break) | Err(Flow::Continue) => {
                    return Err(Box::new(OmniError::new(Phase::Runtime, "'break'/'continue' outside loop", Span::point(1, 1), &self.file)))
                }
            }
        }
        Ok(0)
    }

    // ── statements ───────────────────────────────────────────────────────────

    fn exec_block(&mut self, stmts: &[Stmt], env: &Env) -> Result<(), Flow> {
        for s in stmts {
            self.exec_stmt(s, env)?;
        }
        Ok(())
    }

    fn exec_stmt(&mut self, stmt: &Stmt, env: &Env) -> Result<(), Flow> {
        match stmt {
            Stmt::Expr(e) => {
                self.eval(e, env)?;
                Ok(())
            }
            Stmt::Let { name, value, .. } => {
                let v = match value {
                    Some(e) => self.eval(e, env)?,
                    None => Value::None,
                };
                env.set(name, v);
                Ok(())
            }
            Stmt::Assign { target, value, .. } => {
                let v = self.eval(value, env)?;
                self.assign_target(target, v, env)
            }
            Stmt::If { branches, orelse, .. } => {
                for (cond, body) in branches {
                    if self.eval(cond, env)?.truthy() {
                        return self.exec_block(body, &env.child());
                    }
                }
                self.exec_block(orelse, &env.child())
            }
            Stmt::While { cond, body, orelse, .. } => {
                let mut ran_orelse_needed = true;
                while self.eval(cond, env)?.truthy() {
                    match self.exec_block(body, &env.child()) {
                        Ok(()) => {}
                        Err(Flow::Break) => {
                            ran_orelse_needed = false;
                            break;
                        }
                        Err(Flow::Continue) => continue,
                        Err(other) => return Err(other),
                    }
                }
                if ran_orelse_needed {
                    self.exec_block(orelse, &env.child())?;
                }
                Ok(())
            }
            Stmt::For { target, iter, body, orelse, span } => {
                let it = self.eval(iter, env)?;
                let items = self.iter_values(&it, *span)?;
                let mut broke = false;
                for item in items {
                    let scope = env.child();
                    self.bind_for_target(target, item, &scope, *span)?;
                    match self.exec_block(body, &scope) {
                        Ok(()) => {}
                        Err(Flow::Break) => {
                            broke = true;
                            break;
                        }
                        Err(Flow::Continue) => continue,
                        Err(other) => return Err(other),
                    }
                }
                if !broke {
                    self.exec_block(orelse, &env.child())?;
                }
                Ok(())
            }
            Stmt::FnDef(f) => {
                env.set(&f.name, Value::Function { def: Rc::new(f.clone()), env: env.clone() });
                Ok(())
            }
            Stmt::ClassDef(c) => {
                let class = self.build_class(c, env)?;
                env.set(&c.name, Value::Class(class));
                Ok(())
            }
            Stmt::Return { value, .. } => {
                let v = match value {
                    Some(e) => self.eval(e, env)?,
                    None => Value::None,
                };
                Err(Flow::Return(v))
            }
            Stmt::Break(_) => Err(Flow::Break),
            Stmt::Continue(_) => Err(Flow::Continue),
            Stmt::Pass => Ok(()),
            Stmt::Raise { exc, span } => {
                let v = match exc {
                    Some(e) => self.eval(e, env)?,
                    None => return Err(self.rt("no active exception to re-raise", *span)),
                };
                Err(Flow::Raise(v))
            }
            Stmt::Try { body, handlers, orelse, finally, .. } => {
                let result = self.exec_block(body, &env.child());
                let after_try = match result {
                    Ok(()) => {
                        let r = self.exec_block(orelse, &env.child());
                        r
                    }
                    Err(Flow::Raise(exc_val)) => self.run_handlers(handlers, exc_val, env),
                    Err(other) => Err(other),
                };
                // finally always runs, even if body/handler propagated.
                let finally_result = self.exec_block(finally, &env.child());
                finally_result?;
                after_try
            }
            Stmt::Import { .. } => Ok(()), // module system is future work; parsed, not enforced
            Stmt::Assert { cond, msg, span } => {
                if !self.eval(cond, env)?.truthy() {
                    let m = match msg {
                        Some(e) => self.eval(e, env)?.display(),
                        None => "assertion failed".to_string(),
                    };
                    return Err(self.rt(m, *span));
                }
                Ok(())
            }
            Stmt::Global { .. } => Ok(()), // scope-declaration hint only in this bootstrap
            Stmt::Del { target, span } => {
                if let Expr::Ident { name, .. } = target {
                    env.vars.borrow_mut().remove(name);
                    Ok(())
                } else {
                    Err(self.rt("'del' only supports plain names in this bootstrap", *span))
                }
            }
            // Parse-level-only omni-integration dialect extension (see
            // `ast::Stmt::ConfigBlock`) — not modeled as a runtime value.
            Stmt::ConfigBlock { .. } => Ok(()),
            // `mod`'s items are executed inline at the enclosing scope —
            // this bootstrap has no module/namespace system to give them
            // their own scope, which is fine for the parse-only `check` bar.
            Stmt::Mod { body, .. } => self.exec_block(body, env),
        }
    }

    fn run_handlers(&mut self, handlers: &[ExceptClause], exc_val: Value, env: &Env) -> Result<(), Flow> {
        for h in handlers {
            let matches = match &h.exc_type {
                None => true,
                Some(tn) => exception_matches(&exc_val, tn),
            };
            if !matches {
                continue;
            }
            let scope = env.child();
            if let Some(b) = &h.bind {
                scope.set(b, exc_val.clone());
            }
            return self.exec_block(&h.body, &scope);
        }
        Err(Flow::Raise(exc_val))
    }

    fn assign_target(&mut self, target: &Expr, value: Value, env: &Env) -> Result<(), Flow> {
        match target {
            Expr::Ident { name, .. } => {
                env.assign_existing_or_local(name, value);
                Ok(())
            }
            Expr::Attr { obj, name, span } => {
                let ov = self.eval(obj, env)?;
                match ov {
                    Value::Instance(inst) => {
                        inst.fields.borrow_mut().insert(name.clone(), value);
                        Ok(())
                    }
                    other => Err(self.rt(format!("cannot set attribute on {}", other.type_name()), *span)),
                }
            }
            Expr::Index { obj, index, span } => {
                let ov = self.eval(obj, env)?;
                let iv = self.eval(index, env)?;
                match ov {
                    Value::List(l) => {
                        let idx = self.list_index(&iv, l.borrow().len(), *span)?;
                        l.borrow_mut()[idx] = value;
                        Ok(())
                    }
                    Value::Dict(d) => {
                        let mut d = d.borrow_mut();
                        if let Some(entry) = d.iter_mut().find(|(k, _)| k.eq_val(&iv)) {
                            entry.1 = value;
                        } else {
                            d.push((iv, value));
                        }
                        Ok(())
                    }
                    other => Err(self.rt(format!("'{}' object does not support item assignment", other.type_name()), *span)),
                }
            }
            other => Err(self.rt("invalid assignment target", other.span())),
        }
    }

    // ── classes ──────────────────────────────────────────────────────────────

    fn build_class(&mut self, c: &ClassDef, env: &Env) -> Result<Rc<ClassVal>, Flow> {
        let base = if let Some(base_name) = c.bases.first() {
            match env.get(base_name) {
                Some(Value::Class(bc)) => Some(bc),
                _ => return Err(self.rt(format!("base class '{base_name}' not found"), c.span)),
            }
        } else {
            None
        };
        let mut methods = HashMap::new();
        for m in &c.methods {
            methods.insert(m.name.clone(), Rc::new(m.clone()));
        }
        let class_vars = RefCell::new(HashMap::new());
        let class = Rc::new(ClassVal { name: c.name.clone(), base, methods, class_vars });
        for (name, expr) in &c.class_vars {
            let v = self.eval(expr, env)?;
            class.class_vars.borrow_mut().insert(name.clone(), v);
        }
        Ok(class)
    }

    /// Instantiates `class(args)`: allocates the instance, then calls
    /// `__init__(self, ...)` if defined (real Python constructor semantics).
    fn instantiate(&mut self, class: Rc<ClassVal>, args: Vec<Value>, kwargs: Vec<(String, Value)>, span: Span) -> EResult {
        let inst = Rc::new(InstanceVal { class: class.clone(), fields: RefCell::new(HashMap::new()) });
        if let Some(init) = class.find_method("__init__") {
            let recv = Value::Instance(inst.clone());
            self.call_function(&init, self.globals.clone(), Some(recv), args, kwargs, span)?;
        }
        Ok(Value::Instance(inst))
    }

    // ── invocation ───────────────────────────────────────────────────────────

    fn call_function(
        &mut self,
        f: &Rc<FnDef>,
        closure_env: Env,
        self_val: Option<Value>,
        args: Vec<Value>,
        kwargs: Vec<(String, Value)>,
        span: Span,
    ) -> EResult {
        let scope = closure_env.child();
        // Skip the `self` parameter itself when binding positional args — it's
        // bound directly from `self_val`, not consumed from the call's
        // argument list (matches Python: `obj.method(x)` passes `x` as the
        // method's *second* parameter, `self` is implicit).
        let params: &[Param] = if self_val.is_some() && f.params.first().is_some_and(|p| p.name == "self") {
            &f.params[1..]
        } else {
            &f.params[..]
        };
        if let Some(sv) = self_val {
            scope.set("self", sv);
        }
        self.bind_params(params, args, kwargs, &scope, span)?;
        match self.exec_block(&f.body, &scope) {
            Ok(()) => Ok(Value::None),
            Err(Flow::Return(v)) => Ok(v),
            Err(other) => Err(other),
        }
    }

    fn bind_params(&mut self, params: &[Param], mut args: Vec<Value>, kwargs: Vec<(String, Value)>, scope: &Env, span: Span) -> Result<(), Flow> {
        let mut kwmap: HashMap<String, Value> = kwargs.into_iter().collect();
        let mut ai = 0;
        for p in params {
            if p.is_vararg {
                let rest: Vec<Value> = args.drain(ai..).collect();
                scope.set(&p.name, Value::List(Rc::new(RefCell::new(rest))));
                continue;
            }
            if p.is_kwarg {
                let entries: Vec<(Value, Value)> = kwmap.drain().map(|(k, v)| (Value::Str(Rc::from(k.as_str())), v)).collect();
                scope.set(&p.name, Value::Dict(Rc::new(RefCell::new(entries))));
                continue;
            }
            if let Some(v) = kwmap.remove(&p.name) {
                scope.set(&p.name, v);
            } else if ai < args.len() {
                scope.set(&p.name, args[ai].clone());
                ai += 1;
            } else if let Some(default) = &p.default {
                let v = self.eval(default, scope)?;
                scope.set(&p.name, v);
            } else {
                return Err(self.rt(format!("missing required argument '{}'", p.name), span));
            }
        }
        Ok(())
    }

    pub fn call_value(&mut self, callee: &Value, args: Vec<Value>, kwargs: Vec<(String, Value)>, span: Span) -> EResult {
        match callee {
            Value::Function { def, env } => self.call_function(def, env.clone(), None, args, kwargs, span),
            Value::BoundMethod { recv, def, env } => self.call_function(def, env.clone(), Some((**recv).clone()), args, kwargs, span),
            Value::Closure { params, body, env, is_lambda_expr } => {
                let scope = env.child();
                self.bind_params(params, args, kwargs, &scope, span)?;
                if let Some(expr) = is_lambda_expr {
                    self.eval(expr, &scope)
                } else {
                    match self.exec_block(body, &scope) {
                        Ok(()) => Ok(Value::None),
                        Err(Flow::Return(v)) => Ok(v),
                        Err(other) => Err(other),
                    }
                }
            }
            Value::Class(c) => self.instantiate(c.clone(), args, kwargs, span),
            Value::Builtin(name) => crate::builtins::call_builtin(self, name, args, span),
            other => Err(self.rt(format!("'{}' object is not callable", other.type_name()), span)),
        }
    }

    // ── expressions ──────────────────────────────────────────────────────────

    pub fn eval(&mut self, expr: &Expr, env: &Env) -> EResult {
        match expr {
            Expr::Int { v, .. } => Ok(Value::Int(*v)),
            Expr::Float { v, .. } => Ok(Value::Float(*v)),
            Expr::Str { v, .. } => Ok(Value::Str(Rc::from(v.as_str()))),
            Expr::Bool { v, .. } => Ok(Value::Bool(*v)),
            Expr::None_ { .. } => Ok(Value::None),
            Expr::FStr { parts, .. } => {
                let mut s = String::new();
                for part in parts {
                    match part {
                        FStrPart::Lit(t) => s.push_str(t),
                        FStrPart::Expr(e) => s.push_str(&self.eval(e, env)?.display()),
                    }
                }
                Ok(Value::Str(Rc::from(s.as_str())))
            }
            Expr::Ident { name, span } => env.get(name).ok_or_else(|| self.rt(format!("name '{name}' is not defined"), *span)),
            Expr::List { elems, .. } => {
                let mut v = Vec::with_capacity(elems.len());
                for e in elems {
                    v.push(self.eval(e, env)?);
                }
                Ok(Value::List(Rc::new(RefCell::new(v))))
            }
            Expr::Try { inner, .. } => self.eval(inner, env),
            // Parse-level-only omni-integration dialect extension (see
            // `ast::Expr::Match`'s doc comment) — evaluates the first arm
            // whose pattern name is `_` if any (a catch-all is nearly
            // always present and nearly always the intended fallback),
            // otherwise the first arm outright. Not real pattern matching.
            Expr::Match { arms, .. } => {
                let arm = arms.iter().find(|(name, ..)| name == "_").or_else(|| arms.first());
                match arm {
                    Some((_, _, body)) => self.eval(body, env),
                    None => Ok(Value::None),
                }
            }
            Expr::Repeat { value, count, span } => {
                let val = self.eval(value, env)?;
                let n = match self.eval(count, env)? {
                    Value::Int(n) => n,
                    other => return Err(self.rt(format!("'vec![value; count]' requires an integer count, got {}", other.type_name()), *span)),
                };
                Ok(Value::List(Rc::new(RefCell::new(vec![val; n.max(0) as usize]))))
            }
            Expr::Tuple { elems, .. } => {
                let mut v = Vec::with_capacity(elems.len());
                for e in elems {
                    v.push(self.eval(e, env)?);
                }
                Ok(Value::Tuple(Rc::new(v)))
            }
            Expr::Dict { entries, .. } => {
                let mut v = Vec::with_capacity(entries.len());
                for (k, val) in entries {
                    v.push((self.eval(k, env)?, self.eval(val, env)?));
                }
                Ok(Value::Dict(Rc::new(RefCell::new(v))))
            }
            Expr::ListComp { expr, target, iter, cond, span } => {
                let it = self.eval(iter, env)?;
                let items = self.iter_values(&it, iter.span())?;
                let mut out = Vec::new();
                for item in items {
                    let scope = env.child();
                    self.bind_for_target(target, item, &scope, *span)?;
                    if let Some(c) = cond {
                        if !self.eval(c, &scope)?.truthy() {
                            continue;
                        }
                    }
                    out.push(self.eval(expr, &scope)?);
                }
                Ok(Value::List(Rc::new(RefCell::new(out))))
            }
            Expr::DictComp { key, value, target, iter, cond, span } => {
                let it = self.eval(iter, env)?;
                let items = self.iter_values(&it, iter.span())?;
                let mut out = Vec::new();
                for item in items {
                    let scope = env.child();
                    self.bind_for_target(target, item, &scope, *span)?;
                    if let Some(c) = cond {
                        if !self.eval(c, &scope)?.truthy() {
                            continue;
                        }
                    }
                    out.push((self.eval(key, &scope)?, self.eval(value, &scope)?));
                }
                Ok(Value::Dict(Rc::new(RefCell::new(out))))
            }
            Expr::BinOp { op, left, right, span } => {
                let l = self.eval(left, env)?;
                let r = self.eval(right, env)?;
                self.binop(op, l, r, *span)
            }
            Expr::UnaryOp { op, expr, span } => {
                let v = self.eval(expr, env)?;
                match (op.as_str(), &v) {
                    ("-", Value::Int(n)) => Ok(Value::Int(-n)),
                    ("-", Value::Float(f)) => Ok(Value::Float(-f)),
                    ("+", Value::Int(_) | Value::Float(_)) => Ok(v),
                    _ => Err(self.rt(format!("bad operand type for unary {op}: '{}'", v.type_name()), *span)),
                }
            }
            Expr::BoolOp { op, left, right, .. } => {
                let l = self.eval(left, env)?;
                if op == "and" {
                    if !l.truthy() {
                        return Ok(l);
                    }
                    self.eval(right, env)
                } else {
                    if l.truthy() {
                        return Ok(l);
                    }
                    self.eval(right, env)
                }
            }
            Expr::Not { expr, .. } => Ok(Value::Bool(!self.eval(expr, env)?.truthy())),
            Expr::Compare { left, ops, comparators, span } => {
                let mut prev = self.eval(left, env)?;
                for (op, comp_expr) in ops.iter().zip(comparators.iter()) {
                    let cur = self.eval(comp_expr, env)?;
                    if !self.compare(op, &prev, &cur, *span)? {
                        return Ok(Value::Bool(false));
                    }
                    prev = cur;
                }
                Ok(Value::Bool(true))
            }
            Expr::Call { func, args, kwargs, span } => {
                let arg_vals = args.iter().map(|a| self.eval(a, env)).collect::<Result<Vec<_>, _>>()?;
                let mut kw_vals = Vec::with_capacity(kwargs.len());
                for (k, v) in kwargs {
                    kw_vals.push((k.clone(), self.eval(v, env)?));
                }
                // Method-call sugar: `obj.method(args)` binds `self` without
                // materializing a BoundMethod for the common case.
                if let Expr::Attr { obj, name, span: aspan } = func.as_ref() {
                    let recv = self.eval(obj, env)?;
                    return self.call_method(recv, name, arg_vals, kw_vals, *aspan);
                }
                let callee = self.eval(func, env)?;
                self.call_value(&callee, arg_vals, kw_vals, *span)
            }
            Expr::Attr { obj, name, span } => {
                let ov = self.eval(obj, env)?;
                self.get_attr(ov, name, *span)
            }
            Expr::Index { obj, index, span } => {
                let ov = self.eval(obj, env)?;
                let iv = self.eval(index, env)?;
                self.index_get(ov, iv, *span)
            }
            Expr::Slice { obj, lo, hi, step, span } => {
                let ov = self.eval(obj, env)?;
                let lo = lo.as_ref().map(|e| self.eval(e, env)).transpose()?;
                let hi = hi.as_ref().map(|e| self.eval(e, env)).transpose()?;
                let step = step.as_ref().map(|e| self.eval(e, env)).transpose()?;
                self.slice_get(ov, lo, hi, step, *span)
            }
            Expr::Lambda { params, body, .. } => Ok(Value::Closure {
                params: Rc::new(params.clone()),
                body: Rc::new(vec![]),
                env: env.clone(),
                is_lambda_expr: Some(Rc::new((**body).clone())),
            }),
            Expr::Ternary { body, cond, orelse, .. } => {
                if self.eval(cond, env)?.truthy() {
                    self.eval(body, env)
                } else {
                    self.eval(orelse, env)
                }
            }
            Expr::Await { expr, .. } => self.eval(expr, env), // no real event loop yet — awaits its value synchronously
            Expr::Yield { .. } => Err(self.rt(
                "generators ('yield') are not implemented in this bootstrap — real lazy-suspension semantics need a coroutine-capable evaluator, not faked here",
                expr.span(),
            )),
        }
    }

    fn call_method(&mut self, recv: Value, name: &str, args: Vec<Value>, kwargs: Vec<(String, Value)>, span: Span) -> EResult {
        if let Value::Instance(inst) = &recv {
            if let Some(m) = inst.class.find_method(name) {
                return self.call_function(&m, self.globals.clone(), Some(recv.clone()), args, kwargs, span);
            }
        }
        if let Value::Class(c) = &recv {
            if let Some(m) = c.find_method(name) {
                // Called on the class itself (no instance) — static-style call.
                return self.call_function(&m, self.globals.clone(), None, args, kwargs, span);
            }
        }
        crate::builtins::call_method_builtin(self, &recv, name, args, span)
    }

    fn get_attr(&mut self, recv: Value, name: &str, span: Span) -> EResult {
        match &recv {
            Value::Instance(inst) => {
                if let Some(v) = inst.fields.borrow().get(name) {
                    return Ok(v.clone());
                }
                if let Some(m) = inst.class.find_method(name) {
                    return Ok(Value::BoundMethod { recv: Box::new(recv.clone()), def: m, env: self.globals.clone() });
                }
                if let Some(v) = inst.class.class_vars.borrow().get(name) {
                    return Ok(v.clone());
                }
                Err(self.rt(format!("'{}' object has no attribute '{}'", inst.class.name, name), span))
            }
            Value::Class(c) => {
                if let Some(v) = c.class_vars.borrow().get(name) {
                    return Ok(v.clone());
                }
                if let Some(m) = c.find_method(name) {
                    return Ok(Value::Function { def: m, env: self.globals.clone() });
                }
                Err(self.rt(format!("class '{}' has no attribute '{}'", c.name, name), span))
            }
            Value::Tensor(t) if name == "shape" => Ok(Value::List(Rc::new(RefCell::new(t.shape.iter().map(|&n| Value::Int(n as i64)).collect())))),
            _ => Err(self.rt(format!("'{}' object has no attribute '{}'", recv.type_name(), name), span)),
        }
    }

    fn index_get(&mut self, obj: Value, idx: Value, span: Span) -> EResult {
        match obj {
            Value::List(l) => {
                let l = l.borrow();
                let i = self.list_index(&idx, l.len(), span)?;
                Ok(l[i].clone())
            }
            Value::Tuple(t) => {
                let i = self.list_index(&idx, t.len(), span)?;
                Ok(t[i].clone())
            }
            Value::Str(s) => {
                let chars: Vec<char> = s.chars().collect();
                let i = self.list_index(&idx, chars.len(), span)?;
                Ok(Value::Str(Rc::from(chars[i].to_string().as_str())))
            }
            Value::Dict(d) => {
                d.borrow().iter().find(|(k, _)| k.eq_val(&idx)).map(|(_, v)| v.clone()).ok_or_else(|| self.rt(format!("KeyError: {}", idx.repr()), span))
            }
            other => Err(self.rt(format!("'{}' object is not subscriptable", other.type_name()), span)),
        }
    }

    fn slice_get(&mut self, obj: Value, lo: Option<Value>, hi: Option<Value>, step: Option<Value>, span: Span) -> EResult {
        let step_n = step.as_ref().map(|v| v.as_f64().unwrap_or(1.0) as i64).unwrap_or(1);
        if step_n != 1 {
            return Err(self.rt("stepped slicing is not implemented in this bootstrap", span));
        }
        let len_of = |n: usize| n as i64;
        let clamp = |v: i64, len: i64| -> usize {
            let v = if v < 0 { (v + len).max(0) } else { v.min(len) };
            v as usize
        };
        match obj {
            Value::List(l) => {
                let len = len_of(l.borrow().len());
                let lo = lo.as_ref().and_then(|v| v.as_f64()).map(|f| f as i64).unwrap_or(0);
                let hi = hi.as_ref().and_then(|v| v.as_f64()).map(|f| f as i64).unwrap_or(len);
                let (lo, hi) = (clamp(lo, len), clamp(hi, len));
                let l = l.borrow();
                Ok(Value::List(Rc::new(RefCell::new(if lo < hi { l[lo..hi].to_vec() } else { vec![] }))))
            }
            Value::Str(s) => {
                let chars: Vec<char> = s.chars().collect();
                let len = len_of(chars.len());
                let lo = lo.as_ref().and_then(|v| v.as_f64()).map(|f| f as i64).unwrap_or(0);
                let hi = hi.as_ref().and_then(|v| v.as_f64()).map(|f| f as i64).unwrap_or(len);
                let (lo, hi) = (clamp(lo, len), clamp(hi, len));
                let s: String = if lo < hi { chars[lo..hi].iter().collect() } else { String::new() };
                Ok(Value::Str(Rc::from(s.as_str())))
            }
            other => Err(self.rt(format!("'{}' object is not sliceable", other.type_name()), span)),
        }
    }

    fn list_index(&self, idx: &Value, len: usize, span: Span) -> Result<usize, Flow> {
        let n = idx.as_f64().ok_or_else(|| self.rt("list index must be an integer", span))? as i64;
        let n = if n < 0 { n + len as i64 } else { n };
        if n < 0 || n as usize >= len {
            return Err(self.rt("list index out of range", span));
        }
        Ok(n as usize)
    }

    fn iter_values(&mut self, v: &Value, span: Span) -> Result<Vec<Value>, Flow> {
        match v {
            Value::List(l) => Ok(l.borrow().clone()),
            Value::Tuple(t) => Ok((**t).clone()),
            Value::Str(s) => Ok(s.chars().map(|c| Value::Str(Rc::from(c.to_string().as_str()))).collect()),
            Value::Dict(d) => Ok(d.borrow().iter().map(|(k, _)| k.clone()).collect()),
            other => Err(self.rt(format!("'{}' object is not iterable", other.type_name()), span)),
        }
    }

    fn binop(&mut self, op: &str, l: Value, r: Value, span: Span) -> EResult {
        use Value::*;
        match (op, &l, &r) {
            ("+", Str(a), Str(b)) => Ok(Str(Rc::from(format!("{a}{b}").as_str()))),
            ("+", List(a), List(b)) => {
                let mut v = a.borrow().clone();
                v.extend(b.borrow().iter().cloned());
                Ok(List(Rc::new(RefCell::new(v))))
            }
            ("*", Str(a), Int(n)) | ("*", Int(n), Str(a)) => Ok(Str(Rc::from(a.repeat((*n).max(0) as usize).as_str()))),
            ("*", List(a), Int(n)) | ("*", Int(n), List(a)) => {
                let mut v = Vec::new();
                for _ in 0..(*n).max(0) {
                    v.extend(a.borrow().iter().cloned());
                }
                Ok(List(Rc::new(RefCell::new(v))))
            }
            _ => {
                if let (Int(a), Int(b)) = (&l, &r) {
                    return match op {
                        "+" => Ok(Int(a + b)),
                        "-" => Ok(Int(a - b)),
                        "*" => Ok(Int(a * b)),
                        "/" => {
                            if *b == 0 {
                                Err(self.rt("division by zero", span))
                            } else {
                                Ok(Float(*a as f64 / *b as f64))
                            }
                        }
                        "%" => {
                            if *b == 0 {
                                Err(self.rt("modulo by zero", span))
                            } else {
                                Ok(Int(a.rem_euclid(*b)))
                            }
                        }
                        "**" => Ok(if *b >= 0 { Int(a.pow((*b) as u32)) } else { Float((*a as f64).powi(*b as i32)) }),
                        "^" => Ok(Int(a ^ b)),
                        "|" => Ok(Int(a | b)),
                        "&" => Ok(Int(a & b)),
                        "<<" => Ok(Int(a.wrapping_shl(*b as u32))),
                        ">>" => Ok(Int(a.wrapping_shr(*b as u32))),
                        _ => Err(self.rt(format!("unsupported operator '{op}'"), span)),
                    };
                }
                let (Some(fa), Some(fb)) = (l.as_f64(), r.as_f64()) else {
                    return Err(self.rt(format!("unsupported operand type(s) for {op}: '{}' and '{}'", l.type_name(), r.type_name()), span));
                };
                match op {
                    "+" => Ok(Float(fa + fb)),
                    "-" => Ok(Float(fa - fb)),
                    "*" => Ok(Float(fa * fb)),
                    "/" => {
                        if fb == 0.0 {
                            Err(self.rt("division by zero", span))
                        } else {
                            Ok(Float(fa / fb))
                        }
                    }
                    "%" => Ok(Float(fa.rem_euclid(fb))),
                    "**" => Ok(Float(fa.powf(fb))),
                    _ => Err(self.rt(format!("unsupported operator '{op}'"), span)),
                }
            }
        }
    }

    fn compare(&mut self, op: &str, l: &Value, r: &Value, span: Span) -> Result<bool, Flow> {
        Ok(match op {
            "==" => l.eq_val(r),
            "!=" => !l.eq_val(r),
            "is" => l.eq_val(r),
            "is not" => !l.eq_val(r),
            "in" => self.iter_values(r, span)?.iter().any(|x| x.eq_val(l)),
            "not in" => !self.iter_values(r, span)?.iter().any(|x| x.eq_val(l)),
            "<" | "<=" | ">" | ">=" => {
                let ord = match (l, r) {
                    (Value::Str(a), Value::Str(b)) => a.as_ref().partial_cmp(b.as_ref()),
                    _ => {
                        let (Some(a), Some(b)) = (l.as_f64(), r.as_f64()) else {
                            return Err(self.rt(format!("'{}' not supported between instances of '{}' and '{}'", op, l.type_name(), r.type_name()), span));
                        };
                        a.partial_cmp(&b)
                    }
                };
                let Some(ord) = ord else { return Ok(false) };
                match op {
                    "<" => ord.is_lt(),
                    "<=" => ord.is_le(),
                    ">" => ord.is_gt(),
                    ">=" => ord.is_ge(),
                    _ => unreachable!(),
                }
            }
            _ => return Err(self.rt(format!("unsupported comparison '{op}'"), span)),
        })
    }
}

/// Matches an `except TypeName:` clause: by raised-instance's class name (and
/// its ancestor chain), or by literal string equality when a bare string was
/// raised (this bootstrap has no built-in `Exception` hierarchy yet).
fn exception_matches(exc_val: &Value, type_name: &str) -> bool {
    match exc_val {
        Value::Instance(inst) => inst.class.is_or_extends(type_name),
        _ => type_name == "Exception" || type_name == "BaseException",
    }
}
