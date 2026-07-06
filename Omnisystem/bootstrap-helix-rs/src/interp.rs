//! Helix tree-walking interpreter.
//!
//! Two restrictions are **genuinely enforced**, not just implied by the
//! domain — this is what makes Helix authentic rather than "Titan plus
//! vector types":
//!   - **No recursion.** A call stack of currently-executing fn/kernel/
//!     shader names is tracked; calling a name already on the stack is a
//!     real, reportable error. Real shader compilers reject recursion
//!     because GPU hardware/SPIR-V has no call stack for it.
//!   - **No dynamic loop bounds.** `for i in 0..N` requires `N` to be a
//!     literal integer in the source, checked by inspecting the AST node
//!     itself (not by evaluating it and hoping it's constant) — matches
//!     real shader compilation, which needs to unroll or bound loops
//!     statically for fixed-function/parallel hardware.
//!
//! `dispatch(kernel, args..., n)` simulates a compute dispatch by running
//! the kernel body once per thread id in `0..n`, **sequentially** — there's
//! no real GPU here, so this is honestly a single-threaded reference
//! execution (exactly what a shader validator / CPU fallback path does),
//! not a claim of actual parallel hardware execution.

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
    fns: HashMap<String, FnDef>,
    kernels: HashMap<String, KernelDef>,
    shaders: HashMap<String, ShaderDef>,
    call_stack: Vec<String>,
    pub out: String,
}

impl Interp {
    pub fn new(file: &str) -> Self {
        Interp { file: file.to_string(), globals: Scope::new(), fns: HashMap::new(), kernels: HashMap::new(), shaders: HashMap::new(), call_stack: Vec::new(), out: String::new() }
    }

    fn rt(&self, msg: impl Into<String>, span: Span) -> Flow {
        Flow::Error(Box::new(OmniError::new(Phase::Runtime, msg, span, &self.file)))
    }

    pub fn print(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }

    pub fn run_module(&mut self, module: &Module) -> Result<i32, Box<OmniError>> {
        for f in &module.fns {
            self.fns.insert(f.name.clone(), f.clone());
        }
        for k in &module.kernels {
            self.kernels.insert(k.name.clone(), k.clone());
        }
        for s in &module.shaders {
            self.shaders.insert(s.name.clone(), s.clone());
        }
        let env = self.globals.clone();
        for stmt in &module.script {
            match self.exec_stmt(stmt, &env) {
                Ok(_) => {}
                Err(Flow::Error(e)) => return Err(e),
                Err(Flow::Return(_)) => {}
            }
        }
        Ok(0)
    }

    // ── dispatch & shader-stage execution ─────────────────────────────────────

    fn dispatch(&mut self, kernel_name: &str, mut extra_args: Vec<Value>, span: Span) -> EResult {
        let Some(n_val) = extra_args.pop() else {
            return Err(self.rt("dispatch(kernel, args..., n) requires a thread count", span));
        };
        let n = n_val.as_f64().ok_or_else(|| self.rt("dispatch's last argument (thread count) must be a number", span))? as i64;
        let Some(kdef) = self.kernels.get(kernel_name).cloned() else {
            return Err(self.rt(format!("no kernel named '{kernel_name}'"), span));
        };
        if kdef.params.is_empty() {
            return Err(self.rt(format!("kernel '{kernel_name}' must declare at least a thread-id parameter"), span));
        }
        for id in 0..n {
            let scope = self.globals.child();
            scope.declare(&kdef.params[0], Value::Int(id));
            for (p, a) in kdef.params[1..].iter().zip(extra_args.iter()) {
                scope.declare(p, a.clone());
            }
            self.call_stack.push(kernel_name.to_string());
            let result = self.exec_block(&kdef.body, &scope);
            self.call_stack.pop();
            match result {
                Ok(_) | Err(Flow::Return(_)) => {}
                Err(other) => return Err(other),
            }
        }
        Ok(Value::Unit)
    }

