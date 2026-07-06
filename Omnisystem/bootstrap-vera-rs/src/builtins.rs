//! Vera builtins — `mount`/`render` are the two that make this bootstrap's
//! "no real browser/GPU" honesty work: `mount` instantiates a component
//! (props + initial state + computed), `render` evaluates its markup tree
//! into a real `Element` value that `puts` can serialize to text.

use crate::diag::Span;
use crate::interp::{EResult, Flow, Interp};
use crate::values::Value;

pub fn install_globals(interp: &Interp) {
    for n in ["puts", "print", "mount", "render", "len"] {
        interp.globals.declare(n, Value::Builtin(n));
    }
}

fn err(msg: impl Into<String>, span: Span) -> Flow {
    Flow::Error(Box::new(crate::diag::OmniError::new(crate::diag::Phase::Runtime, msg, span, "")))
}

pub fn call_builtin(interp: &mut Interp, name: &str, mut args: Vec<Value>, span: Span) -> EResult {
    match name {
        "puts" | "print" => {
            let s = args.iter().map(|a| a.display()).collect::<Vec<_>>().join(" ");
            interp.print(&s);
            interp.print("\n");
            Ok(Value::Unit)
        }
        "mount" => {
            if args.is_empty() {
                return Err(err("mount/N requires a component as its first argument", span));
            }
            let comp = args.remove(0);
            let Value::ComponentClass(_) = &comp else {
                return Err(err(format!("mount/N's first argument must be a component, got {}", comp.type_name()), span));
            };
            interp.call_value(&comp, args, span)
        }
        "render" => {
            let Value::ComponentInstance(inst) = &args[0] else {
                return Err(err(format!("render/1 expects a component instance, got {}", args[0].type_name()), span));
            };
            interp.render_public(&inst.clone(), span)
        }
        "len" => match &args[0] {
            Value::List(l) => Ok(Value::Int(l.borrow().len() as i64)),
            Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
            other => Err(err(format!("len/1 expects a list or string, got {}", other.type_name()), span)),
        },
        other => Err(err(format!("unknown builtin '{other}'"), span)),
    }
}

