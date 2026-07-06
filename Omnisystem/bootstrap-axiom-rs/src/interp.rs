//! Axiom verifier.
//!
//! This is not a program interpreter — it's a **checker**. Three genuinely
//! different verification techniques, each honestly scoped to what a
//! bootstrap this size can actually decide correctly:
//!
//!   - `axiom` — a ground (variable-free) proposition, assumed true without
//!     checking (by definition — that's what an axiom *is*), evaluated
//!     once so it can be referenced by name in theorems.
//!   - `theorem ... forall v in lo..hi, ...` — **bounded-exhaustive
//!     verification**: every combination of the quantified variables'
//!     explicit finite ranges is substituted in and the body is checked.
//!     If all combinations satisfy it, that's a genuine, complete proof —
//!     *for the stated finite domain* (this bootstrap says so honestly in
//!     its output; it is not first-order theorem proving over all
//!     integers). If any combination fails, the counterexample is real and
//!     reproducible, not a guess. A theorem with a free variable and no
//!     `forall` binding it is a real, reportable error — Axiom does not
//!     silently assume universality over an unbounded domain.
//!   - `invariant ... over states (...)` — checks the body holds across an
//!     **explicit, enumerated set of states** (TLA+-style state-space
//!     checking, simplified to a fixed list rather than reachability
//!     search from a transition relation — a real, honest scope
//!     boundary for a bootstrap, not reachability analysis).

use std::collections::HashMap;

use crate::ast::*;
use crate::diag::{OmniError, Phase, Span};
use crate::values::*;

pub type RResult<T> = Result<T, Box<OmniError>>;

pub struct Verifier {
    pub file: String,
    axiom_values: HashMap<String, Value>,
    pub out: String,
}

impl Verifier {
    pub fn new(file: &str) -> Self {
        Verifier { file: file.to_string(), axiom_values: HashMap::new(), out: String::new() }
    }

    fn rt(&self, msg: impl Into<String>, span: Span) -> Box<OmniError> {
        Box::new(OmniError::new(Phase::Runtime, msg, span, &self.file))
    }

    fn print(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }

    pub fn verify_module(&mut self, module: &Module) -> RResult<i32> {
        let mut all_ok = true;

        for a in &module.axioms {
            let env = Env::new();
            let v = self.eval(&a.body, &env)?;
            let Some(b) = v.as_bool() else {
                return Err(self.rt(format!("axiom '{}' must be a boolean proposition, got {}", a.name, v.type_name()), a.body.span()));
            };
            self.axiom_values.insert(a.name.clone(), Value::Bool(b));
            self.print(&format!("axiom {} -> ASSUMED ({})", a.name, b));
        }

        for t in &module.theorems {
            let ok = self.verify_theorem(t)?;
            all_ok &= ok;
        }

        for inv in &module.invariants {
            let ok = self.verify_invariant(inv)?;
            all_ok &= ok;
        }

        Ok(if all_ok { 0 } else { 1 })
    }

    fn base_env(&self) -> Env {
        self.axiom_values.clone()
    }

    fn verify_theorem(&mut self, t: &TheoremDef) -> RResult<bool> {
        if t.foralls.is_empty() {
            let env = self.base_env();
            let v = self.eval(&t.body, &env)?;
            let Some(b) = v.as_bool() else {
                return Err(self.rt(format!("theorem '{}' must be a boolean proposition, got {}", t.name, v.type_name()), t.body.span()));
            };
            self.print(&format!("theorem {} -> {}", t.name, if b { "PROVEN (ground)".to_string() } else { "DISPROVEN".to_string() }));
            return Ok(b);
        }

        let mut total = 1usize;
        for qb in &t.foralls {
            if qb.hi <= qb.lo {
                return Err(self.rt(format!("quantifier '{}' has an empty range {}..{}", qb.var, qb.lo, qb.hi), t.span));
            }
            total *= (qb.hi - qb.lo) as usize;
        }

        for combo in cartesian(&t.foralls) {
            let mut env = self.base_env();
            for (name, val) in &combo {
                env.insert(name.clone(), Value::Int(*val));
            }
            let v = self.eval(&t.body, &env)?;
            let Some(b) = v.as_bool() else {
                return Err(self.rt(format!("theorem '{}' must be a boolean proposition, got {}", t.name, v.type_name()), t.body.span()));
            };
            if !b {
                let ce = combo.iter().map(|(n, v)| format!("{n}={v}")).collect::<Vec<_>>().join(", ");
                self.print(&format!("theorem {} -> DISPROVEN counterexample: {ce} (checked {total} cases)", t.name));
                return Ok(false);
            }
        }
        self.print(&format!("theorem {} -> PROVEN (exhaustively verified over {total} cases)", t.name));
        Ok(true)
    }

