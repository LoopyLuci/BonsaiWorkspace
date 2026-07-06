//! Sylva runtime values — dynamically typed, duck-typed dispatch (no static
//! type checking anywhere, matching Python/JS semantics rather than Titan's
//! static structs+traits model).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::FnDef;

pub type Env = Rc<Scope>;

pub struct Scope {
    pub vars: RefCell<HashMap<String, Value>>,
    pub parent: Option<Env>,
}

impl Scope {
    pub fn new() -> Env {
        Rc::new(Scope { vars: RefCell::new(HashMap::new()), parent: None })
    }
    pub fn child(self: &Env) -> Env {
        Rc::new(Scope { vars: RefCell::new(HashMap::new()), parent: Some(self.clone()) })
    }
    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.vars.borrow().get(name) {
            return Some(v.clone());
        }
        self.parent.as_ref().and_then(|p| p.get(name))
    }
    pub fn set(&self, name: &str, v: Value) {
        self.vars.borrow_mut().insert(name.to_string(), v);
    }
    /// Assigns to the nearest enclosing scope that already declares `name`
    /// (Python assignment-to-existing-binding semantics); falls back to
    /// declaring it in the current scope if not found anywhere (matches
    /// Python's implicit-local-on-first-assignment rule for function
    /// bodies, simplified: this bootstrap doesn't implement `global`/
    /// `nonlocal` scope resolution beyond the `Global` no-op statement).
    pub fn assign_existing_or_local(self: &Env, name: &str, v: Value) {
        let mut scope = Some(self.clone());
        while let Some(s) = scope {
            if s.vars.borrow().contains_key(name) {
                s.vars.borrow_mut().insert(name.to_string(), v);
                return;
            }
            scope = s.parent.clone();
        }
        self.vars.borrow_mut().insert(name.to_string(), v);
    }
}

#[derive(Clone)]
pub struct ClassVal {
    pub name: String,
    pub base: Option<Rc<ClassVal>>,
    pub methods: HashMap<String, Rc<FnDef>>,
    pub class_vars: RefCell<HashMap<String, Value>>,
}

impl ClassVal {
    /// Method resolution order: this class, then walk up `base` chain —
    /// single inheritance, matching the `class Name(Base):` grammar.
    pub fn find_method(&self, name: &str) -> Option<Rc<FnDef>> {
        if let Some(m) = self.methods.get(name) {
            return Some(m.clone());
        }
        self.base.as_ref().and_then(|b| b.find_method(name))
    }
    pub fn is_or_extends(&self, name: &str) -> bool {
        if self.name == name {
            return true;
        }
        self.base.as_ref().is_some_and(|b| b.is_or_extends(name))
    }
}

pub struct InstanceVal {
    pub class: Rc<ClassVal>,
    pub fields: RefCell<HashMap<String, Value>>,
}

#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Bool(bool),
    None,
    List(Rc<RefCell<Vec<Value>>>),
    Tuple(Rc<Vec<Value>>),
    /// Insertion-ordered key/value pairs — linear lookup, matching Python
    /// 3.7+ dict semantics (ordered) without requiring `Value: Hash`.
    Dict(Rc<RefCell<Vec<(Value, Value)>>>),
    Closure { params: Rc<Vec<crate::ast::Param>>, body: Rc<Vec<crate::ast::Stmt>>, env: Env, is_lambda_expr: Option<Rc<crate::ast::Expr>> },
    Function { def: Rc<FnDef>, env: Env },
    BoundMethod { recv: Box<Value>, def: Rc<FnDef>, env: Env },
    Class(Rc<ClassVal>),
    Instance(Rc<InstanceVal>),
    Builtin(&'static str),
    /// Tensor — Sylva's ML-domain value: an n-dimensional array of f64 with
    /// row-major strides. A real, minimal subset of the full stdlib spec
    /// (shape/zeros/ones/get/sum/mean/add/mul/matmul for 2D), not the full
    /// ~40-method surface — the rest is future work, not faked here.
    Tensor(Rc<TensorVal>),
}

pub struct TensorVal {
    pub data: RefCell<Vec<f64>>,
    pub shape: Vec<usize>,
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::None => false,
            Value::List(v) => !v.borrow().is_empty(),
            Value::Tuple(v) => !v.is_empty(),
            Value::Dict(d) => !d.borrow().is_empty(),
            _ => true,
        }
    }

    pub fn type_name(&self) -> String {
        match self {
            Value::Int(_) => "int".to_string(),
            Value::Float(_) => "float".to_string(),
            Value::Str(_) => "str".to_string(),
            Value::Bool(_) => "bool".to_string(),
            Value::None => "NoneType".to_string(),
            Value::List(_) => "list".to_string(),
            Value::Tuple(_) => "tuple".to_string(),
            Value::Dict(_) => "dict".to_string(),
            Value::Closure { .. } | Value::Function { .. } | Value::BoundMethod { .. } | Value::Builtin(_) => "function".to_string(),
            Value::Class(c) => c.name.clone(),
            Value::Instance(i) => i.class.name.clone(),
            Value::Tensor(_) => "Tensor".to_string(),
        }
    }

    pub fn eq_val(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => *a as f64 == *b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::None, Value::None) => true,
            (Value::List(a), Value::List(b)) => {
                let (a, b) = (a.borrow(), b.borrow());
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.eq_val(y))
            }
            (Value::Tuple(a), Value::Tuple(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.eq_val(y)),
            _ => false,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// Python-style `str()` — human-readable, not necessarily round-trippable.
    pub fn display(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                if f.fract() == 0.0 && f.is_finite() {
                    format!("{f:.1}")
                } else {
                    f.to_string()
                }
            }
            Value::Str(s) => s.to_string(),
            Value::Bool(b) => if *b { "True".to_string() } else { "False".to_string() },
            Value::None => "None".to_string(),
            Value::List(v) => format!("[{}]", v.borrow().iter().map(|x| x.repr()).collect::<Vec<_>>().join(", ")),
            Value::Tuple(v) => {
                if v.len() == 1 {
                    format!("({},)", v[0].repr())
                } else {
                    format!("({})", v.iter().map(|x| x.repr()).collect::<Vec<_>>().join(", "))
                }
            }
            Value::Dict(d) => format!(
                "{{{}}}",
                d.borrow().iter().map(|(k, v)| format!("{}: {}", k.repr(), v.repr())).collect::<Vec<_>>().join(", ")
            ),
            Value::Closure { .. } => "<function <lambda>>".to_string(),
            Value::Function { def, .. } => format!("<function {}>", def.name),
            Value::BoundMethod { def, .. } => format!("<bound method {}>", def.name),
            Value::Builtin(n) => format!("<built-in function {n}>"),
            Value::Class(c) => format!("<class '{}'>", c.name),
            Value::Instance(i) => format!("<{} object>", i.class.name),
            Value::Tensor(t) => format!("Tensor(shape={:?})", t.shape),
        }
    }

    /// Python-style `repr()` — used inside container displays (quotes strings).
    pub fn repr(&self) -> String {
        match self {
            Value::Str(s) => format!("'{s}'"),
            other => other.display(),
        }
    }
}
