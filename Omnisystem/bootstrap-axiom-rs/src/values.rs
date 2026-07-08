//! Axiom values — deliberately minimal: everything here is either an
//! integer or a boolean proposition over integers. There's no mutation at
//! all (every binding is fresh per quantifier-combination or per-state
//! check), so unlike every other bootstrap in this series there's no
//! shared-cell `Env` — a plain immutable `HashMap` snapshot per check is
//! both simpler and more honestly matches "evaluating a proposition under
//! an assignment," which is exactly what this is.

use std::collections::HashMap;

pub type Env = HashMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Bool(_) => "bool",
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }
}