    fn verify_invariant(&mut self, inv: &InvariantDef) -> RResult<bool> {
        for (i, state) in inv.states.iter().enumerate() {
            let mut env = self.base_env();
            for (name, val) in state {
                env.insert(name.clone(), Value::Int(*val));
            }
            let v = self.eval(&inv.body, &env)?;
            let Some(b) = v.as_bool() else {
                return Err(self.rt(format!("invariant '{}' must be a boolean proposition, got {}", inv.name, v.type_name()), inv.body.span()));
            };
            if !b {
                let state_str = state.iter().map(|(n, v)| format!("{n}={v}")).collect::<Vec<_>>().join(", ");
                self.print(&format!("invariant {} -> VIOLATED at state[{i}]: {state_str} (checked {} of {} states)", inv.name, i + 1, inv.states.len()));
                return Ok(false);
            }
        }
        self.print(&format!("invariant {} -> HOLDS (checked {} states)", inv.name, inv.states.len()));
        Ok(true)
    }

    fn eval(&self, expr: &Expr, env: &Env) -> RResult<Value> {
        match expr {
            Expr::Int { v, .. } => Ok(Value::Int(*v)),
            Expr::Bool { v, .. } => Ok(Value::Bool(*v)),
            Expr::Ident { name, span } => env
                .get(name)
                .copied()
                .ok_or_else(|| self.rt(format!("undefined name '{name}' (a theorem's free variables must all be bound by 'forall', with an explicit finite range)"), *span)),
            Expr::UnaryOp { op, expr, span } => {
                let v = self.eval(expr, env)?;
                match (op.as_str(), v) {
                    ("-", Value::Int(n)) => Ok(Value::Int(-n)),
                    ("not", Value::Bool(b)) => Ok(Value::Bool(!b)),
                    (op, v) => Err(self.rt(format!("bad operand type for unary '{op}': {}", v.type_name()), *span)),
                }
            }
            Expr::BinOp { op, left, right, span } => {
                let l = self.eval(left, env)?;
                if op == "and" {
                    let Some(lb) = l.as_bool() else { return Err(self.rt("'and' requires boolean operands", *span)) };
                    if !lb {
                        return Ok(Value::Bool(false));
                    }
                    let r = self.eval(right, env)?;
                    return r.as_bool().map(Value::Bool).ok_or_else(|| self.rt("'and' requires boolean operands", *span));
                }
                if op == "or" {
                    let Some(lb) = l.as_bool() else { return Err(self.rt("'or' requires boolean operands", *span)) };
                    if lb {
                        return Ok(Value::Bool(true));
                    }
                    let r = self.eval(right, env)?;
                    return r.as_bool().map(Value::Bool).ok_or_else(|| self.rt("'or' requires boolean operands", *span));
                }
                if op == "=>" {
                    let Some(lb) = l.as_bool() else { return Err(self.rt("'=>' requires boolean operands", *span)) };
                    if !lb {
                        return Ok(Value::Bool(true)); // false => anything == true
                    }
                    let r = self.eval(right, env)?;
                    return r.as_bool().map(Value::Bool).ok_or_else(|| self.rt("'=>' requires boolean operands", *span));
                }
                let r = self.eval(right, env)?;
                match op.as_str() {
                    "==" => Ok(Value::Bool(values_eq(l, r))),
                    "!=" => Ok(Value::Bool(!values_eq(l, r))),
                    "<" | "<=" | ">" | ">=" => {
                        let (Some(a), Some(b)) = (l.as_int(), r.as_int()) else {
                            return Err(self.rt(format!("'{op}' requires integer operands"), *span));
                        };
                        Ok(Value::Bool(match op.as_str() {
                            "<" => a < b,
                            "<=" => a <= b,
                            ">" => a > b,
                            ">=" => a >= b,
                            _ => unreachable!(),
                        }))
                    }
                    "+" | "-" | "*" | "/" | "%" => {
                        let (Some(a), Some(b)) = (l.as_int(), r.as_int()) else {
                            return Err(self.rt(format!("'{op}' requires integer operands"), *span));
                        };
                        match op.as_str() {
                            "+" => Ok(Value::Int(a + b)),
                            "-" => Ok(Value::Int(a - b)),
                            "*" => Ok(Value::Int(a * b)),
                            "/" => {
                                if b == 0 {
                                    Err(self.rt("division by zero", *span))
                                } else {
                                    Ok(Value::Int(a / b))
                                }
                            }
                            "%" => {
                                if b == 0 {
                                    Err(self.rt("modulo by zero", *span))
                                } else {
                                    Ok(Value::Int(a.rem_euclid(b)))
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                    other => Err(self.rt(format!("unknown operator '{other}'"), *span)),
                }
            }
        }
    }
}

fn values_eq(a: Value, b: Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => false,
    }
}

/// Full Cartesian product of every quantifier's explicit finite range, in
/// deterministic enumeration order (so a reported counterexample is always
/// the *first* one found in a stable, reproducible order, not arbitrary).
fn cartesian(bindings: &[QuantBinding]) -> Vec<Vec<(String, i64)>> {
    let mut out: Vec<Vec<(String, i64)>> = vec![vec![]];
    for qb in bindings {
        let mut next = Vec::with_capacity(out.len() * (qb.hi - qb.lo) as usize);
        for prefix in &out {
            for v in qb.lo..qb.hi {
                let mut combo = prefix.clone();
                combo.push((qb.var.clone(), v));
                next.push(combo);
            }
        }
        out = next;
    }
    out
}
