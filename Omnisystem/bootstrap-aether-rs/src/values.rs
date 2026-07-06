//! Aether runtime values. Atoms are interned as plain `Rc<str>` (good enough
//! for a bootstrap — real Erlang interns into a global atom table for O(1)
//! equality; this compares by string content instead, which is correct but
//! not maximally fast). Actors are modeled as a **cooperative, synchronous**
//! mailbox — see `ActorInstance` doc comment for the honest simplification
//! this makes vs. real preemptive BEAM-style concurrency.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{ActorDef, FnClause};

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

    /// Assigns to the nearest enclosing scope that already binds `name`
    /// (so `total = total + n` inside a `for`'s per-iteration child scope
    /// updates the outer `total`, not a new shadowing local); declares it
    /// in the current scope if not bound anywhere yet.
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

/// All clauses sharing one (name, arity) — Erlang/Elixir semantics: tried
/// top-to-bottom, first pattern (+ guard) match wins.
pub struct FnClauses {
    pub name: String,
    pub clauses: Vec<Rc<FnClause>>,
    pub env: Env,
}

/// A spawned actor instance. **Simplification, stated plainly**: this
/// bootstrap has no OS threads/event loop — `send` synchronously invokes the
/// matching `receive` clause immediately and updates `state` in place. Real
/// Aether (or a later, non-bootstrap implementation) would need an actual
/// scheduler with asynchronous mailboxes for genuine concurrency; faking
/// that here with `Rc<RefCell<...>>` single-threaded mutation would be
/// dishonest about what's actually implemented, so it's documented instead.
pub struct ActorInstance {
    pub def: Rc<ActorDef>,
    pub state: RefCell<Value>,
    pub env: Env,
}

#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Bool(bool),
    Nil,
    Atom(Rc<str>),
    Tuple(Rc<Vec<Value>>),
    List(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<Vec<(Value, Value)>>>),
    FnClauses(Rc<FnClauses>),
    Lambda { params: Rc<Vec<crate::ast::Pattern>>, body: Rc<crate::ast::Expr>, env: Env },
    Actor(Rc<ActorInstance>),
    Builtin(&'static str),
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Nil => false,
            Value::Atom(a) => a.as_ref() != "false" && a.as_ref() != "nil",
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::List(l) => !l.borrow().is_empty(),
            _ => true,
        }
    }

    pub fn type_name(&self) -> String {
        match self {
            Value::Int(_) => "integer".to_string(),
            Value::Float(_) => "float".to_string(),
            Value::Str(_) => "string".to_string(),
            Value::Bool(_) => "boolean".to_string(),
            Value::Nil => "nil".to_string(),
            Value::Atom(_) => "atom".to_string(),
            Value::Tuple(_) => "tuple".to_string(),
            Value::List(_) => "list".to_string(),
            Value::Map(_) => "map".to_string(),
            Value::FnClauses(_) | Value::Lambda { .. } | Value::Builtin(_) => "function".to_string(),
            Value::Actor(_) => "actor".to_string(),
        }
    }

    pub fn eq_val(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => *a as f64 == *b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Atom(a), Value::Atom(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Tuple(a), Value::Tuple(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.eq_val(y)),
            (Value::List(a), Value::List(b)) => {
                let (a, b) = (a.borrow(), b.borrow());
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.eq_val(y))
            }
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
            Value::Bool(b) => b.to_string(),
            Value::Nil => "nil".to_string(),
            Value::Atom(a) => format!(":{a}"),
            Value::Tuple(v) => format!("{{{}}}", v.iter().map(|x| x.display()).collect::<Vec<_>>().join(", ")),
            Value::List(v) => format!("[{}]", v.borrow().iter().map(|x| x.display()).collect::<Vec<_>>().join(", ")),
            Value::Map(m) => format!("%{{{}}}", m.borrow().iter().map(|(k, v)| format!("{}: {}", k.display(), v.display())).collect::<Vec<_>>().join(", ")),
            Value::FnClauses(f) => format!("#Function<{}/{}>", f.name, f.clauses.first().map(|c| c.params.len()).unwrap_or(0)),
            Value::Lambda { .. } => "#Function<lambda>".to_string(),
            Value::Actor(a) => format!("#Actor<{}>", a.def.name),
            Value::Builtin(n) => format!("#Function<{n}>"),
        }
    }
}
