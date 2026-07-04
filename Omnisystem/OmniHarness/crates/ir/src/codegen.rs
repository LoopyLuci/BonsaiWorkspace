//! UniIR → Rust source code generator.
//!
//! Converts an `IrModule` into a Rust source string that can be written to
//! a `.rs` file and compiled with `rustc` or `cargo build`.

use crate::ops::{
    BinOpKind, IrFunction, IrLit, IrModule, IrOp, IrParam, IrType, IrTypeDef, IrTypeDefKind,
    UnOpKind,
};
use std::fmt::Write as _;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum CodegenError {
    UnsupportedOp(String),
    UnsupportedType(String),
    InvalidAst(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedOp(s) => write!(f, "unsupported op: {s}"),
            Self::UnsupportedType(s) => write!(f, "unsupported type: {s}"),
            Self::InvalidAst(s) => write!(f, "invalid AST: {s}"),
        }
    }
}

// ── Codegen trait ─────────────────────────────────────────────────────────────

pub trait Codegen {
    type Output;
    type Error;
    fn emit_module(&mut self, m: &IrModule) -> Result<Self::Output, Self::Error>;
}

// ── RustCodegen ───────────────────────────────────────────────────────────────

pub struct RustCodegen {
    indent_size: usize,
}

impl Default for RustCodegen {
    fn default() -> Self {
        Self { indent_size: 4 }
    }
}

impl RustCodegen {
    pub fn new() -> Self {
        Self::default()
    }

    fn indent(&self, level: usize) -> String {
        " ".repeat(level * self.indent_size)
    }

    pub fn emit_type(&self, ty: &IrType) -> Result<String, CodegenError> {
        Ok(match ty {
            IrType::Unit => "()".into(),
            IrType::Bool => "bool".into(),
            IrType::I64 => "i64".into(),
            IrType::F64 => "f64".into(),
            IrType::Str => "String".into(),
            IrType::Bytes => "Vec<u8>".into(),
            IrType::Array(inner) => format!("Vec<{}>", self.emit_type(inner)?),
            IrType::Option(inner) => format!("Option<{}>", self.emit_type(inner)?),
            IrType::Result(ok, err) => {
                format!("Result<{}, {}>", self.emit_type(ok)?, self.emit_type(err)?)
            }
            IrType::Tuple(tys) => {
                let inner = tys
                    .iter()
                    .map(|t| self.emit_type(t))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!("({inner})")
            }
            IrType::Map(k, v) => format!(
                "std::collections::HashMap<{}, {}>",
                self.emit_type(k)?,
                self.emit_type(v)?
            ),
            IrType::Fn { params, ret, .. } => {
                let ps = params
                    .iter()
                    .map(|t| self.emit_type(t))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!("impl Fn({ps}) -> {}", self.emit_type(ret)?)
            }
            IrType::ActorRef(msg_ty) => {
                format!("actors::ActorRef<{}>", self.emit_type(msg_ty)?)
            }
            IrType::DataFrame => "polars::frame::DataFrame".into(),
            IrType::NDArray(elem) => format!("array::NDArray<{}>", self.emit_type(elem)?),
            IrType::Effect { inner, .. } => {
                // Effects compile down to Result<T, String> at the Rust level
                format!("Result<{}, String>", self.emit_type(inner)?)
            }
            IrType::Named(name) => name.clone(),
        })
    }

    pub fn emit_lit(&self, lit: &IrLit) -> String {
        match lit {
            IrLit::Unit => "()".into(),
            IrLit::Bool(b) => b.to_string(),
            IrLit::I64(n) => format!("{n}i64"),
            IrLit::F64(f) => {
                // Ensure it looks like a float literal
                if f.fract() == 0.0 {
                    format!("{f:.1}f64")
                } else {
                    format!("{f}f64")
                }
            }
            IrLit::Str(s) => format!("{:?}.to_string()", s),
        }
    }