    fn run_stage(&mut self, shader_name: &str, mut args: Vec<Value>, span: Span) -> EResult {
        let Some(sdef) = self.shaders.get(shader_name).cloned() else {
            return Err(self.rt(format!("no shader named '{shader_name}'"), span));
        };
        let Some(param) = sdef.params.first() else {
            return Err(self.rt(format!("shader '{shader_name}' must declare at least one input parameter"), span));
        };
        if args.len() != 1 {
            return Err(self.rt("run_stage(shader, buffer(...)) expects exactly one buffer argument holding the input stream", span));
        }
        let Value::Buffer(inputs) = args.remove(0) else {
            return Err(self.rt("run_stage's second argument must be a buffer (use buffer(v1, v2, ...) to build one)", span));
        };
        let inputs = inputs.borrow().clone();
        let mut outputs = Vec::with_capacity(inputs.len());
        for input in inputs {
            let scope = self.globals.child();
            scope.declare(param, input);
            self.call_stack.push(shader_name.to_string());
            let result = self.exec_block(&sdef.body, &scope);
            self.call_stack.pop();
            match result {
                Ok(v) | Err(Flow::Return(v)) => outputs.push(v),
                Err(other) => return Err(other),
            }
        }
        Ok(Value::Buffer(Rc::new(RefCell::new(outputs))))
    }

    fn call_user_fn(&mut self, name: &str, args: Vec<Value>, span: Span) -> EResult {
        if self.call_stack.iter().any(|f| f == name) {
            return Err(self.rt(
                format!("recursion is not allowed in Helix: '{name}' is already executing (call chain: {})", self.call_stack.join(" -> ")),
                span,
            ));
        }
        let Some(fdef) = self.fns.get(name).cloned() else {
            return Err(self.rt(format!("unknown function '{name}'"), span));
        };
        let scope = self.globals.child();
        for (p, a) in fdef.params.iter().zip(args.iter()) {
            scope.declare(p, a.clone());
        }
        self.call_stack.push(name.to_string());
        let result = self.exec_block(&fdef.body, &scope);
        self.call_stack.pop();
        match result {
            Ok(v) => Ok(v),
            Err(Flow::Return(v)) => Ok(v),
            Err(other) => Err(other),
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
            Stmt::Let { name, value, .. } => {
                let v = self.eval(value, env)?;
                env.declare(name, v.clone());
                Ok(v)
            }
            Stmt::Assign { target, value, span } => {
                let v = self.eval(value, env)?;
                self.assign_target(target, v.clone(), env, *span)?;
                Ok(v)
            }
            Stmt::If { branches, orelse, .. } => {
                for (cond, body) in branches {
                    if self.eval(cond, env)?.truthy() {
                        return self.exec_block(body, &env.child());
                    }
                }
                self.exec_block(orelse, &env.child())
            }
            Stmt::For { var, hi, body, span } => {
                // Real, enforced restriction: the bound must be a literal
                // integer in the source, not merely a value that happens to
                // be constant at runtime — matches real shader-compiler
                // requirements for statically boundable loops.
                let Expr::Int { v: n, .. } = hi else {
                    return Err(self.rt("loop bound must be a compile-time integer literal (Helix disallows dynamic loop bounds, matching real shader-hardware constraints)", *span));
                };
                for i in 0..*n {
                    let scope = env.child();
                    scope.declare(var, Value::Int(i));
                    self.exec_block(body, &scope)?;
                }
                Ok(Value::Unit)
            }
            Stmt::Return { value, .. } => {
                let v = match value {
                    Some(e) => self.eval(e, env)?,
                    None => Value::Unit,
                };
                Err(Flow::Return(v))
            }
            Stmt::Var { name, value, .. } => {
                let v = match value {
                    Some(e) => self.eval(e, env)?,
                    None => Value::Unit,
                };
                env.declare(name, v.clone());
                Ok(v)
            }
            Stmt::While { cond, body, .. } => {
                while self.eval(cond, env)?.truthy() {
                    self.exec_block(body, &env.child())?;
                }
                Ok(Value::Unit)
            }
            Stmt::CFor { init, cond, step, body, .. } => {
                let scope = env.child();
                self.exec_stmt(init, &scope)?;
                while self.eval(cond, &scope)?.truthy() {
                    self.exec_block(body, &scope.child())?;
                    self.exec_stmt(step, &scope)?;
                }
                Ok(Value::Unit)
            }
        }
    }

