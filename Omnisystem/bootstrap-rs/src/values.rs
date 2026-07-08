//! Runtime value model. Containers and structs are reference values
//! (`Rc<RefCell<..>>`) so `&mut self` methods mutate in place — matching the
//! semantics of the runnable Titan subset.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::ast::{Expr, FnItem};

#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(Rc<RefCell<String>>),
    Char(char),
    Unit,
    Tuple(Rc<RefCell<Vec<Value>>>),
    Vec(Rc<RefCell<Vec<Value>>>),
    /// key-string -> (key, value); BTreeMap keeps deterministic iteration.
    Map(Rc<RefCell<BTreeMap<String, (Value, Value)>>>),
    Set(Rc<RefCell<BTreeMap<String, Value>>>),
    Struct { name: Rc<str>, fields: Rc<RefCell<BTreeMap<String, Value>>> },
    Enum { enum_name: Rc<str>, variant: Rc<str>, payload: Rc<RefCell<Vec<Value>>> },
    Range { from: i64, to: i64, inclusive: bool },
    Fn { decl: Rc<FnItem>, self_val: Option<Box<Value>>, owner: Option<Rc<str>> },
    Closure { params: Rc<Vec<String>>, body: Rc<Expr>, env: Env },
    /// Deferred builtin path (e.g. `Vec::new` passed as a function value).
    Builtin(Rc<str>),
}

