//! Builtin standard library — Vec, HashMap, HashSet, String, Option, Result,
//! numeric/char/range methods, and associated constructors (Vec::new, ...).
//! Method calls fall through here when no user `impl` matches.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::diag::Span;
use crate::interp::{EResult, Flow, Interp};
use crate::values::*;

/// Associated (static) functions: Type::fn — returns the constructed value.
pub fn static_builtin(segs: &[String], args: &[Value]) -> Option<Value> {
    let key = segs.join("::");
    let first = || args.first().cloned().unwrap_or(Value::Unit);
    Some(match key.as_str() {
        "Vec::new" | "Vec::with_capacity" => vvec(vec![]),
        "Vec::from" => match args.first() {
            Some(Value::Vec(items)) => vvec(items.borrow().clone()),
            _ => vvec(args.to_vec()),
        },
        "HashMap::new" | "HashMap::with_capacity" | "BTreeMap::new" => Value::Map(Rc::new(RefCell::new(BTreeMap::new()))),
        "HashSet::new" | "BTreeSet::new" => Value::Set(Rc::new(RefCell::new(BTreeMap::new()))),
        "String::new" | "String::with_capacity" => vstr(""),
        "String::from" => vstr(args.first().map(|v| v.display()).unwrap_or_default()),
        "Box::new" | "Rc::new" | "Arc::new" | "RefCell::new" | "Cell::new" | "Mutex::new" | "RwLock::new" => first(),
        // Instants are seconds-since-epoch floats; elapsed()/as_millis() etc.
        // live in num_method.
        "Instant::now" | "SystemTime::now" => {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            Value::Float(secs)
        }
        "Duration::from_millis" => Value::Float(args.first().map(num_of).unwrap_or(0.0) / 1000.0),
        "Duration::from_secs" => Value::Float(args.first().map(num_of).unwrap_or(0.0)),
        _ => {
            // generic wrapper constructors: identity on first arg
            let member = segs.last()?;
            if member == "from" || member == "new" {
                first()
            } else {
                return None;
            }
        }
    })
}

/// Instance methods. `None` = no builtin applies (caller raises a good error).
pub fn call_builtin_method(intr: &mut Interp, recv: &Value, name: &str, args: &[Value], span: Span) -> Option<EResult> {
    // universal
    match name {
        "clone" => return Some(Ok(recv.deep_clone())),
        "to_string" => return Some(Ok(vstr(recv.display()))),
        "eq" => return Some(Ok(Value::Bool(recv.eq_val(args.first()?)))),
        "ne" => return Some(Ok(Value::Bool(!recv.eq_val(args.first()?)))),
        // Mutex/RwLock/RefCell are identity wrappers in the seed; the usual
        // `x.lock().unwrap()` chain therefore needs lock() -> Ok(x).
        "lock" | "read" | "write" => return Some(Ok(ok(recv.clone()))),
        "borrow" | "borrow_mut" | "as_ref" | "as_mut" | "deref" => {
            if !matches!(recv, Value::Enum { .. }) {
                return Some(Ok(recv.clone()));
            }
        }
        _ => {}
    }
    match recv {
        Value::Vec(items) => vec_method(intr, items, name, args, span),
        Value::Map(m) => map_method(m, name, args),
        Value::Set(s) => set_method(s, name, args),
        Value::Str(s) => str_method(s, name, args),
        Value::Int(_) | Value::Float(_) => num_method(recv, name, args),
        Value::Char(c) => char_method(*c, name, args),
        Value::Bool(b) => bool_method(*b, name, args),
        Value::Enum { .. } => enum_method(intr, recv, name, args, span),
        Value::Range { .. } => range_method(intr, recv, name, args, span),
        Value::Struct { name: sname, fields } if &**sname == "__MapEntry" => {
            entry_method(intr, fields, name, args, span)
        }
        _ => None,
    }
}

