//! Helix builtins — vecN constructors, real vector math (dot/cross/
//! normalize/length/mix/clamp — numerically verified, not stubbed), scalar
//! math, and `buffer(...)` to build the shared mutable arrays `dispatch`/
//! `run_stage` operate over.

use std::cell::RefCell;
use std::rc::Rc;

use crate::diag::Span;
use crate::interp::{EResult, Flow, Interp};
use crate::values::Value;

fn err(msg: impl Into<String>, span: Span) -> Flow {
    Flow::Error(Box::new(crate::diag::OmniError::new(crate::diag::Phase::Runtime, msg, span, "")))
}

fn as_vec(v: &Value, span: Span) -> Result<Rc<RefCell<Vec<f64>>>, Flow> {
    match v {
        Value::Vec(x) => Ok(x.clone()),
        other => Err(err(format!("expected a vector, got {}", other.type_name()), span)),
    }
}
fn as_num(v: &Value, span: Span) -> Result<f64, Flow> {
    v.as_f64().ok_or_else(|| err(format!("expected a number, got {}", v.type_name()), span))
}

pub fn call_builtin(_interp: &mut Interp, name: &str, args: Vec<Value>, span: Span) -> EResult {
    match name {
        "puts" | "print" => {
            let s = args.iter().map(|a| a.display()).collect::<Vec<_>>().join(" ");
            _interp.print(&s);
            Ok(Value::Unit)
        }
        "vec2" | "vec3" | "vec4" => {
            let n = match name {
                "vec2" => 2,
                "vec3" => 3,
                _ => 4,
            };
            // Real GLSL-style construction: scalars and smaller vectors can
            // be mixed and concatenated (`vec4(v3, 1.0)`), not just N loose
            // scalars.
            let mut comps = Vec::with_capacity(n);
            for a in &args {
                match a {
                    Value::Vec(v) => comps.extend(v.borrow().iter().copied()),
                    other => comps.push(as_num(other, span)?),
                }
            }
            if comps.len() == 1 {
                let f = comps[0];
                comps = vec![f; n];
            }
            if comps.len() != n {
                return Err(err(format!("{name}(...) needs {n} components (or a single scalar to splat), got {}", comps.len()), span));
            }
            Ok(Value::Vec(Rc::new(RefCell::new(comps))))
        }
        "buffer" => {
            let items: Vec<Value> = args;
            Ok(Value::Buffer(Rc::new(RefCell::new(items))))
        }
        "dot" => {
            let (a, b) = (as_vec(&args[0], span)?, as_vec(&args[1], span)?);
            let (a, b) = (a.borrow(), b.borrow());
            if a.len() != b.len() {
                return Err(err("dot() requires equal-length vectors", span));
            }
            Ok(Value::Float(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()))
        }
        "cross" => {
            let (a, b) = (as_vec(&args[0], span)?, as_vec(&args[1], span)?);
            let (a, b) = (a.borrow(), b.borrow());
            if a.len() != 3 || b.len() != 3 {
                return Err(err("cross() requires two vec3s", span));
            }
            Ok(Value::Vec(Rc::new(RefCell::new(vec![a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]))))
        }
        "length" => {
            let a = as_vec(&args[0], span)?;
            let ab = a.borrow();
            let len = ab.iter().map(|x| x * x).sum::<f64>().sqrt();
            Ok(Value::Float(len))
        }
        "normalize" => {
            let a = as_vec(&args[0], span)?;
            let ab = a.borrow();
            let len = ab.iter().map(|x| x * x).sum::<f64>().sqrt();
            if len == 0.0 {
                return Err(err("normalize() of the zero vector is undefined", span));
            }
            let out: std::vec::Vec<f64> = ab.iter().map(|x| x / len).collect();
            Ok(Value::Vec(Rc::new(RefCell::new(out))))
        }
        "mix" => {
            let t = as_num(&args[2], span)?;
            match (&args[0], &args[1]) {
                (Value::Vec(a), Value::Vec(b)) => {
                    let (a, b) = (a.borrow(), b.borrow());
                    if a.len() != b.len() {
                        return Err(err("mix() requires equal-length vectors", span));
                    }
                    Ok(Value::Vec(Rc::new(RefCell::new(a.iter().zip(b.iter()).map(|(x, y)| x + (y - x) * t).collect()))))
                }
                _ => {
                    let (a, b) = (as_num(&args[0], span)?, as_num(&args[1], span)?);
                    Ok(Value::Float(a + (b - a) * t))
                }
            }
        }
        "clamp" => {
            let (lo, hi) = (as_num(&args[1], span)?, as_num(&args[2], span)?);
            match &args[0] {
                Value::Vec(v) => Ok(Value::Vec(Rc::new(RefCell::new(v.borrow().iter().map(|x| x.clamp(lo, hi)).collect())))),
                other => Ok(Value::Float(as_num(other, span)?.clamp(lo, hi))),
            }
        }
        "min" => Ok(Value::Float(as_num(&args[0], span)?.min(as_num(&args[1], span)?))),
        "max" => Ok(Value::Float(as_num(&args[0], span)?.max(as_num(&args[1], span)?))),
        "abs" => match &args[0] {
            Value::Int(n) => Ok(Value::Int(n.abs())),
            other => Ok(Value::Float(as_num(other, span)?.abs())),
        },
        "floor" => Ok(Value::Float(as_num(&args[0], span)?.floor())),
        "fract" => {
            let v = as_num(&args[0], span)?;
            Ok(Value::Float(v - v.floor()))
        }
        "pow" => Ok(Value::Float(as_num(&args[0], span)?.powf(as_num(&args[1], span)?))),
        "sqrt" => {
            let v = as_num(&args[0], span)?;
            if v < 0.0 {
                return Err(err("sqrt() of a negative number", span));
            }
            Ok(Value::Float(v.sqrt()))
        }
        other => Err(err(format!("unknown builtin '{other}'"), span)),
    }
}
