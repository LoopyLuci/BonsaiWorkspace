//! Sylva builtins — global functions (`print`, `len`, `range`, ...), method
//! dispatch for list/dict/str values, and a real (if minimal) `Tensor`
//! constructor set for Sylva's ML domain.

use std::cell::RefCell;
use std::rc::Rc;

use crate::diag::Span;
use crate::interp::{EResult, Interp};
use crate::values::{TensorVal, Value};

/// Registers every global builtin name into the interpreter's global scope
/// as a `Value::Builtin`, dispatched by `call_builtin`.
pub fn install_globals(interp: &Interp) {
    const NAMES: &[&str] = &[
        "print", "len", "range", "str", "int", "float", "bool", "list", "dict", "tuple", "abs",
        "min", "max", "sum", "sorted", "reversed", "enumerate", "zip", "map", "filter", "any",
        "all", "round", "isinstance", "type", "input",
        // Tensor / ML domain constructors — the real, minimal subset of
        // SYLVA_STANDARD_LIBRARY.sylva's Tensor spec.
        "zeros", "ones", "tensor",
    ];
    for n in NAMES {
        interp.globals.set(n, Value::Builtin(n));
    }
}

pub fn call_builtin(interp: &mut Interp, name: &str, args: Vec<Value>, span: Span) -> EResult {
    match name {
        "print" => {
            let s = args.iter().map(|a| a.display()).collect::<Vec<_>>().join(" ");
            interp.print(&s);
            interp.print("\n");
            Ok(Value::None)
        }
        "len" => Ok(Value::Int(value_len(&args[0], interp, span)? as i64)),
        "range" => build_range(&args, interp, span),
        "str" => Ok(Value::Str(Rc::from(args.first().map(|a| a.display()).unwrap_or_default().as_str()))),
        "int" => Ok(Value::Int(match args.first() {
            Some(Value::Str(s)) => s.trim().parse().map_err(|_| interp_err(interp, format!("invalid literal for int(): '{s}'"), span))?,
            Some(v) => v.as_f64().ok_or_else(|| interp_err(interp, "cannot convert to int".to_string(), span))? as i64,
            None => 0,
        })),
        "float" => Ok(Value::Float(match args.first() {
            Some(Value::Str(s)) => s.trim().parse().map_err(|_| interp_err(interp, format!("could not convert string to float: '{s}'"), span))?,
            Some(v) => v.as_f64().ok_or_else(|| interp_err(interp, "cannot convert to float".to_string(), span))?,
            None => 0.0,
        })),
        "bool" => Ok(Value::Bool(args.first().map(|a| a.truthy()).unwrap_or(false))),
        "list" => {
            let items = match args.first() {
                Some(v) => iter_to_vec(v, interp, span)?,
                None => vec![],
            };
            Ok(Value::List(Rc::new(RefCell::new(items))))
        }
        "dict" => Ok(Value::Dict(Rc::new(RefCell::new(vec![])))),
        "tuple" => {
            let items = match args.first() {
                Some(v) => iter_to_vec(v, interp, span)?,
                None => vec![],
            };
            Ok(Value::Tuple(Rc::new(items)))
        }
        "abs" => match &args[0] {
            Value::Int(n) => Ok(Value::Int(n.abs())),
            Value::Float(f) => Ok(Value::Float(f.abs())),
            other => Err(interp_err(interp, format!("bad operand type for abs(): '{}'", other.type_name()), span)),
        },
        "min" | "max" => {
            let items = if args.len() == 1 { iter_to_vec(&args[0], interp, span)? } else { args };
            if items.is_empty() {
                return Err(interp_err(interp, format!("{name}() arg is an empty sequence"), span));
            }
            let mut best = items[0].clone();
            for it in &items[1..] {
                let (a, b) = (best.as_f64(), it.as_f64());
                if let (Some(a), Some(b)) = (a, b) {
                    if (name == "min" && b < a) || (name == "max" && b > a) {
                        best = it.clone();
                    }
                }
            }
            Ok(best)
        }
        "sum" => {
            let items = iter_to_vec(&args[0], interp, span)?;
            let mut total = 0.0;
            let mut all_int = true;
            for it in &items {
                match it {
                    Value::Float(_) => all_int = false,
                    _ => {}
                }
                total += it.as_f64().unwrap_or(0.0);
            }
            Ok(if all_int { Value::Int(total as i64) } else { Value::Float(total) })
        }
        "sorted" => {
            let mut items = iter_to_vec(&args[0], interp, span)?;
            items.sort_by(|a, b| a.as_f64().partial_cmp(&b.as_f64()).unwrap_or(std::cmp::Ordering::Equal));
            Ok(Value::List(Rc::new(RefCell::new(items))))
        }
        "reversed" => {
            let mut items = iter_to_vec(&args[0], interp, span)?;
            items.reverse();
            Ok(Value::List(Rc::new(RefCell::new(items))))
        }
        "enumerate" => {
            let items = iter_to_vec(&args[0], interp, span)?;
            let out: Vec<Value> = items.into_iter().enumerate().map(|(i, v)| Value::Tuple(Rc::new(vec![Value::Int(i as i64), v]))).collect();
            Ok(Value::List(Rc::new(RefCell::new(out))))
        }
        "zip" => {
            let seqs: Vec<Vec<Value>> = args.iter().map(|a| iter_to_vec(a, interp, span)).collect::<Result<_, _>>()?;
            let min_len = seqs.iter().map(|s| s.len()).min().unwrap_or(0);
            let mut out = Vec::with_capacity(min_len);
            for i in 0..min_len {
                out.push(Value::Tuple(Rc::new(seqs.iter().map(|s| s[i].clone()).collect())));
            }
            Ok(Value::List(Rc::new(RefCell::new(out))))
        }
        "map" => {
            let items = iter_to_vec(&args[1], interp, span)?;
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                out.push(interp.call_value(&args[0], vec![it], vec![], span)?);
            }
            Ok(Value::List(Rc::new(RefCell::new(out))))
        }
        "filter" => {
            let items = iter_to_vec(&args[1], interp, span)?;
            let mut out = Vec::new();
            for it in items {
                if interp.call_value(&args[0], vec![it.clone()], vec![], span)?.truthy() {
                    out.push(it);
                }
            }
            Ok(Value::List(Rc::new(RefCell::new(out))))
        }
        "any" => Ok(Value::Bool(iter_to_vec(&args[0], interp, span)?.iter().any(|v| v.truthy()))),
        "all" => Ok(Value::Bool(iter_to_vec(&args[0], interp, span)?.iter().all(|v| v.truthy()))),
        "round" => {
            let n = args[0].as_f64().unwrap_or(0.0);
            let digits = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
            let mult = 10f64.powi(digits);
            let r = (n * mult).round() / mult;
            Ok(if digits <= 0 && args.len() < 2 { Value::Int(r as i64) } else { Value::Float(r) })
        }
        "isinstance" => {
            let type_name = match &args[1] {
                Value::Class(c) => c.name.clone(),
                v => v.display(),
            };
            Ok(Value::Bool(match &args[0] {
                Value::Instance(i) => i.class.is_or_extends(&type_name),
                other => other.type_name() == type_name,
            }))
        }
        "type" => Ok(Value::Str(Rc::from(args[0].type_name().as_str()))),
        "input" => Ok(Value::Str(Rc::from(""))), // no stdin in this bootstrap; returns empty
        "zeros" => make_tensor(&args, 0.0, interp, span),
        "ones" => make_tensor(&args, 1.0, interp, span),
        "tensor" => {
            let items = iter_to_vec(&args[0], interp, span)?;
            let data: Vec<f64> = items.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect();
            let len = data.len();
            Ok(Value::Tensor(Rc::new(TensorVal { data: RefCell::new(data), shape: vec![len] })))
        }
        other => Err(interp_err(interp, format!("unknown builtin '{other}'"), span)),
    }
}

