//! Lightweight Datalog engine built on `datafrog`.
//!
//! Provides a declarative rule language for deriving facts from a base set,
//! using seminaive bottom-up fixpoint evaluation (datalog semantics).
//!
//! # Example
//! ```
//! use query::datalog::{Db, Rule, Atom, Term};
//!
//! let mut db = Db::new();
//! db.add_fact("edge", vec!["a".into(), "b".into()]);
//! db.add_fact("edge", vec!["b".into(), "c".into()]);
//! // rule: reachable(X, Y) :- edge(X, Y).
//! // rule: reachable(X, Z) :- reachable(X, Y), edge(Y, Z).
//! let results = db.query_transitive("edge", "reachable");
//! assert!(results.contains(&vec!["a".into(), "c".into()]));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

// ── Types ─────────────────────────────────────────────────────────────────────

/// A tuple of string values (a "fact" or "row").
pub type Tuple = Vec<String>;

/// A Datalog term — either a variable or a constant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Term {
    Var(String),
    Const(String),
}

/// A Datalog atom: a relation name + argument terms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atom {
    pub relation: String,
    pub args: Vec<Term>,
}

/// A Datalog rule: `head :- body`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub head: Atom,
    pub body: Vec<Atom>,
}

#[derive(Debug, Error)]
pub enum DatalogError {
    #[error("arity mismatch: relation {rel} expects arity {expected}, got {got}")]
    ArityMismatch {
        rel: String,
        expected: usize,
        got: usize,
    },
    #[error("unbound variable {0} in rule head")]
    UnboundVariable(String),
    #[error("{0}")]
    Other(String),
}

pub type DatalogResult<T> = Result<T, DatalogError>;

// ── Database ──────────────────────────────────────────────────────────────────

/// In-memory Datalog database.
pub struct Db {
    /// Base (EDB) facts: relation → set of tuples.
    facts: HashMap<String, HashSet<Tuple>>,
    /// Derived (IDB) facts.
    derived: HashMap<String, HashSet<Tuple>>,
    /// Rules.
    rules: Vec<Rule>,
}

impl Db {
    pub fn new() -> Self {
        Self {
            facts: HashMap::new(),
            derived: HashMap::new(),
            rules: Vec::new(),
        }
    }

    /// Add a base fact.
    pub fn add_fact(&mut self, relation: &str, tuple: Tuple) {
        self.facts.entry(relation.into()).or_default().insert(tuple);
    }

    /// Add a rule to the program.
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Query all facts (EDB + IDB) for a relation after running fixpoint.
    pub fn query(&mut self, relation: &str) -> Vec<Tuple> {
        self.fixpoint();
        let mut out: Vec<Tuple> = self
            .derived
            .get(relation)
            .or_else(|| self.facts.get(relation))
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        out.sort();
        out
    }

    /// Convenience: compute transitive closure of `base` into `derived_name`.
    ///
    /// Equivalent to:
    /// ```datalog
    /// derived_name(X,Y) :- base(X,Y).
    /// derived_name(X,Z) :- derived_name(X,Y), base(Y,Z).
    /// ```
    pub fn query_transitive(&mut self, base: &str, derived_name: &str) -> Vec<Tuple> {
        // Seed rule 1: derived :- base
        self.add_rule(Rule {
            head: Atom {
                relation: derived_name.into(),
                args: vec![Term::Var("X".into()), Term::Var("Y".into())],
            },
            body: vec![Atom {
                relation: base.into(),
                args: vec![Term::Var("X".into()), Term::Var("Y".into())],
            }],
        });
        // Rule 2: derived(X,Z) :- derived(X,Y), base(Y,Z)
        self.add_rule(Rule {
            head: Atom {
                relation: derived_name.into(),
                args: vec![Term::Var("X".into()), Term::Var("Z".into())],
            },
            body: vec![
                Atom {
                    relation: derived_name.into(),
                    args: vec![Term::Var("X".into()), Term::Var("Y".into())],
                },
                Atom {
                    relation: base.into(),
                    args: vec![Term::Var("Y".into()), Term::Var("Z".into())],
                },
            ],
        });
        self.query(derived_name)
    }

    // ── Fixpoint evaluation (naive, O(n²) per iteration) ──────────────────────

