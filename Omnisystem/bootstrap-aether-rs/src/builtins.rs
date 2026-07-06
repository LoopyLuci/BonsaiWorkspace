//! Aether builtins — `IO.puts`-style output plus list/tuple/map helpers.
//! Kept intentionally small: the point of this bootstrap is the language's
//! own structural features (multi-clause dispatch, actors, pattern
//! matching), not a large stdlib surface.

use std::cell::RefCell;
use std::rc::Rc;

use crate::diag::Span;
use crate::interp::{EResult, Flow, Interp};
use crate::values::Value;

pub fn install_globals(interp: &Interp) {
    const NAMES: &[&str] = &["puts", "print", "length", "hd", "tl", "elem", "map", "filter", "reduce", "reverse", "sum", "abs", "to_string", "is_atom", "is_list", "is_tuple"];
    for n in NAMES {
        interp.globals.set(n, Value::Builtin(n));
    }
}

fn err(msg: impl Into<String>, span: Span) -> Flow {
    Flow::Error(Box::new(crate::diag::OmniError::new(crate::diag::Phase::Runtime, msg, span, "")))
}

pub fn call_builtin(interp: &mut Interp, name: &str, args: Vec<Value>, span: Span) -> EResult {
    match name {
        "puts" | "print" => {
            let s = args.iter().map(|a| a.display()).collect::<Vec<_>>().join(" ");
            interp.print(&s);
            interp.print("\n");
            Ok(Value::Atom(Rc::from("ok")))
        }
        "length" => match &args[0] {
            Value::List(l) => Ok(Value::Int(l.borrow().len() as i64)),
            Value::Tuple(t) => Ok(Value::Int(t.len() as i64)),
            Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
            other => Err(err(format!("length/1 expects a list, tuple, or string, got {}", other.type_name()), span)),
        },
        "hd" => match &args[0] {
            Value::List(l) => l.borrow().first().cloned().ok_or_else(|| err("hd/1 of empty list", span)),
            other => Err(err(format!("hd/1 expects a list, got {}", other.type_name()), span)),
        },
        "tl" => match &args[0] {
            Value::List(l) => {
                let l = l.borrow();
                if l.is_empty() {
                    Err(err("tl/1 of empty list", span))
                } else {
                    Ok(Value::List(Rc::new(RefCell::new(l[1..].to_vec()))))
                }
            }
            other => Err(err(format!("tl/1 expects a list, got {}", other.type_name()), span)),
        },
        "elem" => {
            let idx = args[1].as_f64().unwrap_or(0.0) as usize;
            match &args[0] {
                Value::Tuple(t) => t.get(idx).cloned().ok_or_else(|| err("elem/2 index out of range", span)),
                other => Err(err(format!("elem/2 expects a tuple, got {}", other.type_name()), span)),
            }
        }
        "map" => {
            let Value::List(l) = &args[1] else { return Err(err("map/2's second argument must be a list", span)) };
            let items = l.borrow().clone();
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                out.push(interp.call_value(&args[0], vec![it], span)?);
            }
            Ok(Value::List(Rc::new(RefCell::new(out))))
        }
        "filter" => {
            let Value::List(l) = &args[1] else { return Err(err("filter/2's second argument must be a list", span)) };
            let items = l.borrow().clone();
            let mut out = Vec::new();
            for it in items {
                if interp.call_value(&args[0], vec![it.clone()], span)?.truthy() {
                    out.push(it);
                }
            }
            Ok(Value::List(Rc::new(RefCell::new(out))))
        }
        "reduce" => {
            let Value::List(l) = &args[0] else { return Err(err("reduce/3's first argument must be a list", span)) };
            let items = l.borrow().clone();
            let mut acc = args[1].clone();
            for it in items {
                acc = interp.call_value(&args[2], vec![acc, it], span)?;
            }
            Ok(acc)
        }
        "reverse" => match &args[0] {
            Value::List(l) => {
                let mut v = l.borrow().clone();
                v.reverse();
                Ok(Value::List(Rc::new(RefCell::new(v))))
            }
            other => Err(err(format!("reverse/1 expects a list, got {}", other.type_name()), span)),
        },
        "sum" => match &args[0] {
            Value::List(l) => {
                let items = l.borrow();
                let total: f64 = items.iter().map(|v| v.as_f64().unwrap_or(0.0)).sum();
                if items.iter().all(|v| matches!(v, Value::Int(_))) {
                    Ok(Value::Int(total as i64))
                } else {
                    Ok(Value::Float(total))
                }
            }
            other => Err(err(format!("sum/1 expects a list, got {}", other.type_name()), span)),
        },
        "abs" => match &args[0] {
            Value::Int(n) => Ok(Value::Int(n.abs())),
            Value::Float(f) => Ok(Value::Float(f.abs())),
            other => Err(err(format!("abs/1 expects a number, got {}", other.type_name()), span)),
        },
        "to_string" => Ok(Value::Str(Rc::from(args[0].display().as_str()))),
        "is_atom" => Ok(Value::Bool(matches!(args[0], Value::Atom(_)))),
        "is_list" => Ok(Value::Bool(matches!(args[0], Value::List(_)))),
        "is_tuple" => Ok(Value::Bool(matches!(args[0], Value::Tuple(_)))),
        other => Err(err(format!("unknown builtin '{other}'"), span)),
    }
}
