//! Aether tree-walking interpreter.
//!
//! Structural differences from Titan and Sylva (deliberate, per the same
//! per-language-uniqueness mandate that drove Sylva's redesign):
//!   - **Multi-clause function dispatch**: functions sharing (name, arity)
//!     are grouped into one `FnClauses` value; a call tries each clause's
//!     pattern + guard in order and runs the first match — this is genuine
//!     Erlang/Elixir semantics, structurally different from Titan's single
//!     body (with an internal `match`) and Sylva's single dynamic body.
//!   - **Actors are cooperative/synchronous** in this bootstrap (documented
//!     in `values.rs::ActorInstance`) — `send` directly runs the matching
//!     `receive` clause and returns, no real scheduler.
//!   - No exceptions, no `Result`/`?` — Aether errors surface as `{:error,
//!     reason}` tuples by convention (Erlang idiom), checked with `case`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::ast::*;
use crate::diag::{OmniError, Phase, Span};
use crate::values::*;

pub enum Flow {
    Return(Value),
    Error(Box<OmniError>),
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

    fn rt(&self, msg: impl Into<String>, span: Span) -> Flow {
        Flow::Error(Box::new(OmniError::new(Phase::Runtime, msg, span, &self.file)))
    }

    pub fn print(&mut self, s: &str) {
        self.out.push_str(s);
    }

    /// Registers every top-level item, then runs any bare top-level
    /// expression statements in order (script-style, like Sylva — Aether
    /// doesn't require a `main` entry point either).
    pub fn run_module(&mut self, module: &Module) -> Result<i32, Box<OmniError>> {
        let env = self.globals.clone();
        // Pass 1: register all fn clauses / actors (so forward references
        // between top-level functions work regardless of definition order —
        // matches real Erlang module semantics, where all functions in a
        // module are mutually visible).
        for item in &module.items {
            match item {
                Item::FnClause(c) => self.register_fn_clause(c, &env),
                Item::ActorDef(a) => self.register_actor_def(a, &env),
                Item::TopStmt(_) => {}
            }
        }
        // Pass 2: execute top-level statements top-to-bottom.
        for item in &module.items {
            if let Item::TopStmt(s) = item {
                match self.exec_stmt(s, &env) {
                    Ok(_) => {}
                    Err(Flow::Error(err)) => return Err(err),
                    Err(Flow::Return(_)) => {}
                }
            }
        }
        Ok(0)
    }

    fn register_fn_clause(&mut self, c: &FnClause, env: &Env) {
        let entry = env.get(&c.name);
        let clauses = match entry {
            Some(Value::FnClauses(existing)) => {
                let mut cl = existing.clauses.clone();
                cl.push(Rc::new(c.clone()));
                cl
            }
            _ => vec![Rc::new(c.clone())],
        };
        env.set(&c.name, Value::FnClauses(Rc::new(FnClauses { name: c.name.clone(), clauses, env: env.clone() })));
    }

    fn register_actor_def(&mut self, a: &ActorDef, env: &Env) {
        env.set(&format!("__actordef__{}", a.name), Value::Actor(Rc::new(ActorInstance { def: Rc::new(a.clone()), state: RefCell::new(Value::Nil), env: env.clone() })));
    }

    // ── statements ───────────────────────────────────────────────────────────

    fn exec_block(&mut self, stmts: &[Stmt], env: &Env) -> Result<Value, Flow> {
        let mut last = Value::Nil;
        for s in stmts {
            last = self.exec_stmt(s, env)?;
        }
        Ok(last)
    }

