//! Vera tree-walking interpreter.
//!
//! Structural differences from Titan/Sylva/Aether (deliberate, per the
//! per-language-uniqueness mandate): components are the unit of definition
//! (not free functions/classes), state is reactive-by-storage (every
//! binding is a shared cell — see `values.rs`), and `render` evaluates an
//! **embedded markup tree** into a real `Element` value tree rather than
//! calling into a builder API. There's no actual browser/GPU here, so
//! "real" is defined as: render produces a genuine data structure, methods
//! genuinely mutate the same state cells render reads, and a second render
//! after a method call textually differs in exactly the way it should —
//! all verified by tests, not asserted.

use std::cell::RefCell;
use std::collections::HashMap;
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
    pub components: HashMap<String, Rc<ComponentDef>>,
    pub out: String,
}

impl Interp {
    pub fn new(file: &str) -> Self {
        Interp { file: file.to_string(), globals: Scope::new(), components: HashMap::new(), out: String::new() }
    }

    fn rt(&self, msg: impl Into<String>, span: Span) -> Flow {
        Flow::Error(Box::new(OmniError::new(Phase::Runtime, msg, span, &self.file)))
    }

    pub fn print(&mut self, s: &str) {
        self.out.push_str(s);
    }

    pub fn run_module(&mut self, module: &Module) -> Result<i32, Box<OmniError>> {
        for c in &module.components {
            let rc = Rc::new(c.clone());
            self.components.insert(c.name.clone(), rc.clone());
            self.globals.declare(&c.name, Value::ComponentClass(rc));
        }
        let env = self.globals.clone();
        for s in &module.script {
            match self.exec_stmt(s, &env) {
                Ok(_) => {}
                Err(Flow::Error(e)) => return Err(e),
                Err(Flow::Return(_)) => {}
            }
        }
        Ok(0)
    }

    // ── component instantiation & rendering ───────────────────────────────────

    /// `mount(ComponentClass, prop_values...)` — allocates the instance's
    /// env (props bound positionally by the component's declared prop
    /// order, state initialized from each `state` declaration's expr,
    /// computed values populated by `refresh_computed`).
    fn mount(&mut self, def: &Rc<ComponentDef>, prop_values: Vec<Value>, span: Span) -> Result<Rc<ComponentInstance>, Flow> {
        let env = Scope::new();
        // Documented simplification: props are matched positionally against
        // the component's declared prop order; a missing prop value simply
        // isn't bound rather than erroring, since Vera has no static
        // type/arity checker yet.
        for (name, val) in def.props.iter().zip(prop_values.into_iter()) {
            env.declare(name, val);
        }
        for (name, init) in &def.state {
            let v = self.eval(init, &env)?;
            env.declare(name, v);
        }
        let instance = Rc::new(ComponentInstance { def: def.clone(), env });
        self.refresh_computed(&instance, span)?;
        Ok(instance)
    }

    fn refresh_computed(&mut self, instance: &Rc<ComponentInstance>, _span: Span) -> Result<(), Flow> {
        for (name, expr) in &instance.def.computed {
            let v = self.eval(expr, &instance.env)?;
            instance.env.declare(name, v);
        }
        Ok(())
    }

    /// Renders a mounted instance's `render` block into a `Value` tree
    /// (usually one `Element`, or a synthetic `fragment` wrapper if the
    /// block has 0 or >1 top-level nodes).
    fn render_instance(&mut self, instance: &Rc<ComponentInstance>, span: Span) -> EResult {
        self.refresh_computed(instance, span)?;
        let mut nodes = Vec::new();
        for n in &instance.def.render {
            self.render_node(n, &instance.env, &mut nodes)?;
        }
        if nodes.len() == 1 {
            Ok(nodes.into_iter().next().unwrap())
        } else {
            Ok(Value::Element(Rc::new(ElementVal { tag: "fragment".to_string(), attrs: vec![], children: nodes })))
        }
    }