fn make_tensor(args: &[Value], fill: f64, interp: &mut Interp, span: Span) -> EResult {
    let shape: Vec<usize> = if args.len() == 1 {
        match &args[0] {
            Value::Int(n) => vec![*n as usize],
            Value::List(l) => l.borrow().iter().map(|v| v.as_f64().unwrap_or(0.0) as usize).collect(),
            _ => return Err(interp_err(interp, "shape must be an int or list of ints".to_string(), span)),
        }
    } else {
        args.iter().map(|v| v.as_f64().unwrap_or(0.0) as usize).collect()
    };
    let total: usize = shape.iter().product::<usize>().max(if shape.is_empty() { 0 } else { 1 });
    Ok(Value::Tensor(Rc::new(TensorVal { data: RefCell::new(vec![fill; total]), shape })))
}

fn build_range(args: &[Value], interp: &mut Interp, span: Span) -> EResult {
    let (start, stop, step) = match args.len() {
        1 => (0, args[0].as_f64().unwrap_or(0.0) as i64, 1),
        2 => (args[0].as_f64().unwrap_or(0.0) as i64, args[1].as_f64().unwrap_or(0.0) as i64, 1),
        3 => (args[0].as_f64().unwrap_or(0.0) as i64, args[1].as_f64().unwrap_or(0.0) as i64, args[2].as_f64().unwrap_or(1.0) as i64),
        _ => return Err(interp_err(interp, "range() takes 1 to 3 arguments".to_string(), span)),
    };
    if step == 0 {
        return Err(interp_err(interp, "range() arg 3 must not be zero".to_string(), span));
    }
    let mut out = Vec::new();
    let mut i = start;
    while (step > 0 && i < stop) || (step < 0 && i > stop) {
        out.push(Value::Int(i));
        i += step;
    }
    Ok(Value::List(Rc::new(RefCell::new(out))))
}