/// Methods on the `entry(k)` handle: mutate the underlying map in place.
fn entry_method(
    intr: &mut Interp,
    fields: &Rc<RefCell<BTreeMap<String, Value>>>,
    name: &str,
    args: &[Value],
    span: Span,
) -> Option<EResult> {
    let (map, key) = {
        let f = fields.borrow();
        let Some(Value::Map(m)) = f.get("__map").cloned() else { return None };
        let Some(k) = f.get("__key").cloned() else { return None };
        (m, k)
    };
    let kk = key.key();
    let existing = map.borrow().get(&kk).map(|(_, v)| v.clone());
    Some(match name {
        "or_insert" | "or_default" => {
            if let Some(v) = existing {
                Ok(v)
            } else {
                let v = args.first().cloned().unwrap_or(Value::Int(0));
                map.borrow_mut().insert(kk, (key, v.clone()));
                Ok(v)
            }
        }
        "or_insert_with" => {
            if let Some(v) = existing {
                Ok(v)
            } else {
                let f = args.first()?.clone();
                match intr.apply(&f, vec![], span) {
                    Ok(v) => {
                        map.borrow_mut().insert(kk, (key, v.clone()));
                        Ok(v)
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
        }
        "and_modify" => {
            if let Some(v) = existing {
                let f = args.first()?.clone();
                if let Err(e) = intr.apply(&f, vec![v], span) {
                    return Some(Err(e));
                }
            }
            let mut nf = BTreeMap::new();
            nf.insert("__map".to_string(), Value::Map(map));
            nf.insert("__key".to_string(), key);
            Ok(Value::Struct { name: Rc::from("__MapEntry"), fields: Rc::new(RefCell::new(nf)) })
        }
        _ => return None,
    })
}

fn int_of(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        Value::Float(f) => *f as i64,
        _ => 0,
    }
}
fn num_of(v: &Value) -> f64 {
    match v {
        Value::Int(n) => *n as f64,
        Value::Float(f) => *f,
        _ => 0.0,
    }
}
fn cmp_vals(a: &Value, b: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Value::Str(x), Value::Str(y)) => x.borrow().cmp(&y.borrow()),
        (Value::Char(x), Value::Char(y)) => x.cmp(y),
        _ => num_of(a).partial_cmp(&num_of(b)).unwrap_or(Ordering::Equal),
    }
}

