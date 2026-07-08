//! Vera runtime values. Every binding (prop, state, local) is stored as a
//! shared mutable cell (`Rc<RefCell<Value>>`) rather than a plain value —
//! this is what lets `state count = 0` be genuinely reactive: assigning
//! `count = count + 1` inside a method mutates the same cell the last
//! render read from, so re-rendering afterward observably reflects the
//! change (verified by a real before/after tree-diff test, not asserted).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{ComponentDef, Stmt};

pub type Cell = Rc<RefCell<Value>>;
pub type Env = Rc<Scope>;

pub struct Scope {
    pub vars: RefCell<HashMap<String, Cell>>,
    pub parent: Option<Env>,
}

impl Scope {
    pub fn new() -> Env {
        Rc::new(Scope { vars: RefCell::new(HashMap::new()), parent: None })
    }
    pub fn child(self: &Env) -> Env {
        Rc::new(Scope { vars: RefCell::new(HashMap::new()), parent: Some(self.clone()) })
    }
    pub fn declare(&self, name: &str, v: Value) {
        self.vars.borrow_mut().insert(name.to_string(), Rc::new(RefCell::new(v)));
    }
    pub fn get_cell(&self, name: &str) -> Option<Cell> {
        if let Some(c) = self.vars.borrow().get(name) {
            return Some(c.clone());
        }
        self.parent.as_ref().and_then(|p| p.get_cell(name))
    }
    pub fn get(&self, name: &str) -> Option<Value> {
        self.get_cell(name).map(|c| c.borrow().clone())
    }
    /// Assigns to the nearest enclosing scope that already declares `name`
    /// (see Sylva/Aether's identical `assign_existing_or_local` — same
    /// reasoning: without it, reassigning `count` inside a nested block
    /// would shadow instead of mutate the reactive cell).
    pub fn assign_existing_or_local(self: &Env, name: &str, v: Value) {
        if let Some(cell) = self.get_cell(name) {
            *cell.borrow_mut() = v;
        } else {
            self.declare(name, v);
        }
    }
}

pub struct ElementVal {
    pub tag: String,
    pub attrs: Vec<(String, Value)>,
    pub children: Vec<Value>,
}

pub struct ComponentInstance {
    pub def: Rc<ComponentDef>,
    pub env: Env, // holds props + state cells; computed + methods re-evaluated against it
}

#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Bool(bool),
    Unit,
    List(Rc<RefCell<Vec<Value>>>),
    Element(Rc<ElementVal>),
    Closure { params: Rc<Vec<String>>, body: Rc<Vec<Stmt>>, env: Env },
    ComponentClass(Rc<ComponentDef>),
    ComponentInstance(Rc<ComponentInstance>),
    Builtin(&'static str),
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::Unit => false,
            Value::List(l) => !l.borrow().is_empty(),
            _ => true,
        }
    }

    pub fn type_name(&self) -> String {
        match self {
            Value::Int(_) => "int".to_string(),
            Value::Float(_) => "float".to_string(),
            Value::Str(_) => "string".to_string(),
            Value::Bool(_) => "bool".to_string(),
            Value::Unit => "unit".to_string(),
            Value::List(_) => "list".to_string(),
            Value::Element(_) => "element".to_string(),
            Value::Closure { .. } | Value::Builtin(_) => "function".to_string(),
            Value::ComponentClass(c) => format!("component<{}>", c.name),
            Value::ComponentInstance(i) => format!("instance<{}>", i.def.name),
        }
    }

    pub fn eq_val(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => *a as f64 == *b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
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
            Value::Unit => String::new(),
            Value::List(l) => l.borrow().iter().map(|v| v.display()).collect::<Vec<_>>().join(", "),
            Value::Element(e) => render_element_to_string(e, 0),
            Value::Closure { .. } => "<closure>".to_string(),
            Value::Builtin(n) => format!("<builtin {n}>"),
            Value::ComponentClass(c) => format!("<component {}>", c.name),
            Value::ComponentInstance(i) => format!("<instance {}>", i.def.name),
        }
    }
}

/// Serializes a rendered element tree to a deterministic, human-readable
/// string — this is the bootstrap's stand-in for an actual browser/GPU
/// renderer: real UI output isn't available in this environment, but the
/// *tree* a render pass produces is fully real data, and printing it lets
/// tests assert on genuine before/after reactivity (a re-render after a
/// state change produces textually different, correct output).
pub fn render_element_to_string(e: &ElementVal, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let attrs = if e.attrs.is_empty() {
        String::new()
    } else {
        format!(" {}", e.attrs.iter().map(|(k, v)| format!("{k}={}", v.display())).collect::<Vec<_>>().join(" "))
    };
    if e.children.is_empty() {
        return format!("{pad}<{}{attrs}/>", e.tag);
    }
    let mut out = format!("{pad}<{}{attrs}>\n", e.tag);
    for c in &e.children {
        match c {
            Value::Element(child) => out.push_str(&render_element_to_string(child, indent + 1)),
            other => out.push_str(&format!("{}{}", "  ".repeat(indent + 1), other.display())),
        }
        out.push('\n');
    }
    out.push_str(&format!("{pad}</{}>", e.tag));
    out
}