    fn render_node(&mut self, node: &Node, env: &Env, out: &mut Vec<Value>) -> Result<(), Flow> {
        match node {
            Node::Text(s) => {
                out.push(Value::Str(Rc::from(s.as_str())));
                Ok(())
            }
            Node::Expr(e) => {
                out.push(self.eval(e, env)?);
                Ok(())
            }
            Node::If { cond, then_branch, else_branch } => {
                let branch = if self.eval(cond, env)?.truthy() { then_branch } else { else_branch };
                for n in branch {
                    self.render_node(n, env, out)?;
                }
                Ok(())
            }
            Node::For { var, iter, body } => {
                let it = self.eval(iter, env)?;
                let Value::List(items) = it else { return Err(self.rt("for-loop in markup requires a list", iter.span())) };
                let items = items.borrow().clone();
                for item in items {
                    let scope = env.child();
                    scope.declare(var, item);
                    for n in body {
                        self.render_node(n, &scope, out)?;
                    }
                }
                Ok(())
            }
            Node::Element { tag, attrs, children, span } => {
                if let Some(def) = self.components.get(tag).cloned() {
                    // Composition: this tag names a defined component —
                    // instantiate it with attrs-as-props (matched by
                    // declared prop order) and splice its own render output
                    // in place of this node, matching real UI composition.
                    let mut prop_values = Vec::with_capacity(attrs.len());
                    for (_, expr) in attrs {
                        prop_values.push(self.eval(expr, env)?);
                    }
                    let child_instance = self.mount(&def, prop_values, *span)?;
                    let rendered = self.render_instance(&child_instance, *span)?;
                    out.push(rendered);
                    return Ok(());
                }
                // Host element (div, button, span, ...).
                let mut attr_vals = Vec::with_capacity(attrs.len());
                for (name, expr) in attrs {
                    attr_vals.push((name.clone(), self.eval(expr, env)?));
                }
                let mut child_vals = Vec::new();
                for c in children {
                    self.render_node(c, env, &mut child_vals)?;
                }
                out.push(Value::Element(Rc::new(ElementVal { tag: tag.clone(), attrs: attr_vals, children: child_vals })));
                Ok(())
            }
            // See `ast::Node::Match`'s doc comment — no real pattern
            // matching underneath; renders the first arm whose pattern
            // name is `_` if any, else the first arm outright.
            Node::Let { name, value } => {
                let v = self.eval(value, env)?;
                env.declare(name, v);
                Ok(())
            }
            Node::Match { subject, arms } => {
                self.eval(subject, env)?;
                let arm = arms.iter().find(|(name, ..)| name == "_").or_else(|| arms.first());
                if let Some((_, _, nodes)) = arm {
                    for n in nodes {
                        self.render_node(n, env, out)?;
                    }
                }
                Ok(())
            }
        }
    }

    // ── statements ───────────────────────────────────────────────────────────