fn value_len(v: &Value, interp: &mut Interp, span: Span) -> Result<usize, crate::interp::Flow> {
    Ok(match v {
        Value::List(l) => l.borrow().len(),
        Value::Tuple(t) => t.len(),
        Value::Str(s) => s.chars().count(),
        Value::Dict(d) => d.borrow().len(),
        other => return Err(interp_err(interp, format!("object of type '{}' has no len()", other.type_name()), span)),
    })
}

fn iter_to_vec(v: &Value, interp: &mut Interp, span: Span) -> Result<Vec<Value>, crate::interp::Flow> {
    match v {
        Value::List(l) => Ok(l.borrow().clone()),
        Value::Tuple(t) => Ok((**t).clone()),
        Value::Str(s) => Ok(s.chars().map(|c| Value::Str(Rc::from(c.to_string().as_str()))).collect()),
        Value::Dict(d) => Ok(d.borrow().iter().map(|(k, _)| k.clone()).collect()),
        other => Err(interp_err(interp, format!("'{}' object is not iterable", other.type_name()), span)),
    }
}

/// Builtins raise the same catchable-exception `Flow::Raise` as `Interp::rt`
/// (see its doc comment) — a `len()` TypeError must be catchable by
/// `try`/`except` exactly like a division-by-zero from a binary operator.
fn interp_err(_interp: &Interp, msg: String, _span: Span) -> crate::interp::Flow {
    crate::interp::Flow::Raise(Value::Str(Rc::from(msg.as_str())))
}

/// Dispatches `recv.method(args)` for built-in (non-class-instance) values —
/// list/dict/str methods. Called after class-method lookup fails (or `recv`
/// isn't an Instance/Class at all).
pub fn call_method_builtin(interp: &mut Interp, recv: &Value, name: &str, args: Vec<Value>, span: Span) -> EResult {
    match recv {
        Value::List(l) => list_method(l, name, args, interp, span),
        Value::Dict(d) => dict_method(d, name, args, interp, span),
        Value::Str(s) => str_method(s, name, args, interp, span),
        Value::Tensor(t) => tensor_method(t, name, args, interp, span),
        other => Err(interp_err(interp, format!("'{}' object has no method '{}'", other.type_name(), name), span)),
    }
}

fn list_method(l: &Rc<RefCell<Vec<Value>>>, name: &str, args: Vec<Value>, interp: &mut Interp, span: Span) -> EResult {
    match name {
        "append" => {
            l.borrow_mut().push(args[0].clone());
            Ok(Value::None)
        }
        "pop" => {
            let mut lb = l.borrow_mut();
            let idx = if let Some(i) = args.first() { i.as_f64().unwrap_or(0.0) as i64 } else { lb.len() as i64 - 1 };
            let idx = if idx < 0 { idx + lb.len() as i64 } else { idx };
            if idx < 0 || idx as usize >= lb.len() {
                return Err(interp_err(interp, "pop index out of range".to_string(), span));
            }
            Ok(lb.remove(idx as usize))
        }
        "insert" => {
            let idx = args[0].as_f64().unwrap_or(0.0) as usize;
            l.borrow_mut().insert(idx.min(l.borrow().len()), args[1].clone());
            Ok(Value::None)
        }
        "remove" => {
            let mut lb = l.borrow_mut();
            if let Some(pos) = lb.iter().position(|x| x.eq_val(&args[0])) {
                lb.remove(pos);
                Ok(Value::None)
            } else {
                Err(interp_err(interp, "list.remove(x): x not in list".to_string(), span))
            }
        }
        "extend" => {
            let items = iter_to_vec(&args[0], interp, span)?;
            l.borrow_mut().extend(items);
            Ok(Value::None)
        }
        "clear" => {
            l.borrow_mut().clear();
            Ok(Value::None)
        }
        "reverse" => {
            l.borrow_mut().reverse();
            Ok(Value::None)
        }
        "sort" => {
            l.borrow_mut().sort_by(|a, b| a.as_f64().partial_cmp(&b.as_f64()).unwrap_or(std::cmp::Ordering::Equal));
            Ok(Value::None)
        }
        "index" => l
            .borrow()
            .iter()
            .position(|x| x.eq_val(&args[0]))
            .map(|i| Value::Int(i as i64))
            .ok_or_else(|| interp_err(interp, format!("{} is not in list", args[0].repr()), span)),
        "count" => Ok(Value::Int(l.borrow().iter().filter(|x| x.eq_val(&args[0])).count() as i64)),
        "copy" => Ok(Value::List(Rc::new(RefCell::new(l.borrow().clone())))),
        other => Err(interp_err(interp, format!("'list' object has no attribute '{other}'"), span)),
    }
}