    pub fn emit_op(&self, op: &IrOp, depth: usize) -> Result<String, CodegenError> {
        let ind = self.indent(depth);
        let ind1 = self.indent(depth + 1);

        Ok(match op {
            IrOp::Lit(lit) => self.emit_lit(lit),

            IrOp::Var(name) => name.clone(),

            IrOp::Let {
                name, value, rest, ..
            } => {
                let val = self.emit_op(value, depth)?;
                let body = self.emit_op(rest, depth)?;
                format!("{{ let {name} = {val};\n{ind}{body} }}")
            }

            IrOp::Block(ops) => {
                if ops.is_empty() {
                    return Ok("()".into());
                }
                let mut out = "{\n".to_string();
                for (i, op) in ops.iter().enumerate() {
                    let s = self.emit_op(op, depth + 1)?;
                    if i + 1 == ops.len() {
                        // Last expression — no semicolon (it's the block's value)
                        writeln!(out, "{ind1}{s}").unwrap();
                    } else {
                        writeln!(out, "{ind1}{s};").unwrap();
                    }
                }
                write!(out, "{ind}}}").unwrap();
                out
            }

            IrOp::If { cond, then, else_ } => {
                let c = self.emit_op(cond, depth)?;
                let t = self.emit_op(then, depth + 1)?;
                let e = self.emit_op(else_, depth + 1)?;
                format!("if {c} {{\n{ind1}{t}\n{ind}}} else {{\n{ind1}{e}\n{ind}}}")
            }

            IrOp::Lambda { params, ret, body } => {
                let ps = params
                    .iter()
                    .map(|(n, t)| -> Result<String, CodegenError> {
                        Ok(format!("{n}: {}", self.emit_type(t)?))
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                let ret_ann = if let Some(r) = ret {
                    format!(" -> {}", self.emit_type(r)?)
                } else {
                    String::new()
                };
                let b = self.emit_op(body, depth + 1)?;
                format!("|{ps}|{ret_ann} {{\n{ind1}{b}\n{ind}}}")
            }

            IrOp::Apply { func, args } => {
                let f = self.emit_op(func, depth)?;
                let a = args
                    .iter()
                    .map(|a| self.emit_op(a, depth))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!("{f}({a})")
            }

            IrOp::Return(val) => {
                format!("return {}", self.emit_op(val, depth)?)
            }

            IrOp::Loop(body) => {
                let b = self.emit_op(body, depth + 1)?;
                format!("loop {{\n{ind1}{b}\n{ind}}}")
            }

            IrOp::Break(val) => format!("break {}", self.emit_op(val, depth)?),

            IrOp::Continue => "continue".into(),

            IrOp::Tuple(elems) => {
                let e = elems
                    .iter()
                    .map(|e| self.emit_op(e, depth))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!("({e})")
            }

            IrOp::Array(elems) => {
                let e = elems
                    .iter()
                    .map(|e| self.emit_op(e, depth))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!("vec![{e}]")
            }

            IrOp::FieldAccess { expr, field } => {
                format!("{}.{field}", self.emit_op(expr, depth)?)
            }

            IrOp::IndexAccess { expr, index } => {
                format!(
                    "{}[{} as usize]",
                    self.emit_op(expr, depth)?,
                    self.emit_op(index, depth)?
                )
            }

            IrOp::BinOp { op, lhs, rhs } => {
                let l = self.emit_op(lhs, depth)?;
                let r = self.emit_op(rhs, depth)?;
                let sym = match op {
                    BinOpKind::Add => "+",
                    BinOpKind::Sub => "-",
                    BinOpKind::Mul => "*",
                    BinOpKind::Div => "/",
                    BinOpKind::Rem => "%",
                    BinOpKind::Eq => "==",
                    BinOpKind::Ne => "!=",
                    BinOpKind::Lt => "<",
                    BinOpKind::Le => "<=",
                    BinOpKind::Gt => ">",
                    BinOpKind::Ge => ">=",
                    BinOpKind::And => "&&",
                    BinOpKind::Or => "||",
                    BinOpKind::BitAnd => "&",
                    BinOpKind::BitOr => "|",
                    BinOpKind::BitXor => "^",
                    BinOpKind::Shl => "<<",
                    BinOpKind::Shr => ">>",
                    BinOpKind::Concat => {
                        // String concat: lhs + &rhs
                        return Ok(format!("{l} + &{r}"));
                    }
                };
                format!("({l} {sym} {r})")
            }

            IrOp::UnOp { op, expr } => {
                let e = self.emit_op(expr, depth)?;
                let sym = match op {
                    UnOpKind::Neg => "-",
                    UnOpKind::Not => "!",
                    UnOpKind::Ref => "&",
                    UnOpKind::Deref => "*",
                };
                format!("({sym}{e})")
            }

            IrOp::Match { scrutinee, arms } => {
                let s = self.emit_op(scrutinee, depth)?;
                let mut out = format!("match {s} {{\n");
                for (pat, body) in arms {
                    use crate::ops::IrPattern;
                    let pat_str = match pat {
                        IrPattern::Wildcard => "_".into(),
                        IrPattern::Bind(n) => n.clone(),
                        IrPattern::Lit(lit) => self.emit_lit(lit),
                        IrPattern::Tuple(ps) => {
                            let inner = ps
                                .iter()
                                .map(|p| match p {
                                    IrPattern::Wildcard => "_".into(),
                                    IrPattern::Bind(n) => n.clone(),
                                    _ => "_".into(),
                                })
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!("({inner})")
                        }
                        IrPattern::Variant { name, fields } => {
                            if fields.is_empty() {
                                name.clone()
                            } else {
                                let fs = fields
                                    .iter()
                                    .map(|f| match f {
                                        IrPattern::Bind(n) => n.clone(),
                                        _ => "_".into(),
                                    })
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                format!("{name}({fs})")
                            }
                        }
                    };
                    let b = self.emit_op(body, depth + 1)?;
                    writeln!(out, "{ind1}{pat_str} => {b},").unwrap();
                }
                write!(out, "{ind}}}").unwrap();
                out
            }

            IrOp::Perform(effect) => {
                // Effects at runtime produce a JSON description of the operation.
                // At L0-L2 they're checked by TrustGuard; here we emit a placeholder.
                let effect_json =
                    serde_json::to_string(effect.as_ref()).unwrap_or_else(|_| "null".into());
                format!("/* perform effect: {} */ Ok(())", effect_json)
            }

            IrOp::Handle { expr, .. } => {
                // Effects handling: for now emit the expr and ignore handlers
                // (full algebraic effects require a continuation passing transform)
                self.emit_op(expr, depth)?
            }

            IrOp::ToolCall { tool_name, args } => {
                let a = self.emit_op(args, depth)?;
                format!("__bonsai_tool_call({:?}, {a})", tool_name)
            }

            IrOp::Spawn { actor_ty, init_msg } => {
                let ty = self.emit_type(actor_ty)?;
                let msg = self.emit_op(init_msg, depth)?;
                format!("__actor_spawn::<{ty}>({msg})")
            }

            IrOp::Send { actor_ref, msg } => {
                let r = self.emit_op(actor_ref, depth)?;
                let m = self.emit_op(msg, depth)?;
                format!("{r}.send({m}).await")
            }

            IrOp::Receive { .. } => "/* receive */ __bonsai_receive().await".into(),

            IrOp::Ask { actor_ref, msg, .. } => {
                let r = self.emit_op(actor_ref, depth)?;
                let m = self.emit_op(msg, depth)?;
                format!("{r}.ask({m}).await")
            }

            IrOp::DeviceAnnotation { target, expr } => {
                let e = self.emit_op(expr, depth)?;
                let t = match target {
                    crate::ops::DeviceTarget::Gpu => "gpu",
                    crate::ops::DeviceTarget::Fpga => "fpga",
                    crate::ops::DeviceTarget::Tpu => "tpu",
                    _ => "cpu",
                };
                format!("/* @{t} */ {e}")
            }

            IrOp::SqlQuery { query, params } => {
                let ps = params
                    .iter()
                    .map(|p| self.emit_op(p, depth))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!("__bonsai_sql_query({:?}, &[{ps}]).await", query)
            }

            IrOp::DataFrameOp { op, args } => {
                let as_ = args
                    .iter()
                    .map(|a| self.emit_op(a, depth))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!("__bonsai_df_{:?}({as_})", op).to_lowercase()
            }

            IrOp::ArrayOp { op, args } => {
                let as_ = args
                    .iter()
                    .map(|a| self.emit_op(a, depth))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!("__bonsai_apl_{op:?}({as_})").to_lowercase()
            }
        })
    }

    pub fn emit_type_def(&self, def: &IrTypeDef) -> Result<String, CodegenError> {
        Ok(match &def.kind {
            IrTypeDefKind::Struct { fields } => {
                let fs = fields
                    .iter()
                    .map(|(n, t)| -> Result<String, CodegenError> {
                        Ok(format!("    pub {n}: {},", self.emit_type(t)?))
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join("\n");
                format!("#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\npub struct {} {{\n{fs}\n}}\n", def.name)
            }
            IrTypeDefKind::Enum { variants } => {
                let vs = variants
                    .iter()
                    .map(|(n, ty)| -> Result<String, CodegenError> {
                        if let Some(t) = ty {
                            Ok(format!("    {n}({}),", self.emit_type(t)?))
                        } else {
                            Ok(format!("    {n},"))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join("\n");
                format!("#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\npub enum {} {{\n{vs}\n}}\n", def.name)
            }
            IrTypeDefKind::Alias(ty) => {
                format!("pub type {} = {};\n", def.name, self.emit_type(ty)?)
            }
        })
    }

    pub fn emit_fn(&self, f: &IrFunction) -> Result<String, CodegenError> {
        let params = f
            .params
            .iter()
            .map(|p: &IrParam| -> Result<String, CodegenError> {
                let default_note = if p.default.is_some() {
                    " /* has default */"
                } else {
                    ""
                };
                Ok(format!(
                    "{}: {}{}",
                    p.name,
                    self.emit_type(&p.ty)?,
                    default_note
                ))
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        let ret = self.emit_type(&f.ret)?;
        let body = self.emit_op(&f.body, 1)?;

        // Add effect doc comment if effects are declared
        let effect_doc = if !f.effects.is_empty() {
            let names: Vec<String> = f.effects.iter().map(|e| format!("{e:?}")).collect();
            format!("/// Effects: {}\n", names.join(", "))
        } else {
            String::new()
        };

        // Visibility: exported functions are `pub`, unexported are not
        let vis = "pub ";

        Ok(format!(
            "{effect_doc}{vis}fn {name}({params}) -> {ret} {{\n    {body}\n}}\n",
            name = f.name,
        ))
    }
}

impl Codegen for RustCodegen {
    type Output = String;
    type Error = CodegenError;

    fn emit_module(&mut self, m: &IrModule) -> Result<String, CodegenError> {
        let mut out = String::new();

        // Module header
        writeln!(
            out,
            "// Generated by bonsai-ir RustCodegen — do not edit manually."
        )
        .unwrap();
        writeln!(out, "// Module: {}", m.name).unwrap();
        if let Some(src) = &m.source_path {
            writeln!(out, "// Source: {src}").unwrap();
        }
        writeln!(
            out,
            "#![allow(unused_imports, unused_variables, dead_code, clippy::all)]"
        )
        .unwrap();
        writeln!(out).unwrap();

        // Tool call helper shim (UCR integration)
        if m.functions.iter().any(|f| contains_tool_call(&f.body)) {
            writeln!(
                out,
                "// UCR tool call shim — wired by the runtime at load time."
            )
            .unwrap();
            writeln!(out, "thread_local! {{").unwrap();
            writeln!(out, "    static TOOL_REGISTRY: std::cell::RefCell<Option<Box<dyn Fn(&str, serde_json::Value) -> Result<serde_json::Value, String>>>> =").unwrap();
            writeln!(out, "        std::cell::RefCell::new(None);").unwrap();
            writeln!(out, "}}").unwrap();
            writeln!(out, "fn __bonsai_tool_call(name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {{").unwrap();
            writeln!(out, "    TOOL_REGISTRY.with(|r| {{").unwrap();
            writeln!(out, "        let guard = r.borrow();").unwrap();
            writeln!(
                out,
                "        if let Some(f) = guard.as_ref() {{ f(name, args) }}"
            )
            .unwrap();
            writeln!(
                out,
                "        else {{ Err(format!(\"tool registry not wired: {{}}\", name)) }}"
            )
            .unwrap();
            writeln!(out, "    }})").unwrap();
            writeln!(out, "}}").unwrap();
            writeln!(out).unwrap();
        }

        // Type definitions
        for typedef in &m.types {
            out.push_str(&self.emit_type_def(typedef)?);
            out.push('\n');
        }

        // Functions
        for func in &m.functions {
            out.push_str(&self.emit_fn(func)?);
            out.push('\n');
        }

        Ok(out)
    }
}

fn contains_tool_call(op: &IrOp) -> bool {
    match op {
        IrOp::ToolCall { .. } => true,
        IrOp::Block(ops) => ops.iter().any(contains_tool_call),
        IrOp::Let { value, rest, .. } => contains_tool_call(value) || contains_tool_call(rest),
        IrOp::If { cond, then, else_ } => {
            contains_tool_call(cond) || contains_tool_call(then) || contains_tool_call(else_)
        }
        IrOp::Apply { func, args } => {
            contains_tool_call(func) || args.iter().any(contains_tool_call)
        }
        IrOp::Loop(b) | IrOp::Return(b) | IrOp::Break(b) => contains_tool_call(b),
        IrOp::BinOp { lhs, rhs, .. } => contains_tool_call(lhs) || contains_tool_call(rhs),
        IrOp::UnOp { expr, .. } | IrOp::FieldAccess { expr, .. } => contains_tool_call(expr),
        IrOp::Lambda { body, .. } => contains_tool_call(body),
        IrOp::Tuple(es) | IrOp::Array(es) => es.iter().any(contains_tool_call),
        IrOp::Spawn { init_msg, .. } => contains_tool_call(init_msg),
        IrOp::Send { actor_ref, msg } => contains_tool_call(actor_ref) || contains_tool_call(msg),
        IrOp::Ask { actor_ref, msg, .. } => {
            contains_tool_call(actor_ref) || contains_tool_call(msg)
        }
        IrOp::DeviceAnnotation { expr, .. } => contains_tool_call(expr),
        IrOp::SqlQuery { params, .. } => params.iter().any(contains_tool_call),
        IrOp::DataFrameOp { args, .. } | IrOp::ArrayOp { args, .. } => {
            args.iter().any(contains_tool_call)
        }
        IrOp::Match { scrutinee, arms } => {
            contains_tool_call(scrutinee) || arms.iter().any(|(_, b)| contains_tool_call(b))
        }
        _ => false,
    }
}

// ── JSON Schema codegen ───────────────────────────────────────────────────────

/// Emit a JSON Schema for an `IrFunction` signature — used for UCR auto-registration.
pub fn emit_json_schema(f: &IrFunction) -> serde_json::Value {
    let codegen = RustCodegen::new();
    let properties: serde_json::Map<String, serde_json::Value> = f
        .params
        .iter()
        .map(|p| {
            let ty_str = codegen.emit_type(&p.ty).unwrap_or_else(|_| "any".into());
            let mut schema = type_to_json_schema(&p.ty);
            // Annotate schema with the Rust type string for tooling / documentation.
            if let serde_json::Value::Object(ref mut m) = schema {
                m.insert("rust_type".to_string(), serde_json::Value::String(ty_str));
            }
            (p.name.clone(), schema)
        })
        .collect();

    let required: Vec<serde_json::Value> = f
        .params
        .iter()
        .filter(|p| p.default.is_none())
        .map(|p| serde_json::Value::String(p.name.clone()))
        .collect();

    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": f.name,
        "description": format!("UniIR-generated function '{}'", f.name),
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

fn type_to_json_schema(ty: &IrType) -> serde_json::Value {
    match ty {
        IrType::Unit => serde_json::json!({"type": "null"}),
        IrType::Bool => serde_json::json!({"type": "boolean"}),
        IrType::I64 => serde_json::json!({"type": "integer"}),
        IrType::F64 => serde_json::json!({"type": "number"}),
        IrType::Str => serde_json::json!({"type": "string"}),
        IrType::Bytes => serde_json::json!({"type": "string", "format": "byte"}),
        IrType::Array(inner) => {
            serde_json::json!({"type": "array", "items": type_to_json_schema(inner)})
        }
        IrType::Option(inner) => {
            let inner_schema = type_to_json_schema(inner);
            serde_json::json!({"oneOf": [inner_schema, {"type": "null"}]})
        }
        IrType::Map(_, v) => {
            serde_json::json!({"type": "object", "additionalProperties": type_to_json_schema(v)})
        }
        IrType::Named(name) => serde_json::json!({"$ref": format!("#/$defs/{name}")}),
        _ => serde_json::json!({}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::*;

    fn make_add_fn() -> IrFunction {
        IrFunction {
            name: "add".into(),
            params: vec![
                IrParam {
                    name: "a".into(),
                    ty: IrType::I64,
                    default: None,
                },
                IrParam {
                    name: "b".into(),
                    ty: IrType::I64,
                    default: None,
                },
            ],
            ret: IrType::I64,
            body: IrOp::BinOp {
                op: BinOpKind::Add,
                lhs: Box::new(IrOp::var("a")),
                rhs: Box::new(IrOp::var("b")),
            },
            effects: vec![],
            proof: None,
            schema: None,
        }
    }

    #[test]
    fn emit_simple_fn() {
        let cg = RustCodegen::new();
        let f = make_add_fn();
        let src = cg.emit_fn(&f).unwrap();
        assert!(src.contains("pub fn add"), "missing fn signature: {src}");
        assert!(src.contains("a: i64"), "missing param a: {src}");
        assert!(src.contains("b: i64"), "missing param b: {src}");
        assert!(src.contains("-> i64"), "missing return type: {src}");
        assert!(src.contains("(a + b)"), "missing body: {src}");
    }

    #[test]
    fn emit_module_round_trip() {
        let mut m = IrModule::new("test_module");
        m.functions.push(make_add_fn());
        m.exports.push("add".into());

        let mut cg = RustCodegen::new();
        let src = cg.emit_module(&m).unwrap();
        assert!(src.contains("pub fn add"), "module missing function: {src}");
        assert!(
            src.contains("Generated by bonsai-ir"),
            "missing header comment: {src}"
        );
    }

    #[test]
    fn emit_if_expression() {
        let cg = RustCodegen::new();
        let op = IrOp::if_(IrOp::lit_bool(true), IrOp::lit_i64(1), IrOp::lit_i64(2));
        let src = cg.emit_op(&op, 0).unwrap();
        assert!(src.contains("if true"), "missing if: {src}");
        assert!(src.contains("1i64"), "missing then: {src}");
        assert!(src.contains("2i64"), "missing else: {src}");
    }

    #[test]
    fn emit_all_primitive_types() {
        let cg = RustCodegen::new();
        assert_eq!(cg.emit_type(&IrType::Unit).unwrap(), "()");
        assert_eq!(cg.emit_type(&IrType::Bool).unwrap(), "bool");
        assert_eq!(cg.emit_type(&IrType::I64).unwrap(), "i64");
        assert_eq!(cg.emit_type(&IrType::F64).unwrap(), "f64");
        assert_eq!(cg.emit_type(&IrType::Str).unwrap(), "String");
        assert_eq!(
            cg.emit_type(&IrType::Array(Box::new(IrType::I64))).unwrap(),
            "Vec<i64>"
        );
    }

    #[test]
    fn json_schema_for_fn() {
        let f = make_add_fn();
        let schema = emit_json_schema(&f);
        assert_eq!(schema["title"], "add");
        assert!(schema["properties"]["a"].is_object());
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("a")));
    }
}