    fn fixpoint(&mut self) {
        // Seed derived with EDB
        for (rel, tuples) in &self.facts {
            self.derived
                .entry(rel.clone())
                .or_default()
                .extend(tuples.iter().cloned());
        }

        let rules = self.rules.clone();
        loop {
            let mut new_facts: HashMap<String, HashSet<Tuple>> = HashMap::new();
            for rule in &rules {
                let derived_facts = derive_rule(rule, &self.derived);
                for t in derived_facts {
                    new_facts
                        .entry(rule.head.relation.clone())
                        .or_default()
                        .insert(t);
                }
            }
            let mut changed = false;
            for (rel, tuples) in new_facts {
                let set = self.derived.entry(rel).or_default();
                for t in tuples {
                    if set.insert(t) {
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }
}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}

// ── Rule evaluation ───────────────────────────────────────────────────────────

fn derive_rule(rule: &Rule, db: &HashMap<String, HashSet<Tuple>>) -> Vec<Tuple> {
    // Start with one empty binding
    let mut bindings: Vec<HashMap<String, String>> = vec![HashMap::new()];

    for atom in &rule.body {
        let rows = db.get(&atom.relation);
        let mut next_bindings = Vec::new();
        for binding in &bindings {
            if let Some(rows) = rows {
                for row in rows {
                    if row.len() != atom.args.len() {
                        continue;
                    }
                    if let Some(ext) = extend_binding(binding, &atom.args, row) {
                        next_bindings.push(ext);
                    }
                }
            }
        }
        bindings = next_bindings;
    }

    // Apply bindings to rule head
    let mut out = Vec::new();
    for binding in &bindings {
        if let Some(t) = apply_binding(&rule.head.args, binding) {
            out.push(t);
        }
    }
    out
}

fn extend_binding(
    binding: &HashMap<String, String>,
    args: &[Term],
    row: &[String],
) -> Option<HashMap<String, String>> {
    let mut new_binding = binding.clone();
    for (arg, val) in args.iter().zip(row.iter()) {
        match arg {
            Term::Const(c) => {
                if c != val {
                    return None;
                }
            }
            Term::Var(v) => {
                if let Some(existing) = new_binding.get(v) {
                    if existing != val {
                        return None;
                    }
                } else {
                    new_binding.insert(v.clone(), val.clone());
                }
            }
        }
    }
    Some(new_binding)
}

fn apply_binding(args: &[Term], binding: &HashMap<String, String>) -> Option<Tuple> {
    args.iter()
        .map(|a| match a {
            Term::Const(c) => Some(c.clone()),
            Term::Var(v) => binding.get(v).cloned(),
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitive_closure() {
        let mut db = Db::new();
        db.add_fact("edge", vec!["a".into(), "b".into()]);
        db.add_fact("edge", vec!["b".into(), "c".into()]);
        db.add_fact("edge", vec!["c".into(), "d".into()]);
        let r = db.query_transitive("edge", "reach");
        let pairs: Vec<Vec<String>> = vec![
            vec!["a".into(), "b".into()],
            vec!["a".into(), "c".into()],
            vec!["a".into(), "d".into()],
            vec!["b".into(), "c".into()],
            vec!["b".into(), "d".into()],
            vec!["c".into(), "d".into()],
        ];
        for p in &pairs {
            assert!(r.contains(p), "missing: {p:?}");
        }
    }

    #[test]
    fn simple_join() {
        let mut db = Db::new();
        db.add_fact("parent", vec!["alice".into(), "bob".into()]);
        db.add_fact("parent", vec!["bob".into(), "carol".into()]);
        // grandparent(X,Z) :- parent(X,Y), parent(Y,Z).
        db.add_rule(Rule {
            head: Atom {
                relation: "grandparent".into(),
                args: vec![Term::Var("X".into()), Term::Var("Z".into())],
            },
            body: vec![
                Atom {
                    relation: "parent".into(),
                    args: vec![Term::Var("X".into()), Term::Var("Y".into())],
                },
                Atom {
                    relation: "parent".into(),
                    args: vec![Term::Var("Y".into()), Term::Var("Z".into())],
                },
            ],
        });
        let r = db.query("grandparent");
        assert_eq!(r, vec![vec!["alice".to_string(), "carol".to_string()]]);
    }

    #[test]
    fn constant_filter() {
        let mut db = Db::new();
        db.add_fact("color", vec!["sky".into(), "blue".into()]);
        db.add_fact("color", vec!["grass".into(), "green".into()]);
        // blue_things(X) :- color(X, "blue").
        db.add_rule(Rule {
            head: Atom {
                relation: "blue_things".into(),
                args: vec![Term::Var("X".into())],
            },
            body: vec![Atom {
                relation: "color".into(),
                args: vec![Term::Var("X".into()), Term::Const("blue".into())],
            }],
        });
        let r = db.query("blue_things");
        assert_eq!(r, vec![vec!["sky".to_string()]]);
    }
}