    fn exec_stmt(&mut self, stmt: &Stmt, env: &Env) -> EResult {
        match stmt {
            Stmt::Expr(e) => self.eval(e, env),
            Stmt::Assign { name, value, .. } => {
                let v = self.eval(value, env)?;
                env.assign_existing_or_local(name, v.clone());
                Ok(v)
            }
            Stmt::If { branches, orelse, .. } => self.eval_if(branches, orelse, env),
            Stmt::Case { scrut, arms, span } => {
                let v = self.eval(scrut, env)?;
                self.eval_case(&v, arms, env, *span)
            }
            Stmt::For { var, iter, body, span } => {
                let it = self.eval(iter, env)?;
                let items = self.iter_values(&it, *span)?;
                let mut last = Value::Nil;
                for item in items {
                    let scope = env.child();
                    scope.set(var, item);
                    last = self.exec_block(body, &scope)?;
                }
                Ok(last)
            }
            Stmt::Return { value, .. } => {
                let v = match value {
                    Some(e) => self.eval(e, env)?,
                    None => Value::Nil,
                };
                Err(Flow::Return(v))
            }
        }
    }

    /// Shared by `Stmt::If` and `Expr::If` — `if` is a real expression in
    /// Aether (Elixir idiom: `x = if cond do a else b end`), not statement-only.
    fn eval_if(&mut self, branches: &[(Expr, Vec<Stmt>)], orelse: &[Stmt], env: &Env) -> EResult {
        for (cond, body) in branches {
            if self.eval(cond, env)?.truthy() {
                return self.exec_block(body, &env.child());
            }
        }
        self.exec_block(orelse, &env.child())
    }

    /// Shared by `Stmt::Case` and `Expr::Case` — see `eval_if`'s doc comment.
    fn eval_case(&mut self, v: &Value, arms: &[CaseArm], env: &Env, span: Span) -> EResult {
        for arm in arms {
            let scope = env.child();
            if !self.match_pattern(&arm.pattern, v, &scope) {
                continue;
            }
            if let Some(g) = &arm.guard {
                if !self.eval(g, &scope)?.truthy() {
                    continue;
                }
            }
            return self.exec_block(&arm.body, &scope);
        }
        Err(self.rt(format!("no case clause matches {}", v.display()), span))
    }

    // ── pattern matching ──────────────────────────────────────────────────────

    fn match_pattern(&mut self, pat: &Pattern, v: &Value, scope: &Env) -> bool {
        match pat {
            Pattern::Wild => true,
            Pattern::Bind(name) => {
                scope.set(name, v.clone());
                true
            }
            Pattern::Lit(e) => {
                // Literal patterns never reference bindings, so evaluating
                // against the call scope (which has nothing bound yet at
                // this point) is safe and matches Erlang's literal-pattern
                // semantics (no variables allowed inside a literal pattern).
                match self.eval(e, scope) {
                    Ok(lit) => lit.eq_val(v),
                    Err(_) => false,
                }
            }
            Pattern::Atom(name) => matches!(v, Value::Atom(a) if a.as_ref() == name),
            Pattern::Tuple(pats) => {
                let Value::Tuple(vals) = v else { return false };
                vals.len() == pats.len() && pats.iter().zip(vals.iter()).all(|(p, x)| self.match_pattern(p, x, scope))
            }
            Pattern::List(pats) => {
                let Value::List(vals) = v else { return false };
                let vals = vals.borrow();
                vals.len() == pats.len() && pats.iter().zip(vals.iter()).all(|(p, x)| self.match_pattern(p, x, scope))
            }
            Pattern::Cons(head, tail) => {
                let Value::List(vals) = v else { return false };
                let vals = vals.borrow();
                if vals.is_empty() {
                    return false;
                }
                if !self.match_pattern(head, &vals[0], scope) {
                    return false;
                }
                let rest = Value::List(Rc::new(RefCell::new(vals[1..].to_vec())));
                self.match_pattern(tail, &rest, scope)
            }
        }
    }

    // ── function dispatch (the core Erlang-flavored capability) ──────────────

