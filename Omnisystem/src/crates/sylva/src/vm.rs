//! Sylva VM — tree-walk interpreter for Sylva expressions.
//!
//! Values are dynamically typed with gradual type checking.
//! The VM is single-threaded; async operations dispatch to registered callbacks.

use crate::ast::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

// ── Values ────────────────────────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SylvaValue {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<SylvaValue>),
    Map(Vec<(SylvaValue, SylvaValue)>),
    Tuple(Vec<SylvaValue>),
    // Closures are not serializable — use opaque JSON placeholder
    #[serde(skip)]
    Closure(ClosureVal),
    // Native function: a Rust closure
    #[serde(skip)]
    Native(Arc<dyn Fn(Vec<SylvaValue>) -> VmResult<SylvaValue> + Send + Sync>),
    // Actor reference (opaque ID)
    ActorRef(String),
}

impl PartialEq for SylvaValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Str(a), Self::Str(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Tuple(a), Self::Tuple(b)) => a == b,
            _ => false,
        }
    }
}

impl std::fmt::Debug for SylvaValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nil => write!(f, "Nil"),
            Self::Bool(b) => write!(f, "Bool({b})"),
            Self::Int(n) => write!(f, "Int({n})"),
            Self::Float(v) => write!(f, "Float({v})"),
            Self::Str(s) => write!(f, "Str({s:?})"),
            Self::List(l) => write!(f, "List({l:?})"),
            Self::Map(m) => write!(f, "Map({m:?})"),
            Self::Tuple(t) => write!(f, "Tuple({t:?})"),
            Self::Closure(_) => write!(f, "<closure>"),
            Self::Native(_) => write!(f, "<native>"),
            Self::ActorRef(id) => write!(f, "ActorRef({id:?})"),
        }
    }
}

impl std::fmt::Display for SylvaValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nil => write!(f, "nil"),
            Self::Bool(b) => write!(f, "{}", b),
            Self::Int(n) => write!(f, "{}", n),
            Self::Float(v) => write!(f, "{}", v),
            Self::Str(s) => write!(f, "{}", s),
            Self::List(l) => write!(
                f,
                "[{}]",
                l.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Tuple(t) => write!(
                f,
                "({})",
                t.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Map(m) => write!(
                f,
                "{{{}}}",
                m.iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Closure(_) => write!(f, "<fn>"),
            Self::Native(_) => write!(f, "<native>"),
            Self::ActorRef(id) => write!(f, "<actor:{id}>"),
        }
    }
}

