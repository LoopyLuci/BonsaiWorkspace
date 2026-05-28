//! Sylva standard library — built-in functions registered in the VM.

use std::sync::Arc;
use crate::vm::{SylvaVm, SylvaValue, VmError, VmResult};

pub fn register_stdlib(vm: &mut SylvaVm) {
    // ── I/O ───────────────────────────────────────────────────────────────────
    native(vm, "print", |args| {
        let s = args.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ");
        println!("{s}");
        Ok(SylvaValue::Nil)
    });

    native(vm, "println", |args| {
        let s = args.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ");
        println!("{s}");
        Ok(SylvaValue::Nil)
    });

    native(vm, "to_string", |args| {
        let s = args.first().map(|v| v.to_string()).unwrap_or_default();
        Ok(SylvaValue::Str(s))
    });

    native(vm, "to_int", |args| {
        match args.first() {
            Some(SylvaValue::Int(n)) => Ok(SylvaValue::Int(*n)),
            Some(SylvaValue::Float(f)) => Ok(SylvaValue::Int(*f as i64)),
            Some(SylvaValue::Str(s)) => s.trim().parse::<i64>()
                .map(SylvaValue::Int)
                .map_err(|_| VmError::Runtime(format!("cannot convert {s:?} to Int"))),
            other => Err(VmError::TypeError(format!("to_int: unexpected {:?}", other)))
        }
    });

    native(vm, "to_float", |args| {
        match args.first() {
            Some(SylvaValue::Float(f)) => Ok(SylvaValue::Float(*f)),
            Some(SylvaValue::Int(n))   => Ok(SylvaValue::Float(*n as f64)),
            Some(SylvaValue::Str(s))   => s.trim().parse::<f64>()
                .map(SylvaValue::Float)
                .map_err(|_| VmError::Runtime(format!("cannot convert {s:?} to Float"))),
            other => Err(VmError::TypeError(format!("to_float: unexpected {:?}", other)))
        }
    });

    // ── Strings ───────────────────────────────────────────────────────────────
    native(vm, "len", |args| {
        match args.first() {
            Some(SylvaValue::Str(s))  => Ok(SylvaValue::Int(s.len() as i64)),
            Some(SylvaValue::List(l)) => Ok(SylvaValue::Int(l.len() as i64)),
            Some(SylvaValue::Map(m))  => Ok(SylvaValue::Int(m.len() as i64)),
            Some(SylvaValue::Tuple(t)) => Ok(SylvaValue::Int(t.len() as i64)),
            other => Err(VmError::TypeError(format!("len: unexpected {:?}", other)))
        }
    });

    native(vm, "split", |args| {
        let (s, sep) = match (args.get(0), args.get(1)) {
            (Some(SylvaValue::Str(s)), Some(SylvaValue::Str(sep))) => (s.clone(), sep.clone()),
            _ => return Err(VmError::TypeError("split(str, sep)".into()))
        };
        Ok(SylvaValue::List(s.split(sep.as_str()).map(|p| SylvaValue::Str(p.into())).collect()))
    });

    native(vm, "join", |args| {
        match (args.get(0), args.get(1)) {
            (Some(SylvaValue::List(l)), Some(SylvaValue::Str(sep))) => {
                let s = l.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(sep.as_str());
                Ok(SylvaValue::Str(s))
            }
            _ => Err(VmError::TypeError("join(list, sep)".into()))
        }
    });

    native(vm, "trim",    |args| str_method(args, |s| s.trim().into()));
    native(vm, "upper",   |args| str_method(args, |s| s.to_uppercase()));
    native(vm, "lower",   |args| str_method(args, |s| s.to_lowercase()));
    native(vm, "starts_with", |args| {
        match (args.get(0), args.get(1)) {
            (Some(SylvaValue::Str(s)), Some(SylvaValue::Str(p))) =>
                Ok(SylvaValue::Bool(s.starts_with(p.as_str()))),
            _ => Err(VmError::TypeError("starts_with(str, prefix)".into()))
        }
    });
    native(vm, "ends_with", |args| {
        match (args.get(0), args.get(1)) {
            (Some(SylvaValue::Str(s)), Some(SylvaValue::Str(p))) =>
                Ok(SylvaValue::Bool(s.ends_with(p.as_str()))),
            _ => Err(VmError::TypeError("ends_with(str, suffix)".into()))
        }
    });
    native(vm, "contains", |args| {
        match (args.get(0), args.get(1)) {
            (Some(SylvaValue::Str(s)), Some(SylvaValue::Str(p))) =>
                Ok(SylvaValue::Bool(s.contains(p.as_str()))),
            (Some(SylvaValue::List(l)), val) => {
                if let Some(val) = val { Ok(SylvaValue::Bool(l.contains(val))) }
                else { Err(VmError::TypeError("contains: missing argument".into())) }
            }
            _ => Err(VmError::TypeError("contains(str/list, needle)".into()))
        }
    });
    native(vm, "replace", |args| {
        match (args.get(0), args.get(1), args.get(2)) {
            (Some(SylvaValue::Str(s)), Some(SylvaValue::Str(from)), Some(SylvaValue::Str(to))) =>
                Ok(SylvaValue::Str(s.replace(from.as_str(), to.as_str()))),
            _ => Err(VmError::TypeError("replace(str, from, to)".into()))
        }
    });

    // ── Lists ─────────────────────────────────────────────────────────────────
    native(vm, "push", |mut args| {
        if args.len() < 2 { return Err(VmError::TypeError("push(list, item)".into())); }
        let item = args.remove(1);
        match args.remove(0) {
            SylvaValue::List(mut l) => { l.push(item); Ok(SylvaValue::List(l)) }
            other => Err(VmError::TypeError(format!("push: expected list, got {}", other)))
        }
    });

    native(vm, "pop", |args| {
        match args.into_iter().next() {
            Some(SylvaValue::List(mut l)) => {
                let v = l.pop().unwrap_or(SylvaValue::Nil);
                Ok(SylvaValue::Tuple(vec![SylvaValue::List(l), v]))
            }
            _ => Err(VmError::TypeError("pop(list)".into()))
        }
    });

    native(vm, "reverse", |args| {
        match args.into_iter().next() {
            Some(SylvaValue::List(mut l)) => { l.reverse(); Ok(SylvaValue::List(l)) }
            _ => Err(VmError::TypeError("reverse(list)".into()))
        }
    });

    native(vm, "range", |args| {
        match (args.get(0), args.get(1)) {
            (Some(SylvaValue::Int(start)), Some(SylvaValue::Int(end))) => {
                Ok(SylvaValue::List((*start..*end).map(SylvaValue::Int).collect()))
            }
            (Some(SylvaValue::Int(end)), None) => {
                Ok(SylvaValue::List((0..*end).map(SylvaValue::Int).collect()))
            }
            _ => Err(VmError::TypeError("range(end) or range(start, end)".into()))
        }
    });

    native(vm, "map", |args| {
        Err(VmError::Runtime("map(list, fn) requires calling the VM recursively — use for loop instead".into()))
    });

    native(vm, "filter", |args| {
        Err(VmError::Runtime("filter(list, fn) requires calling the VM recursively — use for loop instead".into()))
    });

    native(vm, "zip", |args| {
        match (args.get(0), args.get(1)) {
            (Some(SylvaValue::List(a)), Some(SylvaValue::List(b))) => {
                let zipped = a.iter().zip(b.iter())
                    .map(|(x, y)| SylvaValue::Tuple(vec![x.clone(), y.clone()]))
                    .collect();
                Ok(SylvaValue::List(zipped))
            }
            _ => Err(VmError::TypeError("zip(list, list)".into()))
        }
    });

    // ── Math ──────────────────────────────────────────────────────────────────
    native(vm, "abs", |args| {
        match args.first() {
            Some(SylvaValue::Int(n))   => Ok(SylvaValue::Int(n.abs())),
            Some(SylvaValue::Float(f)) => Ok(SylvaValue::Float(f.abs())),
            _ => Err(VmError::TypeError("abs(num)".into()))
        }
    });
    native(vm, "min", |args| {
        match (args.get(0), args.get(1)) {
            (Some(SylvaValue::Int(a)), Some(SylvaValue::Int(b))) => Ok(SylvaValue::Int(*a.min(b))),
            (Some(SylvaValue::Float(a)), Some(SylvaValue::Float(b))) => Ok(SylvaValue::Float(a.min(*b))),
            _ => Err(VmError::TypeError("min(a, b)".into()))
        }
    });
    native(vm, "max", |args| {
        match (args.get(0), args.get(1)) {
            (Some(SylvaValue::Int(a)), Some(SylvaValue::Int(b))) => Ok(SylvaValue::Int(*a.max(b))),
            (Some(SylvaValue::Float(a)), Some(SylvaValue::Float(b))) => Ok(SylvaValue::Float(a.max(*b))),
            _ => Err(VmError::TypeError("max(a, b)".into()))
        }
    });
    native(vm, "sqrt",  |args| float_fn(args, f64::sqrt));
    native(vm, "floor", |args| float_fn(args, f64::floor));
    native(vm, "ceil",  |args| float_fn(args, f64::ceil));
    native(vm, "round", |args| float_fn(args, f64::round));

    // ── JSON ──────────────────────────────────────────────────────────────────
    native(vm, "to_json", |args| {
        let s = args.first().map(|v| v.to_json().to_string()).unwrap_or_default();
        Ok(SylvaValue::Str(s))
    });
    native(vm, "from_json", |args| {
        match args.first() {
            Some(SylvaValue::Str(s)) => {
                let v: serde_json::Value = serde_json::from_str(s)
                    .map_err(|e| VmError::Runtime(format!("from_json: {e}")))?;
                Ok(SylvaValue::from_json(v))
            }
            _ => Err(VmError::TypeError("from_json(str)".into()))
        }
    });

    // ── Type checks ───────────────────────────────────────────────────────────
    native(vm, "is_nil",   |args| Ok(SylvaValue::Bool(matches!(args.first(), Some(SylvaValue::Nil)))));
    native(vm, "is_bool",  |args| Ok(SylvaValue::Bool(matches!(args.first(), Some(SylvaValue::Bool(_))))));
    native(vm, "is_int",   |args| Ok(SylvaValue::Bool(matches!(args.first(), Some(SylvaValue::Int(_))))));
    native(vm, "is_float", |args| Ok(SylvaValue::Bool(matches!(args.first(), Some(SylvaValue::Float(_))))));
    native(vm, "is_str",   |args| Ok(SylvaValue::Bool(matches!(args.first(), Some(SylvaValue::Str(_))))));
    native(vm, "is_list",  |args| Ok(SylvaValue::Bool(matches!(args.first(), Some(SylvaValue::List(_))))));
    native(vm, "is_fn",    |args| Ok(SylvaValue::Bool(matches!(args.first(), Some(SylvaValue::Closure(_) | SylvaValue::Native(_))))));

    // ── Map operations ────────────────────────────────────────────────────────
    native(vm, "keys", |args| {
        match args.first() {
            Some(SylvaValue::Map(m)) => Ok(SylvaValue::List(m.iter().map(|(k, _)| k.clone()).collect())),
            _ => Err(VmError::TypeError("keys(map)".into()))
        }
    });
    native(vm, "values", |args| {
        match args.first() {
            Some(SylvaValue::Map(m)) => Ok(SylvaValue::List(m.iter().map(|(_, v)| v.clone()).collect())),
            _ => Err(VmError::TypeError("values(map)".into()))
        }
    });
    native(vm, "has_key", |args| {
        match (args.get(0), args.get(1)) {
            (Some(SylvaValue::Map(m)), Some(key)) =>
                Ok(SylvaValue::Bool(m.iter().any(|(k, _)| k == key))),
            _ => Err(VmError::TypeError("has_key(map, key)".into()))
        }
    });
}

fn native<F>(vm: &mut SylvaVm, name: &str, f: F)
where F: Fn(Vec<SylvaValue>) -> VmResult<SylvaValue> + Send + Sync + 'static
{
    vm.env.set_global(name.into(), SylvaValue::Native(Arc::new(f)));
}

fn str_method<F>(args: Vec<SylvaValue>, f: F) -> VmResult<SylvaValue>
where F: Fn(&str) -> String
{
    match args.first() {
        Some(SylvaValue::Str(s)) => Ok(SylvaValue::Str(f(s))),
        _ => Err(VmError::TypeError("expected string".into()))
    }
}

fn float_fn<F>(args: Vec<SylvaValue>, f: F) -> VmResult<SylvaValue>
where F: Fn(f64) -> f64
{
    match args.first() {
        Some(SylvaValue::Float(v)) => Ok(SylvaValue::Float(f(*v))),
        Some(SylvaValue::Int(n))   => Ok(SylvaValue::Float(f(*n as f64))),
        _ => Err(VmError::TypeError("expected number".into()))
    }
}