fn vec_method(
    intr: &mut Interp,
    items_rc: &Rc<RefCell<Vec<Value>>>,
    name: &str,
    args: &[Value],
    span: Span,
) -> Option<EResult> {
    // Methods that call back into user code snapshot the items first so we
    // never hold a RefCell borrow across an `apply`.
    let snapshot = || items_rc.borrow().clone();
    Some(match name {
        "push" => {
            items_rc.borrow_mut().push(args.first().cloned().unwrap_or(Value::Unit));
            Ok(Value::Unit)
        }
        "pop" => Ok(items_rc.borrow_mut().pop().map(some).unwrap_or_else(none)),
        "len" => Ok(Value::Int(items_rc.borrow().len() as i64)),
        "is_empty" => Ok(Value::Bool(items_rc.borrow().is_empty())),
        "get" => {
            let i = int_of(args.first()?);
            let items = items_rc.borrow();
            Ok(if i >= 0 && (i as usize) < items.len() { some(items[i as usize].clone()) } else { none() })
        }
        "first" | "next" => Ok(items_rc.borrow().first().cloned().map(some).unwrap_or_else(none)),
        "last" => Ok(items_rc.borrow().last().cloned().map(some).unwrap_or_else(none)),
        "contains" => {
            let target = args.first()?;
            Ok(Value::Bool(items_rc.borrow().iter().any(|x| x.eq_val(target))))
        }
        "clear" => {
            items_rc.borrow_mut().clear();
            Ok(Value::Unit)
        }
        "insert" => {
            let i = int_of(args.first()?) as usize;
            let v = args.get(1).cloned().unwrap_or(Value::Unit);
            let mut it = items_rc.borrow_mut();
            let i = i.min(it.len());
            it.insert(i, v);
            Ok(Value::Unit)
        }
        "remove" => {
            let i = int_of(args.first()?) as usize;
            let mut it = items_rc.borrow_mut();
            if i < it.len() { Ok(it.remove(i)) } else { Ok(Value::Unit) }
        }
        "reverse" => {
            items_rc.borrow_mut().reverse();
            Ok(Value::Unit)
        }
        "sort" | "sort_unstable" => {
            items_rc.borrow_mut().sort_by(cmp_vals);
            Ok(Value::Unit)
        }
        "sort_by" => {
            let f = args.first()?.clone();
            let mut snap = snapshot();
            let mut err = None;
            snap.sort_by(|a, b| {
                if err.is_some() {
                    return std::cmp::Ordering::Equal;
                }
                match intr.apply(&f, vec![a.clone(), b.clone()], span) {
                    Ok(Value::Int(n)) => n.cmp(&0),
                    Ok(Value::Enum { variant, .. }) => match &*variant {
                        "Less" => std::cmp::Ordering::Less,
                        "Greater" => std::cmp::Ordering::Greater,
                        _ => std::cmp::Ordering::Equal,
                    },
                    Ok(_) => std::cmp::Ordering::Equal,
                    Err(e) => {
                        err = Some(e);
                        std::cmp::Ordering::Equal
                    }
                }
            });
            if let Some(e) = err {
                return Some(Err(e));
            }
            *items_rc.borrow_mut() = snap;
            Ok(Value::Unit)
        }
        "dedup" => {
            let mut it = items_rc.borrow_mut();
            let mut i = it.len();
            while i > 1 {
                i -= 1;
                if it[i].eq_val(&it[i - 1]) {
                    it.remove(i);
                }
            }
            Ok(Value::Unit)
        }
        "extend_from_slice" => {
            if let Some(Value::Vec(other)) = args.first() {
                let other_items = other.borrow().clone();
                items_rc.borrow_mut().extend(other_items);
            }
            Ok(Value::Unit)
        }
        "extend" | "append" => {
            if let Some(Value::Vec(other)) = args.first() {
                let other_items = other.borrow().clone();
                items_rc.borrow_mut().extend(other_items);
                if name == "append" {
                    other.borrow_mut().clear();
                }
            }
            Ok(Value::Unit)
        }
        "truncate" => {
            let n = int_of(args.first()?) as usize;
            items_rc.borrow_mut().truncate(n);
            Ok(Value::Unit)
        }
        "swap" => {
            let (i, j) = (int_of(args.first()?) as usize, int_of(args.get(1)?) as usize);
            items_rc.borrow_mut().swap(i, j);
            Ok(Value::Unit)
        }
        "iter" | "into_iter" | "iter_mut" | "to_vec" | "collect" | "cloned" | "copied" | "as_slice" => {
            Ok(vvec(snapshot()))
        }
        "map" => {
            let f = args.first()?.clone();
            let mut out = Vec::new();
            for x in snapshot() {
                match intr.apply(&f, vec![x], span) {
                    Ok(v) => out.push(v),
                    Err(e) => return Some(Err(e)),
                }
            }
            Ok(vvec(out))
        }
        "filter" => {
            let f = args.first()?.clone();
            let mut out = Vec::new();
            for x in snapshot() {
                match intr.apply(&f, vec![x.clone()], span) {
                    Ok(v) => {
                        if v.truthy() {
                            out.push(x);
                        }
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
            Ok(vvec(out))
        }
        "filter_map" => {
            let f = args.first()?.clone();
            let mut out = Vec::new();
            for x in snapshot() {
                match intr.apply(&f, vec![x], span) {
                    Ok(Value::Enum { variant, payload, .. }) if &*variant == "Some" => {
                        out.push(payload.borrow().first().cloned().unwrap_or(Value::Unit));
                    }
                    Ok(_) => {}
                    Err(e) => return Some(Err(e)),
                }
            }
            Ok(vvec(out))
        }
        "for_each" => {
            let f = args.first()?.clone();
            for x in snapshot() {
                if let Err(e) = intr.apply(&f, vec![x], span) {
                    return Some(Err(e));
                }
            }
            Ok(Value::Unit)
        }
        "any" => {
            let f = args.first()?.clone();
            for x in snapshot() {
                match intr.apply(&f, vec![x], span) {
                    Ok(v) => {
                        if v.truthy() {
                            return Some(Ok(Value::Bool(true)));
                        }
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
            Ok(Value::Bool(false))
        }
        "all" => {
            let f = args.first()?.clone();
            for x in snapshot() {
                match intr.apply(&f, vec![x], span) {
                    Ok(v) => {
                        if !v.truthy() {
                            return Some(Ok(Value::Bool(false)));
                        }
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
            Ok(Value::Bool(true))
        }
        "find" => {
            let f = args.first()?.clone();
            for x in snapshot() {
                match intr.apply(&f, vec![x.clone()], span) {
                    Ok(v) => {
                        if v.truthy() {
                            return Some(Ok(some(x)));
                        }
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
            Ok(none())
        }
        "position" => {
            let f = args.first()?.clone();
            for (i, x) in snapshot().into_iter().enumerate() {
                match intr.apply(&f, vec![x], span) {
                    Ok(v) => {
                        if v.truthy() {
                            return Some(Ok(some(Value::Int(i as i64))));
                        }
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
            Ok(none())
        }
        "count" => Ok(Value::Int(items_rc.borrow().len() as i64)),
        "sum" => {
            let (mut s, mut is_f) = (0.0, false);
            for x in items_rc.borrow().iter() {
                if matches!(x, Value::Float(_)) {
                    is_f = true;
                }
                s += num_of(x);
            }
            Ok(if is_f { Value::Float(s) } else { Value::Int(s as i64) })
        }
        "product" => {
            let mut s = 1.0;
            for x in items_rc.borrow().iter() {
                s *= num_of(x);
            }
            Ok(Value::Int(s as i64))
        }
        "min" => Ok(items_rc.borrow().iter().min_by(|a, b| cmp_vals(a, b)).cloned().map(some).unwrap_or_else(none)),
        "max" => Ok(items_rc.borrow().iter().max_by(|a, b| cmp_vals(a, b)).cloned().map(some).unwrap_or_else(none)),
        "fold" | "reduce" => {
            let mut acc = args.first()?.clone();
            let f = args.get(1)?.clone();
            for x in snapshot() {
                match intr.apply(&f, vec![acc, x], span) {
                    Ok(v) => acc = v,
                    Err(e) => return Some(Err(e)),
                }
            }
            Ok(acc)
        }
        "enumerate" => Ok(vvec(
            snapshot().into_iter().enumerate().map(|(i, x)| vtuple(vec![Value::Int(i as i64), x])).collect(),
        )),
        "rev" => {
            let mut s = snapshot();
            s.reverse();
            Ok(vvec(s))
        }
        "take" => {
            let n = int_of(args.first()?) as usize;
            Ok(vvec(snapshot().into_iter().take(n).collect()))
        }
        "skip" => {
            let n = int_of(args.first()?) as usize;
            Ok(vvec(snapshot().into_iter().skip(n).collect()))
        }
        "zip" => {
            let Some(Value::Vec(other)) = args.first() else { return Some(Ok(vvec(vec![]))) };
            let o = other.borrow().clone();
            Ok(vvec(snapshot().into_iter().zip(o).map(|(a, b)| vtuple(vec![a, b])).collect()))
        }
        "chain" => {
            let mut s = snapshot();
            if let Some(Value::Vec(other)) = args.first() {
                s.extend(other.borrow().clone());
            }
            Ok(vvec(s))
        }
        "join" => {
            let sep = args.first().map(|v| v.display()).unwrap_or_default();
            Ok(vstr(items_rc.borrow().iter().map(|v| v.display()).collect::<Vec<_>>().join(&sep)))
        }
        "retain" => {
            let f = args.first()?.clone();
            let mut keep = Vec::new();
            for x in snapshot() {
                match intr.apply(&f, vec![x.clone()], span) {
                    Ok(v) => {
                        if v.truthy() {
                            keep.push(x);
                        }
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
            *items_rc.borrow_mut() = keep;
            Ok(Value::Unit)
        }
        _ => return None,
    })
}

fn map_method(m: &Rc<RefCell<BTreeMap<String, (Value, Value)>>>, name: &str, args: &[Value]) -> Option<EResult> {
    Some(match name {
        "insert" => {
            let k = args.first()?.clone();
            let v = args.get(1).cloned().unwrap_or(Value::Unit);
            let prev = m.borrow_mut().insert(k.key(), (k, v));
            Ok(prev.map(|(_, v)| some(v)).unwrap_or_else(none))
        }
        "get" | "get_mut" => {
            let k = args.first()?;
            Ok(m.borrow().get(&k.key()).map(|(_, v)| some(v.clone())).unwrap_or_else(none))
        }
        "entry" => {
            // Live handle so or_insert/or_insert_with/or_default mutate the map.
            let mut fields = BTreeMap::new();
            fields.insert("__map".to_string(), Value::Map(m.clone()));
            fields.insert("__key".to_string(), args.first()?.clone());
            Ok(Value::Struct { name: Rc::from("__MapEntry"), fields: Rc::new(RefCell::new(fields)) })
        }
        "contains_key" => Ok(Value::Bool(m.borrow().contains_key(&args.first()?.key()))),
        "remove" => {
            let k = args.first()?;
            Ok(m.borrow_mut().remove(&k.key()).map(|(_, v)| some(v)).unwrap_or_else(none))
        }
        "len" => Ok(Value::Int(m.borrow().len() as i64)),
        "is_empty" => Ok(Value::Bool(m.borrow().is_empty())),
        "clear" => {
            m.borrow_mut().clear();
            Ok(Value::Unit)
        }
        "keys" => Ok(vvec(m.borrow().values().map(|(k, _)| k.clone()).collect())),
        "values" | "values_mut" => Ok(vvec(m.borrow().values().map(|(_, v)| v.clone()).collect())),
        "keys_mut" | "iter_mut" => Ok(vvec(m.borrow().values().map(|(k, v)| vtuple(vec![k.clone(), v.clone()])).collect())),
        "iter" | "into_iter" => Ok(vvec(m.borrow().values().map(|(k, v)| vtuple(vec![k.clone(), v.clone()])).collect())),
        "get_or" => {
            let k = args.first()?;
            Ok(m.borrow().get(&k.key()).map(|(_, v)| v.clone()).unwrap_or_else(|| args.get(1).cloned().unwrap_or(Value::Unit)))
        }
        _ => return None,
    })
}

fn set_method(s: &Rc<RefCell<BTreeMap<String, Value>>>, name: &str, args: &[Value]) -> Option<EResult> {
    Some(match name {
        "insert" => {
            let v = args.first()?.clone();
            let had = s.borrow_mut().insert(v.key(), v).is_some();
            Ok(Value::Bool(!had))
        }
        "contains" => Ok(Value::Bool(s.borrow().contains_key(&args.first()?.key()))),
        "remove" => Ok(Value::Bool(s.borrow_mut().remove(&args.first()?.key()).is_some())),
        "len" => Ok(Value::Int(s.borrow().len() as i64)),
        "is_empty" => Ok(Value::Bool(s.borrow().is_empty())),
        "clear" => {
            s.borrow_mut().clear();
            Ok(Value::Unit)
        }
        "iter" | "into_iter" => Ok(vvec(s.borrow().values().cloned().collect())),
        _ => return None,
    })
}

fn str_method(s_rc: &Rc<RefCell<String>>, name: &str, args: &[Value]) -> Option<EResult> {
    let s = s_rc.borrow().clone();
    Some(match name {
        "len" => Ok(Value::Int(s.chars().count() as i64)),
        "is_empty" => Ok(Value::Bool(s.is_empty())),
        "push" => {
            let a = args.first()?;
            s_rc.borrow_mut().push_str(&a.display());
            Ok(Value::Unit)
        }
        "push_str" => {
            s_rc.borrow_mut().push_str(&args.first()?.display());
            Ok(Value::Unit)
        }
        "to_uppercase" | "to_ascii_uppercase" => Ok(vstr(s.to_uppercase())),
        "to_lowercase" | "to_ascii_lowercase" => Ok(vstr(s.to_lowercase())),
        "trim" => Ok(vstr(s.trim())),
        "trim_start" => Ok(vstr(s.trim_start())),
        "trim_end" => Ok(vstr(s.trim_end())),
        "contains" => Ok(Value::Bool(s.contains(&args.first()?.display()))),
        "starts_with" => Ok(Value::Bool(s.starts_with(&args.first()?.display()))),
        "ends_with" => Ok(Value::Bool(s.ends_with(&args.first()?.display()))),
        "replace" => Ok(vstr(s.replace(&args.first()?.display(), &args.get(1)?.display()))),
        "split" => {
            let sep = args.first()?.display();
            Ok(vvec(s.split(&sep).map(vstr).collect()))
        }
        "split_whitespace" => Ok(vvec(s.split_whitespace().map(vstr).collect())),
        "lines" => Ok(vvec(s.lines().map(vstr).collect())),
        "chars" => Ok(vvec(s.chars().map(Value::Char).collect())),
        "bytes" => Ok(vvec(s.bytes().map(|b| Value::Int(b as i64)).collect())),
        "char_at" | "nth" => {
            let i = int_of(args.first()?) as usize;
            Ok(s.chars().nth(i).map(|c| some(Value::Char(c))).unwrap_or_else(none))
        }
        "find" => {
            let needle = args.first()?.display();
            Ok(s.find(&needle).map(|i| some(Value::Int(i as i64))).unwrap_or_else(none))
        }
        "repeat" => Ok(vstr(s.repeat(int_of(args.first()?) as usize))),
        "as_str" | "trim_matches" => Ok(vstr(s)),
        "strip_prefix" => {
            let p = args.first()?.display();
            Ok(s.strip_prefix(&p).map(|r| some(vstr(r))).unwrap_or_else(none))
        }
        "strip_suffix" => {
            let p = args.first()?.display();
            Ok(s.strip_suffix(&p).map(|r| some(vstr(r))).unwrap_or_else(none))
        }
        "parse" => {
            if s.contains('.') {
                match s.trim().parse::<f64>() {
                    Ok(f) => Ok(ok(Value::Float(f))),
                    Err(_) => Ok(err(vstr("invalid number"))),
                }
            } else {
                match s.trim().parse::<i64>() {
                    Ok(n) => Ok(ok(Value::Int(n))),
                    Err(_) => Ok(err(vstr("invalid number"))),
                }
            }
        }
        "to_int" => Ok(s.trim().parse::<i64>().map(|n| some(Value::Int(n))).unwrap_or_else(|_| none())),
        "substring" | "slice" => {
            let a = int_of(args.first()?) as usize;
            let b = args.get(1).map(|v| int_of(v) as usize).unwrap_or(s.chars().count());
            Ok(vstr(s.chars().skip(a).take(b.saturating_sub(a)).collect::<String>()))
        }
        "reverse" => Ok(vstr(s.chars().rev().collect::<String>())),
        "count" => Ok(Value::Int(s.chars().count() as i64)),
        _ => return None,
    })
}

fn num_method(recv: &Value, name: &str, args: &[Value]) -> Option<EResult> {
    let n = num_of(recv);
    let is_int = matches!(recv, Value::Int(_));
    let wrap = |x: f64| if is_int { Value::Int(x as i64) } else { Value::Float(x) };
    Some(match name {
        "abs" => Ok(wrap(n.abs())),
        "pow" | "powi" => Ok(wrap(n.powf(num_of(args.first()?)))),
        "powf" => Ok(Value::Float(n.powf(num_of(args.first()?)))),
        "sqrt" => Ok(Value::Float(n.sqrt())),
        "min" => Ok(wrap(n.min(num_of(args.first()?)))),
        "max" => Ok(wrap(n.max(num_of(args.first()?)))),
        "floor" => Ok(Value::Float(n.floor())),
        "ceil" => Ok(Value::Float(n.ceil())),
        "round" => Ok(Value::Float(n.round())),
        "is_positive" => Ok(Value::Bool(n > 0.0)),
        "is_negative" => Ok(Value::Bool(n < 0.0)),
        "is_even" => Ok(Value::Bool((n as i64) % 2 == 0)),
        "is_odd" => Ok(Value::Bool((n as i64) % 2 != 0)),
        "signum" => Ok(wrap(if n > 0.0 { 1.0 } else if n < 0.0 { -1.0 } else { 0.0 })),
        "to_f64" | "as_f64" => Ok(Value::Float(n)),
        "to_i64" | "as_i64" | "trunc" => Ok(Value::Int(n as i64)),
        "checked_add" => Ok(some(wrap(n + num_of(args.first()?)))),
        "saturating_sub" => Ok(wrap((n - num_of(args.first()?)).max(0.0))),
        "wrapping_mul" => {
            let b = int_of(args.first()?);
            Ok(Value::Int((n as i64).wrapping_mul(b)))
        }
        "wrapping_add" => {
            let b = int_of(args.first()?);
            Ok(Value::Int((n as i64).wrapping_add(b)))
        }
        "count_ones" => Ok(Value::Int((n as i64).count_ones() as i64)),
        // Instant/Duration methods (instants are secs-since-epoch floats)
        "elapsed" => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            Ok(Value::Float((now - n).max(0.0)))
        }
        "as_millis" => Ok(Value::Int((n * 1000.0) as i64)),
        "as_micros" => Ok(Value::Int((n * 1e6) as i64)),
        "as_nanos" => Ok(Value::Int((n * 1e9) as i64)),
        "as_secs" => Ok(Value::Int(n as i64)),
        "as_secs_f64" => Ok(Value::Float(n)),
        _ => return None,
    })
}

fn char_method(c: char, name: &str, args: &[Value]) -> Option<EResult> {
    Some(match name {
        "is_alphabetic" => Ok(Value::Bool(c.is_alphabetic())),
        "is_numeric" | "is_ascii_digit" | "is_digit" => Ok(Value::Bool(c.is_ascii_digit())),
        "is_alphanumeric" => Ok(Value::Bool(c.is_alphanumeric())),
        "is_whitespace" => Ok(Value::Bool(c.is_whitespace())),
        "is_uppercase" => Ok(Value::Bool(c.is_uppercase())),
        "is_lowercase" => Ok(Value::Bool(c.is_lowercase())),
        "to_uppercase" | "to_ascii_uppercase" => Ok(Value::Char(c.to_ascii_uppercase())),
        "to_lowercase" | "to_ascii_lowercase" => Ok(Value::Char(c.to_ascii_lowercase())),
        "to_digit" => {
            let radix = args.first().map(|v| int_of(v) as u32).unwrap_or(10);
            Ok(c.to_digit(radix).map(|d| some(Value::Int(d as i64))).unwrap_or_else(none))
        }
        "as_u32" | "to_int" => Ok(Value::Int(c as u32 as i64)),
        _ => return None,
    })
}

fn bool_method(b: bool, name: &str, args: &[Value]) -> Option<EResult> {
    Some(match name {
        "then" => Ok(if b { some(args.first().cloned().unwrap_or(Value::Unit)) } else { none() }),
        _ => return None,
    })
}

fn enum_method(intr: &mut Interp, recv: &Value, name: &str, args: &[Value], span: Span) -> Option<EResult> {
    let Value::Enum { enum_name, variant, payload } = recv else { return None };
    let is_opt = &**enum_name == "Option";
    let is_res = &**enum_name == "Result";
    if !is_opt && !is_res {
        return match name {
            "variant_name" => Some(Ok(vstr(&**variant))),
            _ => None,
        };
    }
    let ok_variant = if is_opt { "Some" } else { "Ok" };
    let is_ok = &**variant == ok_variant;
    let inner = payload.borrow().first().cloned().unwrap_or(Value::Unit);
    Some(match name {
        "is_some" => Ok(Value::Bool(&**variant == "Some")),
        "is_none" => Ok(Value::Bool(&**variant == "None")),
        "is_ok" => Ok(Value::Bool(&**variant == "Ok")),
        "is_err" => Ok(Value::Bool(&**variant == "Err")),
        "unwrap" => {
            if is_ok {
                Ok(inner)
            } else {
                let extra = if &**variant == "Err" { format!(": {}", inner.debug()) } else { String::new() };
                Err(intr.rt(format!("called `{enum_name}::unwrap()` on a `{variant}` value{extra}"), span))
            }
        }
        "expect" => {
            if is_ok {
                Ok(inner)
            } else {
                Err(intr.rt(args.first().map(|v| v.display()).unwrap_or_default(), span))
            }
        }
        "unwrap_or" => Ok(if is_ok { inner } else { args.first().cloned().unwrap_or(Value::Unit) }),
        "unwrap_or_else" => {
            if is_ok {
                Ok(inner)
            } else {
                let f = args.first()?.clone();
                let call_args = if is_res { vec![inner] } else { vec![] };
                match intr.apply(&f, call_args, span) {
                    Ok(v) => Ok(v),
                    Err(e) => return Some(Err(e)),
                }
            }
        }
        "unwrap_or_default" => Ok(if is_ok { inner } else { Value::Int(0) }),
        "unwrap_err" => {
            if matches!(&**variant, "Err" | "None") {
                Ok(inner)
            } else {
                Err(intr.rt(format!("called unwrap_err on {variant}"), span))
            }
        }
        "map" => {
            if is_ok {
                let f = args.first()?.clone();
                match intr.apply(&f, vec![inner], span) {
                    Ok(v) => Ok(if is_opt { some(v) } else { ok(v) }),
                    Err(e) => return Some(Err(e)),
                }
            } else {
                Ok(recv.clone())
            }
        }
        "map_or" => {
            if is_ok {
                let f = args.get(1)?.clone();
                match intr.apply(&f, vec![inner], span) {
                    Ok(v) => Ok(v),
                    Err(e) => return Some(Err(e)),
                }
            } else {
                Ok(args.first().cloned().unwrap_or(Value::Unit))
            }
        }
        "and_then" => {
            if is_ok {
                let f = args.first()?.clone();
                match intr.apply(&f, vec![inner], span) {
                    Ok(v) => Ok(v),
                    Err(e) => return Some(Err(e)),
                }
            } else {
                Ok(recv.clone())
            }
        }
        "or_else" => {
            if is_ok {
                Ok(recv.clone())
            } else {
                let f = args.first()?.clone();
                match intr.apply(&f, vec![], span) {
                    Ok(v) => Ok(v),
                    Err(e) => return Some(Err(e)),
                }
            }
        }
        "or" => Ok(if is_ok { recv.clone() } else { args.first().cloned().unwrap_or(Value::Unit) }),
        "ok" => Ok(match &**variant {
            "Ok" => some(inner),
            "Some" => recv.clone(),
            _ => none(),
        }),
        "ok_or" => Ok(if &**variant == "Some" { ok(inner) } else { err(args.first().cloned().unwrap_or(Value::Unit)) }),
        "filter" => {
            if &**variant == "Some" {
                let f = args.first()?.clone();
                match intr.apply(&f, vec![inner], span) {
                    Ok(v) => Ok(if v.truthy() { recv.clone() } else { none() }),
                    Err(e) => return Some(Err(e)),
                }
            } else {
                Ok(none())
            }
        }
        "take" | "as_ref" | "as_mut" | "cloned" | "copied" => Ok(recv.clone()),
        "contains" => Ok(Value::Bool(is_ok && inner.eq_val(args.first()?))),
        _ => return None,
    })
}

fn range_method(intr: &mut Interp, recv: &Value, name: &str, args: &[Value], span: Span) -> Option<EResult> {
    let items = match intr.iter_values(recv, span) {
        Ok(v) => v,
        Err(e) => return Some(Err(e)),
    };
    let Value::Range { from, to, inclusive } = recv else { return None };
    Some(match name {
        "collect" | "iter" | "into_iter" => Ok(vvec(items)),
        "rev" => {
            let mut it = items;
            it.reverse();
            Ok(vvec(it))
        }
        "map" => {
            let f = args.first()?.clone();
            let mut out = Vec::new();
            for x in items {
                match intr.apply(&f, vec![x], span) {
                    Ok(v) => out.push(v),
                    Err(e) => return Some(Err(e)),
                }
            }
            Ok(vvec(out))
        }
        "filter" => {
            let f = args.first()?.clone();
            let mut out = Vec::new();
            for x in items {
                match intr.apply(&f, vec![x.clone()], span) {
                    Ok(v) => {
                        if v.truthy() {
                            out.push(x);
                        }
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
            Ok(vvec(out))
        }
        "sum" => Ok(Value::Int(items.iter().map(int_of).sum())),
        "count" | "len" => Ok(Value::Int(items.len() as i64)),
        "contains" => {
            let v = int_of(args.first()?);
            Ok(Value::Bool(v >= *from && if *inclusive { v <= *to } else { v < *to }))
        }
        "for_each" => {
            let f = args.first()?.clone();
            for x in items {
                if let Err(e) = intr.apply(&f, vec![x], span) {
                    return Some(Err(e));
                }
            }
            Ok(Value::Unit)
        }
        "fold" => {
            let mut acc = args.first()?.clone();
            let f = args.get(1)?.clone();
            for x in items {
                match intr.apply(&f, vec![acc, x], span) {
                    Ok(v) => acc = v,
                    Err(e) => return Some(Err(e)),
                }
            }
            Ok(acc)
        }
        _ => return None,
    })
}