pub fn vstr(s: impl Into<String>) -> Value {
    Value::Str(Rc::new(RefCell::new(s.into())))
}
pub fn vvec(items: Vec<Value>) -> Value {
    Value::Vec(Rc::new(RefCell::new(items)))
}
pub fn vtuple(items: Vec<Value>) -> Value {
    Value::Tuple(Rc::new(RefCell::new(items)))
}
pub fn venum(enum_name: &str, variant: &str, payload: Vec<Value>) -> Value {
    Value::Enum {
        enum_name: Rc::from(enum_name),
        variant: Rc::from(variant),
        payload: Rc::new(RefCell::new(payload)),
    }
}
pub fn some(v: Value) -> Value {
    venum("Option", "Some", vec![v])
}
pub fn none() -> Value {
    venum("Option", "None", vec![])
}
pub fn ok(v: Value) -> Value {
    venum("Result", "Ok", vec![v])
}
pub fn err(v: Value) -> Value {
    venum("Result", "Err", vec![v])
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Unit => false,
            _ => true,
        }
    }

    /// Stable hashable key for map/set membership and equality.
    pub fn key(&self) -> String {
        match self {
            Value::Int(v) => format!("n:{v}"),
            Value::Float(v) => format!("n:{v}"),
            Value::Bool(v) => format!("b:{v}"),
            Value::Str(s) => format!("s:{}", s.borrow()),
            Value::Char(c) => format!("c:{c}"),
            Value::Unit => "unit".into(),
            Value::Tuple(items) => {
                format!("t:{}", items.borrow().iter().map(|v| v.key()).collect::<Vec<_>>().join(","))
            }
            Value::Vec(items) => {
                format!("v:{}", items.borrow().iter().map(|v| v.key()).collect::<Vec<_>>().join(","))
            }
            Value::Map(m) => {
                format!("m:{}", m.borrow().keys().cloned().collect::<Vec<_>>().join(","))
            }
            Value::Set(s) => {
                format!("hs:{}", s.borrow().keys().cloned().collect::<Vec<_>>().join(","))
            }
            Value::Struct { name, fields } => format!(
                "st:{name}:{}",
                fields.borrow().iter().map(|(k, v)| format!("{k}={}", v.key())).collect::<Vec<_>>().join(",")
            ),
            Value::Enum { enum_name, variant, payload } => format!(
                "e:{enum_name}:{variant}:{}",
                payload.borrow().iter().map(|v| v.key()).collect::<Vec<_>>().join(",")
            ),
            Value::Range { from, to, inclusive } => format!("r:{from}:{to}:{inclusive}"),
            Value::Fn { decl, .. } => format!("fn:{}", decl.name),
            Value::Closure { .. } => "closure".into(),
            Value::Builtin(name) => format!("builtin:{name}"),
        }
    }

    pub fn eq_val(&self, other: &Value) -> bool {
        // numeric cross-type equality (int vs float)
        match (self, other) {
            (Value::Int(a), Value::Float(b)) => *a as f64 == *b,
            (Value::Float(a), Value::Int(b)) => *a == *b as f64,
            _ => self.key() == other.key(),
        }
    }

    pub fn type_name(&self) -> Option<&str> {
        match self {
            Value::Struct { name, .. } => Some(name),
            Value::Enum { enum_name, .. } => Some(enum_name),
            Value::Vec(_) => Some("Vec"),
            Value::Map(_) => Some("HashMap"),
            Value::Set(_) => Some("HashSet"),
            Value::Str(_) => Some("String"),
            _ => None,
        }
    }

    pub fn display(&self) -> String {
        match self {
            Value::Int(v) => v.to_string(),
            Value::Float(v) => {
                if v.fract() == 0.0 && v.is_finite() && v.abs() < 1e15 {
                    format!("{v:.1}")
                } else {
                    v.to_string()
                }
            }
            Value::Bool(v) => v.to_string(),
            Value::Str(s) => s.borrow().clone(),
            Value::Char(c) => c.to_string(),
            Value::Unit => "()".into(),
            Value::Tuple(items) => {
                format!("({})", items.borrow().iter().map(|v| v.display()).collect::<Vec<_>>().join(", "))
            }
            Value::Vec(items) => {
                format!("[{}]", items.borrow().iter().map(|v| v.debug()).collect::<Vec<_>>().join(", "))
            }
            Value::Set(s) => {
                format!("{{{}}}", s.borrow().values().map(|v| v.debug()).collect::<Vec<_>>().join(", "))
            }
            Value::Map(m) => format!(
                "{{{}}}",
                m.borrow().values().map(|(k, v)| format!("{}: {}", k.debug(), v.debug())).collect::<Vec<_>>().join(", ")
            ),
            Value::Range { from, to, inclusive } => {
                format!("{from}..{}{to}", if *inclusive { "=" } else { "" })
            }
            Value::Enum { variant, payload, .. } => {
                let p = payload.borrow();
                if p.is_empty() {
                    variant.to_string()
                } else {
                    format!("{variant}({})", p.iter().map(|v| v.debug()).collect::<Vec<_>>().join(", "))
                }
            }
            Value::Struct { name, fields } => format!(
                "{name} {{ {} }}",
                fields.borrow().iter().map(|(k, v)| format!("{k}: {}", v.debug())).collect::<Vec<_>>().join(", ")
            ),
            Value::Fn { decl, .. } => format!("fn {}", decl.name),
            Value::Closure { .. } => "closure".into(),
            Value::Builtin(name) => format!("builtin {name}"),
        }
    }

    pub fn debug(&self) -> String {
        match self {
            Value::Str(s) => format!("{:?}", s.borrow()),
            Value::Char(c) => format!("'{c}'"),
            _ => self.display(),
        }
    }

    pub fn deep_clone(&self) -> Value {
        match self {
            Value::Str(s) => vstr(s.borrow().clone()),
            Value::Tuple(items) => vtuple(items.borrow().iter().map(|v| v.deep_clone()).collect()),
            Value::Vec(items) => vvec(items.borrow().iter().map(|v| v.deep_clone()).collect()),
            Value::Map(m) => Value::Map(Rc::new(RefCell::new(
                m.borrow().iter().map(|(k, (kk, vv))| (k.clone(), (kk.deep_clone(), vv.deep_clone()))).collect(),
            ))),
            Value::Set(s) => Value::Set(Rc::new(RefCell::new(
                s.borrow().iter().map(|(k, v)| (k.clone(), v.deep_clone())).collect(),
            ))),
            Value::Struct { name, fields } => Value::Struct {
                name: name.clone(),
                fields: Rc::new(RefCell::new(
                    fields.borrow().iter().map(|(k, v)| (k.clone(), v.deep_clone())).collect(),
                )),
            },
            Value::Enum { enum_name, variant, payload } => Value::Enum {
                enum_name: enum_name.clone(),
                variant: variant.clone(),
                payload: Rc::new(RefCell::new(payload.borrow().iter().map(|v| v.deep_clone()).collect())),
            },
            other => other.clone(),
        }
    }
}

// ── environments ─────────────────────────────────────────────────────────────
pub struct Scope {
    pub vars: BTreeMap<String, Value>,
    pub parent: Option<Env>,
}

#[derive(Clone)]
pub struct Env(pub Rc<RefCell<Scope>>);

impl Env {
    pub fn new() -> Self {
        Env(Rc::new(RefCell::new(Scope { vars: BTreeMap::new(), parent: None })))
    }

    pub fn child(&self) -> Env {
        Env(Rc::new(RefCell::new(Scope { vars: BTreeMap::new(), parent: Some(self.clone()) })))
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        let scope = self.0.borrow();
        if let Some(v) = scope.vars.get(name) {
            return Some(v.clone());
        }
        scope.parent.as_ref().and_then(|p| p.get(name))
    }

    pub fn set(&self, name: &str, v: Value) {
        self.0.borrow_mut().vars.insert(name.to_string(), v);
    }

    /// Assign to an existing binding in the nearest enclosing scope.
    pub fn assign(&self, name: &str, v: Value) -> bool {
        let mut scope = self.0.borrow_mut();
        if scope.vars.contains_key(name) {
            scope.vars.insert(name.to_string(), v);
            return true;
        }
        let parent = scope.parent.clone();
        drop(scope);
        parent.map(|p| p.assign(name, v)).unwrap_or(false)
    }
}