fn dict_method(d: &Rc<RefCell<Vec<(Value, Value)>>>, name: &str, args: Vec<Value>, interp: &mut Interp, span: Span) -> EResult {
    match name {
        "get" => Ok(d.borrow().iter().find(|(k, _)| k.eq_val(&args[0])).map(|(_, v)| v.clone()).unwrap_or_else(|| args.get(1).cloned().unwrap_or(Value::None))),
        "keys" => Ok(Value::List(Rc::new(RefCell::new(d.borrow().iter().map(|(k, _)| k.clone()).collect())))),
        "values" => Ok(Value::List(Rc::new(RefCell::new(d.borrow().iter().map(|(_, v)| v.clone()).collect())))),
        "items" => Ok(Value::List(Rc::new(RefCell::new(d.borrow().iter().map(|(k, v)| Value::Tuple(Rc::new(vec![k.clone(), v.clone()]))).collect())))),
        "pop" => {
            let mut db = d.borrow_mut();
            if let Some(pos) = db.iter().position(|(k, _)| k.eq_val(&args[0])) {
                Ok(db.remove(pos).1)
            } else if let Some(default) = args.get(1) {
                Ok(default.clone())
            } else {
                Err(interp_err(interp, format!("KeyError: {}", args[0].repr()), span))
            }
        }
        "update" => {
            let other = match &args[0] {
                Value::Dict(o) => o.borrow().clone(),
                _ => return Err(interp_err(interp, "update() argument must be a dict".to_string(), span)),
            };
            let mut db = d.borrow_mut();
            for (k, v) in other {
                if let Some(entry) = db.iter_mut().find(|(ek, _)| ek.eq_val(&k)) {
                    entry.1 = v;
                } else {
                    db.push((k, v));
                }
            }
            Ok(Value::None)
        }
        "setdefault" => {
            let mut db = d.borrow_mut();
            if let Some((_, v)) = db.iter().find(|(k, _)| k.eq_val(&args[0])) {
                Ok(v.clone())
            } else {
                let default = args.get(1).cloned().unwrap_or(Value::None);
                db.push((args[0].clone(), default.clone()));
                Ok(default)
            }
        }
        other => Err(interp_err(interp, format!("'dict' object has no attribute '{other}'"), span)),
    }
}