    fn exec_block(&mut self, stmts: &[Stmt], env: &Env) -> Result<Value, Flow> {
        let mut last = Value::Unit;
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
            // See `ast::Stmt::AssignTarget`'s doc comment: the write itself
            // is a no-op (this bootstrap's `Env` has no object/state field
            // mutation model), but the value expression is still evaluated
            // for any side effects/errors in it.
            Stmt::AssignTarget { value, .. } => self.eval(value, env),
            Stmt::If { branches, orelse, .. } => {
                for (cond, body) in branches {
                    if self.eval(cond, env)?.truthy() {
                        return self.exec_block(body, &env.child());
                    }
                }
                self.exec_block(orelse, &env.child())
            }
            Stmt::Return { value, .. } => {
                let v = match value {
                    Some(e) => self.eval(e, env)?,
                    None => Value::Unit,
                };
                Err(Flow::Return(v))
            }
        }
    }

    // ── expressions ──────────────────────────────────────────────────────────

    pub fn eval(&mut self, expr: &Expr, env: &Env) -> EResult {
        match expr {
            Expr::Int { v, .. } => Ok(Value::Int(*v)),
            Expr::Float { v, .. } => Ok(Value::Float(*v)),
            Expr::Str { v, .. } => Ok(Value::Str(Rc::from(v.as_str()))),
            Expr::Bool { v, .. } => Ok(Value::Bool(*v)),
            Expr::Ident { name, span } => env.get(name).ok_or_else(|| self.rt(format!("undefined name '{name}'"), *span)),
            Expr::List { elems, .. } => {
                let mut v = Vec::with_capacity(elems.len());
                for e in elems {
                    v.push(self.eval(e, env)?);
                }
                Ok(Value::List(Rc::new(RefCell::new(v))))
            }
            Expr::Markup(node, _) => {
                let mut out = Vec::new();
                self.render_node(node, env, &mut out)?;
                Ok(out.into_iter().next().unwrap_or(Value::Unit))
            }
            Expr::IfExpr { cond, then_, else_, .. } => {
                if self.eval(cond, env)?.truthy() {
                    self.eval(then_, env)
                } else {
                    self.eval(else_, env)
                }
            }
            // See `ast::Expr::MatchExpr`'s doc comment — no real pattern
            // matching underneath.
            Expr::MatchExpr { subject, arms, .. } => {
                self.eval(subject, env)?;
                let arm = arms.iter().find(|(name, ..)| name == "_").or_else(|| arms.first());
                match arm {
                    Some((_, _, body)) => self.eval(body, env),
                    None => Ok(Value::Unit),
                }
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
            Expr::Lambda { params, body, .. } => Ok(Value::Closure { params: Rc::new(params.clone()), body: Rc::new(body.clone()), env: env.clone() }),
            Expr::Call { func, args, span } => {
                let arg_vals = args.iter().map(|a| self.eval(a, env)).collect::<Result<Vec<_>, _>>()?;
                // Method call sugar: `instance.method(args)`.
                if let Expr::Attr { obj, name, span: aspan } = func.as_ref() {
                    let recv = self.eval(obj, env)?;
                    return self.call_method(&recv, name, arg_vals, *aspan);
                }
                let callee = self.eval(func, env)?;
                self.call_value(&callee, arg_vals, *span)
            }
            Expr::Attr { obj, name, span } => {
                let ov = self.eval(obj, env)?;
                match &ov {
                    Value::ComponentInstance(inst) => inst.env.get(name).ok_or_else(|| self.rt(format!("instance of '{}' has no field '{}'", inst.def.name, name), *span)),
                    other => Err(self.rt(format!("'{}' has no attribute '{}'", other.type_name(), name), *span)),
                }
            }
        }
    }

    fn call_method(&mut self, recv: &Value, name: &str, args: Vec<Value>, span: Span) -> EResult {
        let Value::ComponentInstance(inst) = recv else {
            return Err(self.rt(format!("cannot call method '{name}' on a {}", recv.type_name()), span));
        };
        let Some(m) = inst.def.methods.iter().find(|m| m.name == name) else {
            return Err(self.rt(format!("component '{}' has no method '{}'", inst.def.name, name), span));
        };
        let scope = inst.env.child();
        for (p, a) in m.params.iter().zip(args.iter()) {
            scope.declare(p, a.clone());
        }
        match self.exec_block(&m.body, &scope) {
            Ok(v) => Ok(v),
            Err(Flow::Return(v)) => Ok(v),
            Err(other) => Err(other),
        }
    }

    pub fn call_value(&mut self, callee: &Value, args: Vec<Value>, span: Span) -> EResult {
        match callee {
            Value::Closure { params, body, env } => {
                let scope = env.child();
                for (p, a) in params.iter().zip(args.iter()) {
                    scope.declare(p, a.clone());
                }
                match self.exec_block(body, &scope) {
                    Ok(v) => Ok(v),
                    Err(Flow::Return(v)) => Ok(v),
                    Err(other) => Err(other),
                }
            }
            Value::ComponentClass(def) => {
                let inst = self.mount(def, args, span)?;
                Ok(Value::ComponentInstance(inst))
            }
            Value::Builtin(name) => crate::builtins::call_builtin(self, name, args, span),
            other => Err(self.rt(format!("value of type '{}' is not callable", other.type_name()), span)),
        }
    }

    pub fn render_public(&mut self, instance: &Rc<ComponentInstance>, span: Span) -> EResult {
        self.render_instance(instance, span)
    }

    fn binop(&mut self, op: &str, l: Value, r: Value, span: Span) -> EResult {
        use Value::*;
        match (op, &l, &r) {
            ("+", Str(a), Str(b)) => return Ok(Str(Rc::from(format!("{a}{b}").as_str()))),
            ("==", _, _) => return Ok(Bool(l.eq_val(&r))),
            ("!=", _, _) => return Ok(Bool(!l.eq_val(&r))),
            _ => {}
        }
        if matches!(op, "<" | "<=" | ">" | ">=") {
            let (Some(a), Some(b)) = (l.as_f64(), r.as_f64()) else {
                return Err(self.rt(format!("cannot compare '{}' and '{}'", l.type_name(), r.type_name()), span));
            };
            let ord = a.partial_cmp(&b);
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