impl SylvaValue {
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Nil | Self::Bool(false) => false,
            Self::Int(0) => false,
            Self::Str(s) if s.is_empty() => false,
            _ => true,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Nil => serde_json::Value::Null,
            Self::Bool(b) => (*b).into(),
            Self::Int(n) => (*n).into(),
            Self::Float(f) => serde_json::json!(*f),
            Self::Str(s) => s.clone().into(),
            Self::List(l) => serde_json::Value::Array(l.iter().map(|v| v.to_json()).collect()),
            Self::Tuple(t) => serde_json::Value::Array(t.iter().map(|v| v.to_json()).collect()),
            Self::Map(m) => {
                let obj: serde_json::Map<String, serde_json::Value> = m
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_json()))
                    .collect();
                serde_json::Value::Object(obj)
            }
            Self::Closure(_) | Self::Native(_) => serde_json::json!("<fn>"),
            Self::ActorRef(id) => serde_json::json!({"actor": id}),
        }
    }

    pub fn from_json(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Self::Nil,
            serde_json::Value::Bool(b) => Self::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Self::Int(i)
                } else {
                    Self::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            serde_json::Value::String(s) => Self::Str(s),
            serde_json::Value::Array(a) => Self::List(a.into_iter().map(Self::from_json).collect()),
            serde_json::Value::Object(o) => {
                let pairs = o
                    .into_iter()
                    .map(|(k, v)| (Self::Str(k), Self::from_json(v)))
                    .collect();
                Self::Map(pairs)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClosureVal {
    pub params: Vec<Param>,
    pub body: Expr,
    pub env: Env,
    pub is_async: bool,
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum VmError {
    #[error("undefined variable: {0}")]
    Undefined(String),
    #[error("type error: {0}")]
    TypeError(String),
    #[error("arity mismatch: expected {expected}, got {got}")]
    ArityMismatch { expected: usize, got: usize },
    #[error("index out of bounds: {0}")]
    IndexOutOfBounds(usize),
    #[error("key not found: {0}")]
    KeyNotFound(String),
    #[error("runtime error: {0}")]
    Runtime(String),
    #[error("return")]
    Return(SylvaValue),
    #[error("break")]
    Break(Option<SylvaValue>),
    #[error("continue")]
    Continue,
    #[error("tool call failed: {0}")]
    ToolCallFailed(String),
}

pub type VmResult<T> = Result<T, VmError>;

// ── Environment ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Env {
    frames: Vec<HashMap<String, SylvaValue>>,
}

impl Env {
    pub fn new() -> Self {
        Self {
            frames: vec![HashMap::new()],
        }
    }

    pub fn push_frame(&mut self) {
        self.frames.push(HashMap::new());
    }
    pub fn pop_frame(&mut self) {
        self.frames.pop();
    }

    pub fn get(&self, name: &str) -> Option<&SylvaValue> {
        for frame in self.frames.iter().rev() {
            if let Some(v) = frame.get(name) {
                return Some(v);
            }
        }
        None
    }

    pub fn set(&mut self, name: String, val: SylvaValue) {
        if let Some(frame) = self.frames.last_mut() {
            frame.insert(name, val);
        }
    }

    pub fn set_global(&mut self, name: String, val: SylvaValue) {
        if let Some(frame) = self.frames.first_mut() {
            frame.insert(name, val);
        }
    }

    pub fn assign(&mut self, name: &str, val: SylvaValue) -> bool {
        for frame in self.frames.iter_mut().rev() {
            if frame.contains_key(name) {
                frame.insert(name.into(), val);
                return true;
            }
        }
        false
    }
}

// ── VM ────────────────────────────────────────────────────────────────────────

pub type ToolFn =
    Arc<dyn Fn(String, serde_json::Value) -> VmResult<serde_json::Value> + Send + Sync>;

pub struct SylvaVm {
    pub env: Env,
    /// Registered tool callback — maps to the UCR at runtime.
    pub tool_fn: Option<ToolFn>,
}

impl SylvaVm {
    pub fn new() -> Self {
        let mut vm = Self {
            env: Env::new(),
            tool_fn: None,
        };
        crate::stdlib::register_stdlib(&mut vm);
        vm
    }

    pub fn with_tool_fn(tool_fn: ToolFn) -> Self {
        let mut vm = Self::new();
        vm.tool_fn = Some(tool_fn);
        vm
    }

    pub fn set_global(&mut self, name: impl Into<String>, val: SylvaValue) {
        self.env.set_global(name.into(), val);
    }

    /// Eval a source string and return the last value.
    pub fn eval_str(&mut self, src: &str) -> VmResult<SylvaValue> {
        let module =
            crate::parser::parse_module(src, "repl").map_err(|e| VmError::Runtime(e.message))?;
        let mut last = SylvaValue::Nil;
        for item in &module.items {
            match item {
                Item::FnDef(fndef) => {
                    let name = fndef.name.clone().unwrap_or_else(|| "_".into());
                    let closure = SylvaValue::Closure(ClosureVal {
                        params: fndef.params.clone(),
                        body: *fndef.body.clone(),
                        env: self.env.clone(),
                        is_async: fndef.is_async,
                    });
                    self.env.set(name, closure);
                    last = SylvaValue::Nil;
                }
                Item::LetDef { name, value, .. } => {
                    let val = self.eval_expr(value)?;
                    self.env.set(name.clone(), val.clone());
                    last = val;
                }
                _ => {}
            }
        }
        Ok(last)
    }

    /// Eval an `Expr` AST node.
    pub fn eval_expr(&mut self, expr: &Expr) -> VmResult<SylvaValue> {
        match &expr.kind {
            ExprKind::Nil => Ok(SylvaValue::Nil),
            ExprKind::Bool(b) => Ok(SylvaValue::Bool(*b)),
            ExprKind::Int(n) => Ok(SylvaValue::Int(*n)),
            ExprKind::Float(f) => Ok(SylvaValue::Float(*f)),
            ExprKind::Str(s) => Ok(SylvaValue::Str(s.clone())),
            ExprKind::Tuple(es) => {
                let vals: VmResult<Vec<_>> = es.iter().map(|e| self.eval_expr(e)).collect();
                Ok(SylvaValue::Tuple(vals?))
            }
            ExprKind::List(es) => {
                let vals: VmResult<Vec<_>> = es.iter().map(|e| self.eval_expr(e)).collect();
                Ok(SylvaValue::List(vals?))
            }
            ExprKind::Map(pairs) => {
                let mut out = Vec::new();
                for (k, v) in pairs {
                    out.push((self.eval_expr(k)?, self.eval_expr(v)?));
                }
                Ok(SylvaValue::Map(out))
            }
            ExprKind::Var(name) => self
                .env
                .get(name)
                .cloned()
                .ok_or_else(|| VmError::Undefined(name.clone())),
            ExprKind::Field(obj, field) => {
                let obj_val = self.eval_expr(obj)?;
                match &obj_val {
                    SylvaValue::Map(pairs) => {
                        let key = SylvaValue::Str(field.clone());
                        pairs
                            .iter()
                            .find(|(k, _)| k == &key)
                            .map(|(_, v)| v.clone())
                            .ok_or_else(|| VmError::KeyNotFound(field.clone()))
                    }
                    other => Err(VmError::TypeError(format!(
                        "cannot access field {field} on {other}"
                    ))),
                }
            }
            ExprKind::Index(obj, idx) => {
                let obj_val = self.eval_expr(obj)?;
                let idx_val = self.eval_expr(idx)?;
                match (obj_val, idx_val) {
                    (SylvaValue::List(l), SylvaValue::Int(i)) => {
                        let ui = if i < 0 { l.len() as i64 + i } else { i } as usize;
                        l.get(ui).cloned().ok_or(VmError::IndexOutOfBounds(ui))
                    }
                    (SylvaValue::Map(m), key) => m
                        .iter()
                        .find(|(k, _)| k == &key)
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| VmError::KeyNotFound(key.to_string())),
                    (SylvaValue::Str(s), SylvaValue::Int(i)) => {
                        let ui = if i < 0 { s.len() as i64 + i } else { i } as usize;
                        s.chars()
                            .nth(ui)
                            .map(|c| SylvaValue::Str(c.to_string()))
                            .ok_or(VmError::IndexOutOfBounds(ui))
                    }
                    (obj, idx) => Err(VmError::TypeError(format!(
                        "cannot index {} with {}",
                        obj, idx
                    ))),
                }
            }
            ExprKind::BinOp(op, lhs, rhs) => self.eval_binop(op, lhs, rhs),
            ExprKind::UnOp(op, expr) => {
                let val = self.eval_expr(expr)?;
                match (op, val) {
                    (UnOp::Neg, SylvaValue::Int(n)) => Ok(SylvaValue::Int(-n)),
                    (UnOp::Neg, SylvaValue::Float(f)) => Ok(SylvaValue::Float(-f)),
                    (UnOp::Not, v) => Ok(SylvaValue::Bool(!v.is_truthy())),
                    (op, v) => Err(VmError::TypeError(format!(
                        "cannot apply {:?} to {}",
                        op, v
                    ))),
                }
            }
            ExprKind::Let { name, value, .. } => {
                let val = self.eval_expr(value)?;
                self.env.set(name.clone(), val.clone());
                Ok(val)
            }
            ExprKind::Assign(lhs, rhs) => {
                let val = self.eval_expr(rhs)?;
                if let ExprKind::Var(name) = &lhs.kind {
                    if !self.env.assign(name, val.clone()) {
                        self.env.set(name.clone(), val.clone());
                    }
                }
                Ok(val)
            }
            ExprKind::Block(stmts) => {
                self.env.push_frame();
                let mut last = SylvaValue::Nil;
                let result = stmts.iter().try_for_each(|s| {
                    last = self.eval_expr(s)?;
                    Ok(())
                });
                self.env.pop_frame();
                match result {
                    Ok(()) => Ok(last),
                    Err(VmError::Return(v)) => Err(VmError::Return(v)),
                    Err(e) => Err(e),
                }
            }
            ExprKind::If(cond, then, else_) => {
                let c = self.eval_expr(cond)?;
                if c.is_truthy() {
                    self.eval_expr(then)
                } else if let Some(e) = else_ {
                    self.eval_expr(e)
                } else {
                    Ok(SylvaValue::Nil)
                }
            }
            ExprKind::While(cond, body) => {
                loop {
                    let c = self.eval_expr(cond)?;
                    if !c.is_truthy() {
                        break;
                    }
                    match self.eval_expr(body) {
                        Ok(_) => {}
                        Err(VmError::Break(_)) => break,
                        Err(VmError::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(SylvaValue::Nil)
            }
            ExprKind::For(var, iter, body) => {
                let iter_val = self.eval_expr(iter)?;
                match iter_val {
                    SylvaValue::List(items) => {
                        for item in items {
                            self.env.push_frame();
                            self.env.set(var.clone(), item);
                            match self.eval_expr(body) {
                                Ok(_) => {}
                                Err(VmError::Break(_)) => {
                                    self.env.pop_frame();
                                    break;
                                }
                                Err(VmError::Continue) => {
                                    self.env.pop_frame();
                                    continue;
                                }
                                Err(e) => {
                                    self.env.pop_frame();
                                    return Err(e);
                                }
                            }
                            self.env.pop_frame();
                        }
                    }
                    other => {
                        return Err(VmError::TypeError(format!("cannot iterate over {}", other)))
                    }
                }
                Ok(SylvaValue::Nil)
            }
            ExprKind::Return(val) => {
                let v = if let Some(e) = val {
                    self.eval_expr(e)?
                } else {
                    SylvaValue::Nil
                };
                Err(VmError::Return(v))
            }
            ExprKind::Break(val) => {
                let v = if let Some(e) = val {
                    Some(self.eval_expr(e)?)
                } else {
                    None
                };
                Err(VmError::Break(v))
            }
            ExprKind::Continue => Err(VmError::Continue),

            ExprKind::Fn(fndef) => {
                let closure = SylvaValue::Closure(ClosureVal {
                    params: fndef.params.clone(),
                    body: *fndef.body.clone(),
                    env: self.env.clone(),
                    is_async: fndef.is_async,
                });
                if let Some(name) = &fndef.name {
                    self.env.set(name.clone(), closure.clone());
                }
                Ok(closure)
            }

            ExprKind::Call(func, args) => {
                let f = self.eval_expr(func)?;
                let arg_vals: VmResult<Vec<_>> = args.iter().map(|a| self.eval_expr(a)).collect();
                self.call_value(f, arg_vals?)
            }

            ExprKind::MethodCall(obj, method, args) => {
                let obj_val = self.eval_expr(obj)?;
                let arg_vals: VmResult<Vec<_>> = args.iter().map(|a| self.eval_expr(a)).collect();
                let mut all_args = vec![obj_val];
                all_args.extend(arg_vals?);
                let f = self
                    .env
                    .get(method)
                    .cloned()
                    .ok_or_else(|| VmError::Undefined(method.clone()))?;
                self.call_value(f, all_args)
            }

            ExprKind::Match(scrutinee, arms) => {
                let val = self.eval_expr(scrutinee)?;
                for arm in arms {
                    let mut bindings = HashMap::new();
                    if self.match_pattern(&arm.pattern, &val, &mut bindings) {
                        // check guard
                        self.env.push_frame();
                        for (k, v) in bindings {
                            self.env.set(k, v);
                        }
                        let guard_pass = if let Some(g) = &arm.guard {
                            self.eval_expr(g).map(|v| v.is_truthy()).unwrap_or(false)
                        } else {
                            true
                        };
                        if guard_pass {
                            let result = self.eval_expr(&arm.body);
                            self.env.pop_frame();
                            return result;
                        }
                        self.env.pop_frame();
                    }
                }
                Ok(SylvaValue::Nil)
            }

            ExprKind::Pipe(lhs, rhs) => {
                let arg = self.eval_expr(lhs)?;
                let f = self.eval_expr(rhs)?;
                self.call_value(f, vec![arg])
            }

            ExprKind::Spawn(_) | ExprKind::Send(_, _) | ExprKind::Receive | ExprKind::Await(_) => {
                // Async/actor ops not yet supported in sync VM — return a placeholder
                Ok(SylvaValue::Str("<async-op>".into()))
            }

            ExprKind::Struct(name, fields) => {
                let mut map = Vec::new();
                map.push((
                    SylvaValue::Str("__type__".into()),
                    SylvaValue::Str(name.clone()),
                ));
                for (fname, fexpr) in fields {
                    map.push((SylvaValue::Str(fname.clone()), self.eval_expr(fexpr)?));
                }
                Ok(SylvaValue::Map(map))
            }

            ExprKind::TypeAscription(expr, _ty) => self.eval_expr(expr),
        }
    }

    fn call_value(&mut self, f: SylvaValue, args: Vec<SylvaValue>) -> VmResult<SylvaValue> {
        match f {
            SylvaValue::Closure(c) => {
                if args.len() != c.params.len() {
                    return Err(VmError::ArityMismatch {
                        expected: c.params.len(),
                        got: args.len(),
                    });
                }
                let saved = self.env.clone();
                self.env = c.env.clone();
                self.env.push_frame();
                for (param, val) in c.params.iter().zip(args) {
                    self.env.set(param.name.clone(), val);
                }
                let result = match self.eval_expr(&c.body) {
                    Ok(v) => Ok(v),
                    Err(VmError::Return(v)) => Ok(v),
                    Err(e) => Err(e),
                };
                self.env = saved;
                result
            }
            SylvaValue::Native(f) => f(args),
            other => Err(VmError::TypeError(format!("not callable: {}", other))),
        }
    }

    fn eval_binop(&mut self, op: &BinOp, lhs: &Expr, rhs: &Expr) -> VmResult<SylvaValue> {
        let l = self.eval_expr(lhs)?;
        // Short-circuit for And/Or
        match op {
            BinOp::And => {
                return Ok(SylvaValue::Bool(
                    l.is_truthy() && self.eval_expr(rhs)?.is_truthy(),
                ))
            }
            BinOp::Or => {
                return Ok(SylvaValue::Bool(
                    l.is_truthy() || self.eval_expr(rhs)?.is_truthy(),
                ))
            }
            _ => {}
        }
        let r = self.eval_expr(rhs)?;
        match (op, l, r) {
            (BinOp::Add, SylvaValue::Int(a), SylvaValue::Int(b)) => Ok(SylvaValue::Int(a + b)),
            (BinOp::Sub, SylvaValue::Int(a), SylvaValue::Int(b)) => Ok(SylvaValue::Int(a - b)),
            (BinOp::Mul, SylvaValue::Int(a), SylvaValue::Int(b)) => Ok(SylvaValue::Int(a * b)),
            (BinOp::Div, SylvaValue::Int(a), SylvaValue::Int(b)) => {
                if b == 0 {
                    Err(VmError::Runtime("division by zero".into()))
                } else {
                    Ok(SylvaValue::Int(a / b))
                }
            }
            (BinOp::Rem, SylvaValue::Int(a), SylvaValue::Int(b)) => Ok(SylvaValue::Int(a % b)),
            (BinOp::Add, SylvaValue::Float(a), SylvaValue::Float(b)) => {
                Ok(SylvaValue::Float(a + b))
            }
            (BinOp::Sub, SylvaValue::Float(a), SylvaValue::Float(b)) => {
                Ok(SylvaValue::Float(a - b))
            }
            (BinOp::Mul, SylvaValue::Float(a), SylvaValue::Float(b)) => {
                Ok(SylvaValue::Float(a * b))
            }
            (BinOp::Div, SylvaValue::Float(a), SylvaValue::Float(b)) => {
                Ok(SylvaValue::Float(a / b))
            }
            (BinOp::Add, SylvaValue::Int(a), SylvaValue::Float(b)) => {
                Ok(SylvaValue::Float(a as f64 + b))
            }
            (BinOp::Add, SylvaValue::Float(a), SylvaValue::Int(b)) => {
                Ok(SylvaValue::Float(a + b as f64))
            }
            (BinOp::Concat, SylvaValue::Str(a), SylvaValue::Str(b)) => Ok(SylvaValue::Str(a + &b)),
            (BinOp::Concat, SylvaValue::List(mut a), SylvaValue::List(b)) => {
                a.extend(b);
                Ok(SylvaValue::List(a))
            }
            (BinOp::Eq, a, b) => Ok(SylvaValue::Bool(a == b)),
            (BinOp::Ne, a, b) => Ok(SylvaValue::Bool(a != b)),
            (BinOp::Lt, SylvaValue::Int(a), SylvaValue::Int(b)) => Ok(SylvaValue::Bool(a < b)),
            (BinOp::Le, SylvaValue::Int(a), SylvaValue::Int(b)) => Ok(SylvaValue::Bool(a <= b)),
            (BinOp::Gt, SylvaValue::Int(a), SylvaValue::Int(b)) => Ok(SylvaValue::Bool(a > b)),
            (BinOp::Ge, SylvaValue::Int(a), SylvaValue::Int(b)) => Ok(SylvaValue::Bool(a >= b)),
            (BinOp::Lt, SylvaValue::Str(a), SylvaValue::Str(b)) => Ok(SylvaValue::Bool(a < b)),
            (BinOp::Le, SylvaValue::Str(a), SylvaValue::Str(b)) => Ok(SylvaValue::Bool(a <= b)),
            (BinOp::Gt, SylvaValue::Str(a), SylvaValue::Str(b)) => Ok(SylvaValue::Bool(a > b)),
            (BinOp::Ge, SylvaValue::Str(a), SylvaValue::Str(b)) => Ok(SylvaValue::Bool(a >= b)),
            (op, l, r) => Err(VmError::TypeError(format!(
                "cannot apply {:?} to {} and {}",
                op, l, r
            ))),
        }
    }

    fn match_pattern(
        &self,
        pat: &Pattern,
        val: &SylvaValue,
        bindings: &mut HashMap<String, SylvaValue>,
    ) -> bool {
        match (pat, val) {
            (Pattern::Wildcard, _) => true,
            (Pattern::Nil, SylvaValue::Nil) => true,
            (Pattern::Bool(b), SylvaValue::Bool(v)) => b == v,
            (Pattern::Int(n), SylvaValue::Int(v)) => n == v,
            (Pattern::Str(s), SylvaValue::Str(v)) => s == v,
            (Pattern::Bind(name), v) => {
                bindings.insert(name.clone(), v.clone());
                true
            }
            (Pattern::Tuple(pats), SylvaValue::Tuple(vals)) => {
                pats.len() == vals.len()
                    && pats
                        .iter()
                        .zip(vals)
                        .all(|(p, v)| self.match_pattern(p, v, bindings))
            }
            (Pattern::List(head_pats, rest), SylvaValue::List(items)) => {
                if head_pats.len() > items.len() {
                    return false;
                }
                let matched = head_pats
                    .iter()
                    .zip(items)
                    .all(|(p, v)| self.match_pattern(p, v, bindings));
                if matched {
                    if let Some(rest_pat) = rest {
                        let tail: Vec<_> = items[head_pats.len()..].to_vec();
                        self.match_pattern(rest_pat, &SylvaValue::List(tail), bindings)
                    } else {
                        head_pats.len() == items.len()
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

impl Default for SylvaVm {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_arithmetic() {
        let mut vm = SylvaVm::new();
        let v = vm.eval_str("1 + 2 * 3").unwrap();
        assert_eq!(v, SylvaValue::Int(7));
    }

    #[test]
    fn eval_let_and_var() {
        let mut vm = SylvaVm::new();
        let v = vm.eval_str("let x = 42").unwrap();
        assert_eq!(v, SylvaValue::Int(42));
        let v2 = vm.eval_str("x").unwrap();
        assert_eq!(v2, SylvaValue::Int(42));
    }

    #[test]
    fn eval_fn_call() {
        let mut vm = SylvaVm::new();
        vm.eval_str("fn double(x: Int) -> Int { x * 2 }").unwrap();
        let v = vm.eval_str("double(21)").unwrap();
        assert_eq!(v, SylvaValue::Int(42));
    }

    #[test]
    fn eval_if_else() {
        let mut vm = SylvaVm::new();
        let v = vm.eval_str("if 1 > 0 { 100 } else { 0 }").unwrap();
        assert_eq!(v, SylvaValue::Int(100));
    }

    #[test]
    fn eval_list_concat() {
        let mut vm = SylvaVm::new();
        let v = vm.eval_str("[1, 2] ++ [3, 4]").unwrap();
        assert_eq!(
            v,
            SylvaValue::List(vec![
                SylvaValue::Int(1),
                SylvaValue::Int(2),
                SylvaValue::Int(3),
                SylvaValue::Int(4),
            ])
        );
    }

    #[test]
    fn eval_match() {
        let mut vm = SylvaVm::new();
        let v = vm
            .eval_str(r#"match 2 { 1 => "one", 2 => "two", _ => "other" }"#)
            .unwrap();
        assert_eq!(v, SylvaValue::Str("two".into()));
    }

    #[test]
    fn eval_pipeline() {
        let mut vm = SylvaVm::new();
        vm.eval_str("fn inc(x: Int) -> Int { x + 1 }").unwrap();
        let v = vm.eval_str("5 |> inc").unwrap();
        assert_eq!(v, SylvaValue::Int(6));
    }

    #[test]
    fn eval_string_methods_via_native() {
        let mut vm = SylvaVm::new();
        let v = vm.eval_str(r#"len("hello")"#).unwrap();
        assert_eq!(v, SylvaValue::Int(5));
    }
}