fn str_method(s: &Rc<str>, name: &str, args: Vec<Value>, interp: &mut Interp, span: Span) -> EResult {
    let arg_str = |v: &Value| -> String { v.display() };
    match name {
        "upper" => Ok(Value::Str(Rc::from(s.to_uppercase().as_str()))),
        "lower" => Ok(Value::Str(Rc::from(s.to_lowercase().as_str()))),
        "strip" => Ok(Value::Str(Rc::from(s.trim().to_string().as_str()))),
        "lstrip" => Ok(Value::Str(Rc::from(s.trim_start().to_string().as_str()))),
        "rstrip" => Ok(Value::Str(Rc::from(s.trim_end().to_string().as_str()))),
        "split" => {
            let sep = args.first().map(arg_str);
            let parts: Vec<Value> = match sep {
                Some(sep) if !sep.is_empty() => s.split(sep.as_str()).map(|p| Value::Str(Rc::from(p))).collect(),
                _ => s.split_whitespace().map(|p| Value::Str(Rc::from(p))).collect(),
            };
            Ok(Value::List(Rc::new(RefCell::new(parts))))
        }
        "join" => {
            let items = iter_to_vec(&args[0], interp, span)?;
            Ok(Value::Str(Rc::from(items.iter().map(|v| v.display()).collect::<Vec<_>>().join(s).as_str())))
        }
        "replace" => {
            let (from, to) = (arg_str(&args[0]), arg_str(&args[1]));
            Ok(Value::Str(Rc::from(s.replace(&from, &to).as_str())))
        }
        "startswith" => Ok(Value::Bool(s.starts_with(arg_str(&args[0]).as_str()))),
        "endswith" => Ok(Value::Bool(s.ends_with(arg_str(&args[0]).as_str()))),
        "find" => Ok(Value::Int(s.find(arg_str(&args[0]).as_str()).map(|i| i as i64).unwrap_or(-1))),
        "format" => {
            // Minimal `{}`-style positional formatting.
            let mut out = String::new();
            let mut ai = 0;
            let mut chars = s.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '{' && chars.peek() == Some(&'}') {
                    chars.next();
                    if let Some(a) = args.get(ai) {
                        out.push_str(&a.display());
                    }
                    ai += 1;
                } else {
                    out.push(c);
                }
            }
            Ok(Value::Str(Rc::from(out.as_str())))
        }
        "capitalize" => {
            let mut chars = s.chars();
            let out = match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str().to_lowercase().as_str(),
                None => String::new(),
            };
            Ok(Value::Str(Rc::from(out.as_str())))
        }
        "count" => Ok(Value::Int(s.matches(arg_str(&args[0]).as_str()).count() as i64)),
        other => Err(interp_err(interp, format!("'str' object has no attribute '{other}'"), span)),
    }
}

fn tensor_method(t: &Rc<TensorVal>, name: &str, args: Vec<Value>, interp: &mut Interp, span: Span) -> EResult {
    match name {
        "sum" => Ok(Value::Float(t.data.borrow().iter().sum())),
        "mean" => {
            let d = t.data.borrow();
            Ok(Value::Float(if d.is_empty() { 0.0 } else { d.iter().sum::<f64>() / d.len() as f64 }))
        }
        "get" => {
            let flat = flat_index(&t.shape, &args, interp, span)?;
            Ok(Value::Float(t.data.borrow()[flat]))
        }
        "set" => {
            let val = args.last().and_then(|v| v.as_f64()).unwrap_or(0.0);
            let flat = flat_index(&t.shape, &args[..args.len() - 1], interp, span)?;
            t.data.borrow_mut()[flat] = val;
            Ok(Value::None)
        }
        "add" => tensor_elementwise(t, &args[0], |a, b| a + b, interp, span),
        "mul" => tensor_elementwise(t, &args[0], |a, b| a * b, interp, span),
        "to_list" => Ok(Value::List(Rc::new(RefCell::new(t.data.borrow().iter().map(|&f| Value::Float(f)).collect())))),
        other => Err(interp_err(interp, format!("'Tensor' object has no attribute '{other}'"), span)),
    }
}

fn flat_index(shape: &[usize], idx_args: &[Value], interp: &mut Interp, span: Span) -> Result<usize, crate::interp::Flow> {
    if idx_args.len() != shape.len() {
        return Err(interp_err(interp, format!("expected {} indices, got {}", shape.len(), idx_args.len()), span));
    }
    let mut flat = 0usize;
    let mut stride = 1usize;
    for (dim_i, &dim) in shape.iter().enumerate().rev() {
        let idx = idx_args[dim_i].as_f64().unwrap_or(0.0) as usize;
        flat += idx * stride;
        stride *= dim;
    }
    Ok(flat)
}

fn tensor_elementwise(t: &Rc<TensorVal>, other: &Value, f: impl Fn(f64, f64) -> f64, interp: &mut Interp, span: Span) -> EResult {
    let Value::Tensor(o) = other else {
        return Err(interp_err(interp, "Tensor op requires another Tensor".to_string(), span));
    };
    if t.shape != o.shape {
        return Err(interp_err(interp, format!("shape mismatch: {:?} vs {:?}", t.shape, o.shape), span));
    }
    let data: Vec<f64> = t.data.borrow().iter().zip(o.data.borrow().iter()).map(|(&a, &b)| f(a, b)).collect();
    Ok(Value::Tensor(Rc::new(TensorVal { data: RefCell::new(data), shape: t.shape.clone() })))
}
