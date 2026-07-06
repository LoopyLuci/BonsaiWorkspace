//! Helix runtime values. `Vec` (2/3/4 components) is a genuinely first-class
//! type with swizzle access, not a generic list reused for the purpose —
//! that's the point of it being its own variant rather than `Value::List`.
//! `Buffer` is a shared mutable array a kernel dispatch mutates in place
//! (the compute-shader SSBO idiom), distinct from a fixed-size `Vec`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
    pub fn declare(&self, name: &str, v: Value) {
        self.vars.borrow_mut().insert(name.to_string(), v);
    }
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
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Unit,
    /// A fixed-size (2, 3, or 4 component) vector — genuinely first-class,
    /// with swizzle access (`.xyz`, `.rgb`, `.x`), not a generic list.
    Vec(Rc<RefCell<Vec<f64>>>),
    /// A mutable shared buffer a `dispatch`ed kernel indexes by thread id
    /// and mutates in place — the compute-shader storage-buffer idiom.
    Buffer(Rc<RefCell<Vec<Value>>>),
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Unit => false,
            _ => true,
        }
    }

    pub fn type_name(&self) -> String {
        match self {
            Value::Int(_) => "int".to_string(),
            Value::Float(_) => "float".to_string(),
            Value::Bool(_) => "bool".to_string(),
            Value::Unit => "unit".to_string(),
            Value::Vec(v) => format!("vec{}", v.borrow().len()),
            Value::Buffer(_) => "buffer".to_string(),
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

    pub fn eq_val(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => *a as f64 == *b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Vec(a), Value::Vec(b)) => *a.borrow() == *b.borrow(),
            _ => false,
        }
    }

    pub fn display(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                if f.fract() == 0.0 && f.is_finite() {
                    format!("{f:.4}")
                } else {
                    format!("{f:.4}").trim_end_matches('0').trim_end_matches('.').to_string()
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::Unit => String::new(),
            Value::Vec(v) => format!("vec{}({})", v.borrow().len(), v.borrow().iter().map(|x| format!("{x:.4}")).collect::<Vec<_>>().join(", ")),
            Value::Buffer(b) => format!("[{}]", b.borrow().iter().map(|x| x.display()).collect::<Vec<_>>().join(", ")),
        }
    }
}