    /// Tries each clause's pattern (against positional args) + guard, in
    /// definition order; runs the first match's body. This is the real,
    /// distinguishing multi-clause dispatch — not sugar over a `match`.
    fn call_fn_clauses(&mut self, fc: &Rc<FnClauses>, args: &[Value], span: Span) -> EResult {
        'clauses: for clause in &fc.clauses {
            if clause.params.len() != args.len() {
                continue;
            }
            let scope = fc.env.child();
            for (p, a) in clause.params.iter().zip(args.iter()) {
                if !self.match_pattern(p, a, &scope) {
                    continue 'clauses;
                }
            }
            if let Some(g) = &clause.guard {
                if !self.eval(g, &scope)?.truthy() {
                    continue;
                }
            }
            return match self.exec_block(&clause.body, &scope) {
                Ok(v) => Ok(v),
                Err(Flow::Return(v)) => Ok(v),
                Err(other) => Err(other),
            };
        }
        Err(self.rt(format!("no function clause of '{}'/{} matches the given arguments", fc.name, args.first().map(|_| args.len()).unwrap_or(0)), span))
    }

    pub fn call_value(&mut self, callee: &Value, args: Vec<Value>, span: Span) -> EResult {
        match callee {
            Value::FnClauses(fc) => self.call_fn_clauses(fc, &args, span),
            Value::Lambda { params, body, env } => {
                let scope = env.child();
                if params.len() != args.len() {
                    return Err(self.rt(format!("lambda expects {} argument(s), got {}", params.len(), args.len()), span));
                }
                for (p, a) in params.iter().zip(args.iter()) {
                    if !self.match_pattern(p, a, &scope) {
                        return Err(self.rt("lambda argument does not match pattern", span));
                    }
                }
                self.eval(body, &scope)
            }
            Value::Builtin(name) => crate::builtins::call_builtin(self, name, args, span),
            other => Err(self.rt(format!("value of type '{}' is not callable", other.type_name()), span)),
        }
    }

    // ── actors ─────────────────────────────────────────────────────────────────

    fn spawn_actor(&mut self, actor_name: &str, args: Vec<Value>, env: &Env, span: Span) -> EResult {
        let Some(Value::Actor(template)) = env.get(&format!("__actordef__{actor_name}")) else {
            return Err(self.rt(format!("unknown actor '{actor_name}'"), span));
        };
        // `start`/1 (or /N) initializes state, matching gen_server's `init/1`
        // idiom — if the actor defines a `start` function clause set, call it
        // with the spawn args to compute initial state; otherwise use the
        // first arg (or nil) directly as state.
        let start_fn = template.def.fns.iter().find(|f| f.name == "start" && f.params.len() == args.len());
        let initial_state = if let Some(f) = start_fn {
            let scope = template.env.child();
            for (p, a) in f.params.iter().zip(args.iter()) {
                self.match_pattern(p, a, &scope);
            }
            self.exec_block(&f.body, &scope)?
        } else {
            args.into_iter().next().unwrap_or(Value::Nil)
        };
        Ok(Value::Actor(Rc::new(ActorInstance { def: template.def.clone(), state: RefCell::new(initial_state), env: template.env.clone() })))
    }

    /// `send(actor, msg)` — synchronously matches `msg` against the actor's
    /// `receive` clauses and runs the first match, updating `state` in
    /// place (see `ActorInstance`'s doc comment for the honesty note on
    /// this being cooperative, not preemptive).
    fn send_to_actor(&mut self, actor: &Rc<ActorInstance>, msg: Value, span: Span) -> EResult {
        for rc in &actor.def.receives {
            let scope = actor.env.child();
            if !self.match_pattern(&rc.msg_pattern, &msg, &scope) {
                continue;
            }
            scope.set(&rc.state_binding, actor.state.borrow().clone());
            if let Some(g) = &rc.guard {
                if !self.eval(g, &scope)?.truthy() {
                    continue;
                }
            }
            let new_state = self.exec_block(&rc.body, &scope)?;
            *actor.state.borrow_mut() = new_state.clone();
            return Ok(new_state);
        }
        Err(self.rt(format!("actor '{}' has no receive clause matching {}", actor.def.name, msg.display()), span))
    }

    // ── expressions ──────────────────────────────────────────────────────────

    pub fn eval(&mut self, expr: &Expr, env: &Env) -> EResult {
        match expr {
            Expr::Int { v, .. } => Ok(Value::Int(*v)),
            Expr::Float { v, .. } => Ok(Value::Float(*v)),
            Expr::Str { v, .. } => Ok(Value::Str(Rc::from(v.as_str()))),
            Expr::Bool { v, .. } => Ok(Value::Bool(*v)),
            Expr::Nil { .. } => Ok(Value::Nil),
            Expr::Atom { name, .. } => Ok(Value::Atom(Rc::from(name.as_str()))),
            Expr::IStr { parts, .. } => {
                let mut s = String::new();
                for part in parts {
                    match part {
                        IStrPart::Lit(t) => s.push_str(t),
                        IStrPart::Expr(e) => s.push_str(&self.eval(e, env)?.display()),
                    }
                }
                Ok(Value::Str(Rc::from(s.as_str())))
            }
            Expr::Ident { name, span } => env.get(name).ok_or_else(|| self.rt(format!("undefined variable or function '{name}'"), *span)),
            Expr::Tuple { elems, .. } => {
                let mut v = Vec::with_capacity(elems.len());
                for e in elems {
                    v.push(self.eval(e, env)?);
                }
                Ok(Value::Tuple(Rc::new(v)))
            }
            Expr::List { elems, .. } => {
                let mut v = Vec::with_capacity(elems.len());
                for e in elems {
                    v.push(self.eval(e, env)?);
                }
                Ok(Value::List(Rc::new(RefCell::new(v))))
            }
            Expr::Map { entries, .. } => {
                let mut v = Vec::with_capacity(entries.len());
                for (k, val) in entries {
                    v.push((self.eval(k, env)?, self.eval(val, env)?));
                }
                Ok(Value::Map(Rc::new(RefCell::new(v))))
            }
            Expr::BinOp { op, left, right, span } => {
                let l = self.eval(left, env)?;
                if op == "and" {
                    return if !l.truthy() { Ok(l) } else { self.eval(right, env) };
                }
                if op == "or" {
                    return if l.truthy() { Ok(l) } else { self.eval(right, env) };
                }
                let r = self.eval(right, env)?;
                self.binop(op, l, r, *span)
            }
            Expr::UnaryOp { op, expr, span } => {
                let v = self.eval(expr, env)?;
                match (op.as_str(), &v) {
                    ("-", Value::Int(n)) => Ok(Value::Int(-n)),
                    ("-", Value::Float(f)) => Ok(Value::Float(-f)),
                    ("not", _) => Ok(Value::Bool(!v.truthy())),
                    _ => Err(self.rt(format!("bad operand for unary '{op}'"), *span)),
                }
            }
            Expr::Call { func, args, span } => {
                let arg_vals = args.iter().map(|a| self.eval(a, env)).collect::<Result<Vec<_>, _>>()?;
                if let Expr::Ident { name, .. } = func.as_ref() {
                    if name == "send" && arg_vals.len() == 2 {
                        let Value::Actor(actor) = &arg_vals[0] else {
                            return Err(self.rt("send/2's first argument must be an actor", *span));
                        };
                        return self.send_to_actor(actor, arg_vals[1].clone(), *span);
                    }
                }
                let callee = self.eval(func, env)?;
                self.call_value(&callee, arg_vals, *span)
            }
            Expr::Attr { obj, name, span } => {
                let ov = self.eval(obj, env)?;
                match &ov {
                    Value::Actor(a) if name == "state" => Ok(a.state.borrow().clone()),
                    _ => Err(self.rt(format!("'{}' has no attribute '{}'", ov.type_name(), name), *span)),
                }
            }
            Expr::Index { obj, index, span } => {
                let ov = self.eval(obj, env)?;
                let iv = self.eval(index, env)?;
                self.index_get(ov, iv, *span)
            }
            Expr::Spawn { actor, args, span } => {
                let arg_vals = args.iter().map(|a| self.eval(a, env)).collect::<Result<Vec<_>, _>>()?;
                self.spawn_actor(actor, arg_vals, env, *span)
            }
            Expr::Lambda { params, body, .. } => Ok(Value::Lambda { params: Rc::new(params.clone()), body: Rc::new((**body).clone()), env: env.clone() }),
            Expr::Case { scrut, arms, span } => {
                let v = self.eval(scrut, env)?;
                self.eval_case(&v, arms, env, *span)
            }
            Expr::If { branches, orelse, .. } => self.eval_if(branches, orelse, env),
        }
    }

    fn index_get(&mut self, obj: Value, idx: Value, span: Span) -> EResult {
        match obj {
            Value::List(l) => {
                let l = l.borrow();
                let n = idx.as_f64().ok_or_else(|| self.rt("list index must be numeric", span))? as i64;
                let n = if n < 0 { n + l.len() as i64 } else { n };
                if n < 0 || n as usize >= l.len() {
                    return Err(self.rt("list index out of range", span));
                }
                Ok(l[n as usize].clone())
            }
            Value::Tuple(t) => {
                let n = idx.as_f64().ok_or_else(|| self.rt("tuple index must be numeric", span))? as i64;
                if n < 0 || n as usize >= t.len() {
                    return Err(self.rt("tuple index out of range", span));
                }
                Ok(t[n as usize].clone())
            }
            Value::Map(m) => m.borrow().iter().find(|(k, _)| k.eq_val(&idx)).map(|(_, v)| v.clone()).ok_or_else(|| self.rt(format!("key not found: {}", idx.display()), span)),
            other => Err(self.rt(format!("'{}' is not indexable", other.type_name()), span)),
        }
    }

    fn iter_values(&mut self, v: &Value, span: Span) -> Result<Vec<Value>, Flow> {
        match v {
            Value::List(l) => Ok(l.borrow().clone()),
            other => Err(self.rt(format!("'{}' is not iterable", other.type_name()), span)),
        }
    }

    fn binop(&mut self, op: &str, l: Value, r: Value, span: Span) -> EResult {
        use Value::*;
        match (op, &l, &r) {
            ("+", Str(a), Str(b)) => return Ok(Str(Rc::from(format!("{a}{b}").as_str()))),
            ("+", List(a), List(b)) => {
                let mut v = a.borrow().clone();
                v.extend(b.borrow().iter().cloned());
                return Ok(List(Rc::new(RefCell::new(v))));
            }
            ("==", _, _) => return Ok(Bool(l.eq_val(&r))),
            ("!=", _, _) => return Ok(Bool(!l.eq_val(&r))),
            _ => {}
        }
        if matches!(op, "<" | "<=" | ">" | ">=") {
            let ord = match (&l, &r) {
                (Str(a), Str(b)) => a.as_ref().partial_cmp(b.as_ref()),
                _ => {
                    let (Some(a), Some(b)) = (l.as_f64(), r.as_f64()) else {
                        return Err(self.rt(format!("cannot compare '{}' and '{}'", l.type_name(), r.type_name()), span));
                    };
                    a.partial_cmp(&b)
                }
            };
            let Some(ord) = ord else { return Ok(Bool(false)) };
            return Ok(Bool(match op {
                "<" => ord.is_lt(),
                "<=" => ord.is_le(),
                ">" => ord.is_gt(),
                ">=" => ord.is_ge(),
                _ => unreachable!(),
            }));
        }
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
            _ => Err(self.rt(format!("unsupported operator '{op}'"), span)),
        }
    }
}