    fn assign_target(&mut self, target: &Expr, value: Value, env: &Env, span: Span) -> Result<(), Flow> {
        match target {
            Expr::Ident { name, .. } => {
                env.assign_existing_or_local(name, value);
                Ok(())
            }
            Expr::Index { obj, index, .. } => {
                let ov = self.eval(obj, env)?;
                let iv = self.eval(index, env)?;
                let i = iv.as_f64().ok_or_else(|| self.rt("index must be numeric", span))? as usize;
                match ov {
                    Value::Buffer(b) => {
                        let mut b = b.borrow_mut();
                        if i >= b.len() {
                            return Err(self.rt(format!("buffer index {i} out of range (len {})", b.len()), span));
                        }
                        b[i] = value;
                        Ok(())
                    }
                    Value::Vec(v) => {
                        let f = value.as_f64().ok_or_else(|| self.rt("vector component must be numeric", span))?;
                        let mut v = v.borrow_mut();
                        if i >= v.len() {
                            return Err(self.rt(format!("vector component index {i} out of range"), span));
                        }
                        v[i] = f;
                        Ok(())
                    }
                    other => Err(self.rt(format!("'{}' is not indexable", other.type_name()), span)),
                }
            }
            Expr::Attr { obj, name, .. } => {
                // Swizzle assignment: `v.xy = other_vec2`.
                let ov = self.eval(obj, env)?;
                let Value::Vec(v) = &ov else {
                    return Err(self.rt(format!("'{}' has no field '{}'", ov.type_name(), name), span));
                };
                let idxs = swizzle_indices(name, v.borrow().len(), span, &self.file)?;
                match &value {
                    Value::Vec(src) => {
                        let src = src.borrow();
                        if src.len() != idxs.len() {
                            return Err(self.rt(format!("swizzle assignment length mismatch: '.{name}' has {} components, value has {}", idxs.len(), src.len()), span));
                        }
                        let mut v = v.borrow_mut();
                        for (i, &comp_i) in idxs.iter().enumerate() {
                            v[comp_i] = src[i];
                        }
                        Ok(())
                    }
                    scalar if idxs.len() == 1 => {
                        let f = scalar.as_f64().ok_or_else(|| self.rt("swizzle assignment value must be numeric", span))?;
                        v.borrow_mut()[idxs[0]] = f;
                        Ok(())
                    }
                    _ => Err(self.rt(format!("cannot assign a scalar to multi-component swizzle '.{name}'"), span)),
                }
            }
            _ => Err(self.rt("invalid assignment target", span)),
        }
    }

    // ── expressions ──────────────────────────────────────────────────────────

    pub fn eval(&mut self, expr: &Expr, env: &Env) -> EResult {
        match expr {
            Expr::Int { v, .. } => Ok(Value::Int(*v)),
            Expr::Float { v, .. } => Ok(Value::Float(*v)),
            Expr::Bool { v, .. } => Ok(Value::Bool(*v)),
            Expr::Ident { name, span } => env.get(name).ok_or_else(|| self.rt(format!("undefined name '{name}'"), *span)),
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
                    ("-", Value::Vec(vec)) => Ok(Value::Vec(Rc::new(RefCell::new(vec.borrow().iter().map(|x| -x).collect())))),
                    ("not", _) => Ok(Value::Bool(!v.truthy())),
                    _ => Err(self.rt(format!("bad operand for unary '{op}'"), *span)),
                }
            }
            Expr::Attr { obj, name, span } => {
                let ov = self.eval(obj, env)?;
                match &ov {
                    Value::Vec(v) => {
                        let idxs = swizzle_indices(name, v.borrow().len(), *span, &self.file)?;
                        let vb = v.borrow();
                        if idxs.len() == 1 {
                            Ok(Value::Float(vb[idxs[0]]))
                        } else {
                            Ok(Value::Vec(Rc::new(RefCell::new(idxs.iter().map(|&i| vb[i]).collect()))))
                        }
                    }
                    other => Err(self.rt(format!("'{}' has no field '{}'", other.type_name(), name), *span)),
                }
            }
            Expr::Index { obj, index, span } => {
                let ov = self.eval(obj, env)?;
                let iv = self.eval(index, env)?;
                let i = iv.as_f64().ok_or_else(|| self.rt("index must be numeric", *span))? as usize;
                match ov {
                    Value::Buffer(b) => b.borrow().get(i).cloned().ok_or_else(|| self.rt(format!("buffer index {i} out of range"), *span)),
                    Value::Vec(v) => v.borrow().get(i).copied().map(Value::Float).ok_or_else(|| self.rt(format!("vector component index {i} out of range"), *span)),
                    other => Err(self.rt(format!("'{}' is not indexable", other.type_name()), *span)),
                }
            }
            Expr::Call { func, args, span } => {
                // `dispatch`/`run_stage`'s first argument is a *bare name*
                // reference to a kernel/shader (there are no first-class
                // function values in Helix — entry points are named, like
                // real shader stages), not a variable to evaluate.
                if (func == "dispatch" || func == "run_stage") && !args.is_empty() {
                    let Expr::Ident { name: target, .. } = &args[0] else {
                        return Err(self.rt(format!("{func}'s first argument must be a kernel/shader name"), *span));
                    };
                    let rest = args[1..].iter().map(|a| self.eval(a, env)).collect::<Result<Vec<_>, _>>()?;
                    return if func == "dispatch" { self.dispatch(target, rest, *span) } else { self.run_stage(target, rest, *span) };
                }
                let arg_vals = args.iter().map(|a| self.eval(a, env)).collect::<Result<Vec<_>, _>>()?;
                self.call(func, arg_vals, *span)
            }
            Expr::List { elems, .. } => {
                let vals = elems.iter().map(|e| self.eval(e, env)).collect::<Result<Vec<_>, _>>()?;
                Ok(Value::Buffer(Rc::new(RefCell::new(vals))))
            }
            Expr::MethodCall { obj, args, span, .. } => {
                self.eval(obj, env)?;
                for a in args {
                    self.eval(a, env)?;
                }
                Err(self.rt("method-call syntax (Rust-dialect GPU-binding glue code) is parsed but not evaluable in this bootstrap", *span))
            }
            Expr::Lambda { span, .. } => Err(self.rt("closures (Rust-dialect glue code) are parsed but not evaluable in this bootstrap", *span)),
            Expr::IfExpr { cond, then_, orelse, .. } => {
                if self.eval(cond, env)?.truthy() { self.eval(then_, env) } else { self.eval(orelse, env) }
            }
        }
    }

    fn call(&mut self, name: &str, args: Vec<Value>, span: Span) -> EResult {
        match name {
            _ if self.kernels.contains_key(name) => Err(self.rt(format!("'{name}' is a kernel — call it via dispatch({name}, ...), not directly"), span)),
            _ if self.shaders.contains_key(name) => Err(self.rt(format!("'{name}' is a shader — call it via run_stage({name}, ...), not directly"), span)),
            _ if self.fns.contains_key(name) => self.call_user_fn(name, args, span),
            _ => crate::builtins::call_builtin(self, name, args, span),
        }
    }

    fn binop(&mut self, op: &str, l: Value, r: Value, span: Span) -> EResult {
        use Value::*;
        if let (Vec(a), Vec(b)) = (&l, &r) {
            let (a, b) = (a.borrow(), b.borrow());
            if a.len() != b.len() {
                return Err(self.rt(format!("vector length mismatch: vec{} vs vec{}", a.len(), b.len()), span));
            }
            let out: Option<std::vec::Vec<f64>> = a
                .iter()
                .zip(b.iter())
                .map(|(&x, &y)| match op {
                    "+" => Some(x + y),
                    "-" => Some(x - y),
                    "*" => Some(x * y),
                    "/" => Some(x / y),
                    _ => None,
                })
                .collect();
            return match out {
                Some(v) => Ok(Vec(Rc::new(RefCell::new(v)))),
                None => Err(self.rt(format!("unsupported vector operator '{op}'"), span)),
            };
        }
        if let (Vec(a), other) | (other, Vec(a)) = (&l, &r) {
            if let Some(scalar) = other.as_f64() {
                if matches!(op, "*" | "/") {
                    let out: std::vec::Vec<f64> = a.borrow().iter().map(|&x| if op == "*" { x * scalar } else { x / scalar }).collect();
                    return Ok(Vec(Rc::new(RefCell::new(out))));
                }
            }
        }
        match op {
            "==" => return Ok(Bool(l.eq_val(&r))),
            "!=" => return Ok(Bool(!l.eq_val(&r))),
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

/// Maps a swizzle string (`"xyz"`, `"rgba"`, `"x"`, ...) to component
/// indices. Only the position set (`xyzw`) and color set (`rgba`) are
/// valid, and they cannot be mixed — matches real shader-language swizzle
/// rules exactly.
fn swizzle_indices(name: &str, vec_len: usize, span: Span, file: &str) -> Result<Vec<usize>, Flow> {
    const POS: &str = "xyzw";
    const COL: &str = "rgba";
    let set = if name.chars().all(|c| POS.contains(c)) {
        POS
    } else if name.chars().all(|c| COL.contains(c)) {
        COL
    } else {
        return Err(Flow::Error(Box::new(OmniError::new(
            Phase::Runtime,
            format!("'{name}' is not a valid swizzle — use only 'xyzw' or only 'rgba', not a mix"),
            span,
            file,
        ))));
    };
    let mut idxs = Vec::with_capacity(name.len());
    for c in name.chars() {
        let i = set.find(c).unwrap();
        if i >= vec_len {
            return Err(Flow::Error(Box::new(OmniError::new(Phase::Runtime, format!("swizzle component '{c}' is out of range for a vec{vec_len}"), span, file))));
        }
        idxs.push(i);
    }
    Ok(idxs)
}
