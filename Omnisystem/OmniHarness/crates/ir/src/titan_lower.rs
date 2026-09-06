//! Titan (the `bootstrap-rs` / `titan` crate) AST → UniIR lowering.
//!
//! This is the second real input language for the `ir` crate's
//! parse → `IrModule` → Rust pipeline, alongside the existing Sylva-subset
//! parser in `parser.rs`. It reuses Titan's own real lexer/parser
//! (`titan::parser::parse`) rather than re-implementing one, and lowers
//! Titan's real AST (`titan::ast`) into the same `ops::IrModule` that the
//! Sylva path produces, so both languages flow through one shared codegen.
//!
//! ## Scope of this v1 lowering — read before assuming a feature works
//!
//! Supported:
//! - top-level `fn` items (`pub fn` or plain `fn`), each with a single block body
//! - statements: `let NAME = expr;` with a plain identifier bind pattern, and
//!   bare expression statements (sequenced; the final one is the block's value)
//! - expressions: integer / float / string / bool literals; single-segment
//!   variable references; unary `-` / `!`; binary arithmetic, comparison,
//!   logical and bitwise operators; `if { .. } else { .. }` (both branches
//!   required — no `if let`); `return`; calls to other functions defined in
//!   the same file; and the `println!` / `print!` / `eprintln!` / `format!`
//!   macros (lowered to a new `IrOp::Macro` node, emitted as the real Rust
//!   macro by `codegen.rs`)
//!
//! Explicitly NOT supported in v1 (rejected with a `LowerError`, never
//! silently mishandled or dropped):
//! - `struct` / `enum` / `impl` / `trait` / `mod` / `const` / `use` items,
//!   and therefore also `self`, methods, field access (`a.b`), and
//!   `Type::method(...)` paths
//! - `match`, `loop`, `while`, `for`, labeled break/continue
//! - closures, arrays, tuples, ranges, casts, `?`, indexing
//! - any `let` pattern other than a plain identifier (no destructuring)
//! - multi-segment paths (`Foo::bar`) anywhere
//!
//! This covers `01_hello.titan` and `02_fib.titan` from
//! `Omnisystem/bootstrap/tests/` end-to-end (see the `ir` crate's own test
//! suite and the CLI proof run referenced in the project's commit history);
//! it is a real but partial subset, not full Titan.
//!
//! ## Why types have to be inferred at all
//!
//! Titan's own AST carries no static types: `titan::ast::Param` is just
//! `{ name, is_self }` and `titan::ast::Field` is just `{ name }`. Type
//! annotations written in Titan source (`n: i32`) are consumed and thrown
//! away by `bootstrap-rs`'s parser — Titan is dynamically typed at the
//! parser/interpreter level, with no separate type-checking pass. UniIR, by
//! contrast, requires a concrete `IrType` on every parameter and return.
//!
//! So parameter types here are *not* real type inference — they are a
//! shallow syntactic heuristic: a function's parameters and return type are
//! `F64` if any float literal appears anywhere in the function body,
//! otherwise `I64`. This is correct for arithmetic-shaped functions like
//! `fib`, and simply wrong for anything that needs a `Str`, `Bool`, or
//! struct-typed parameter — such functions are out of scope for v1 unless
//! the return type is explicitly annotated in the Titan source (`-> i32`,
//! `-> bool`, `-> String`, ...), which this pass *does* read from
//! `FnItem::ret` (a real `TypeRef`, unlike parameters).

use crate::ops::{BinOpKind, IrFunction, IrLit, IrModule, IrOp, IrParam, IrType, UnOpKind};
use titan::ast::{Block, Expr, FnItem, Item, Pattern, Program, Stmt};

#[derive(Debug, Clone)]
pub struct LowerError(pub String);

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "titan lowering error: {}", self.0)
    }
}

impl std::error::Error for LowerError {}

type LowerResult<T> = Result<T, LowerError>;

fn unsupported(msg: impl Into<String>) -> LowerError {
    LowerError(format!("unsupported in v1 Titan->IR lowering: {}", msg.into()))
}

/// Parse Titan source (via the real `titan` crate parser) and lower it to an
/// `IrModule`, covering the subset documented at the top of this file.
pub fn lower_source(src: &str, file: &str, module_name: &str) -> LowerResult<IrModule> {
    let program = titan::parser::parse(src, file)
        .map_err(|e| LowerError(format!("titan parse error: {}", e.render(src))))?;
    lower_program(&program, module_name)
}

pub fn lower_program(program: &Program, module_name: &str) -> LowerResult<IrModule> {
    let mut module = IrModule::new(module_name);
    for item in &program.items {
        match item {
            Item::Fn(f) => {
                let func = lower_fn(f)?;
                module.exports.push(func.name.clone());
                module.functions.push(func);
            }
            other => {
                return Err(unsupported(format!(
                    "top-level item kind {:?} (only `fn` items are lowered in v1)",
                    item_kind_name(other)
                )));
            }
        }
    }
    Ok(module)
}

fn item_kind_name(item: &Item) -> &'static str {
    match item {
        Item::Use => "use",
        Item::Struct(_) => "struct",
        Item::Enum(_) => "enum",
        Item::Impl(_) => "impl",
        Item::Trait(_) => "trait",
        Item::Fn(_) => "fn",
        Item::Const(_) => "const",
        Item::Mod(_) => "mod",
    }
}

fn lower_fn(f: &FnItem) -> LowerResult<IrFunction> {
    let body_block = f
        .body
        .as_ref()
        .ok_or_else(|| unsupported(format!("function `{}` has no body (trait signature?)", f.name)))?;

    // Heuristic numeric type: see module doc comment. `F64` if any float
    // literal shows up anywhere in the body, else `I64`.
    let numeric_ty = if block_has_float_literal(body_block) {
        IrType::F64
    } else {
        IrType::I64
    };

    let mut params = Vec::with_capacity(f.params.len());
    for p in &f.params {
        if p.is_self {
            return Err(unsupported(format!(
                "method `{}` takes `self` — methods/impls are not lowered in v1",
                f.name
            )));
        }
        params.push(IrParam {
            name: p.name.clone(),
            ty: numeric_ty.clone(),
            default: None,
        });
    }

    let ret = match &f.ret {
        None => IrType::Unit,
        Some(tr) => map_type_ref_name(&tr.name)
            .ok_or_else(|| unsupported(format!("return type `{}` on `{}`", tr.name, f.name)))?,
    };

    let body = lower_block(body_block)?;

    Ok(IrFunction {
        name: f.name.clone(),
        params,
        ret,
        body,
        effects: vec![],
        proof: None,
        schema: None,
    })
}

/// Map a Titan `TypeRef` name (as written in source, e.g. `-> i32`) to an
/// `IrType`. Only the primitive names actually needed by the v1 subset are
/// recognized; anything else (generics, `Self`, user types, `Vec<T>`, ...)
/// is intentionally left unmapped so the caller can report it clearly.
fn map_type_ref_name(name: &str) -> Option<IrType> {
    Some(match name {
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize" | "isize" => {
            IrType::I64
        }
        "f32" | "f64" => IrType::F64,
        "bool" => IrType::Bool,
        "String" | "str" => IrType::Str,
        "()" => IrType::Unit,
        _ => return None,
    })
}

fn block_has_float_literal(block: &Block) -> bool {
    block.stmts.iter().any(stmt_has_float_literal)
}

fn stmt_has_float_literal(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_has_float_literal),
        Stmt::Expr { expr, .. } => expr_has_float_literal(expr),
        Stmt::Item(_) => false,
    }
}

fn expr_has_float_literal(e: &Expr) -> bool {
    match e {
        Expr::Float { .. } => true,
        Expr::Call { callee, args, .. } => {
            expr_has_float_literal(callee) || args.iter().any(expr_has_float_literal)
        }
        Expr::Method { recv, args, .. } => {
            expr_has_float_literal(recv) || args.iter().any(expr_has_float_literal)
        }
        Expr::Unary { operand, .. } => expr_has_float_literal(operand),
        Expr::Binary { left, right, .. } => {
            expr_has_float_literal(left) || expr_has_float_literal(right)
        }
        Expr::Assign { target, value, .. } => {
            expr_has_float_literal(target) || expr_has_float_literal(value)
        }
        Expr::If { cond, then, els, .. } => {
            expr_has_float_literal(cond)
                || block_has_float_literal(then)
                || els.as_deref().is_some_and(expr_has_float_literal)
        }
        Expr::Match { scrut, arms, .. } => {
            expr_has_float_literal(scrut) || arms.iter().any(|a| expr_has_float_literal(&a.body))
        }
        Expr::While { cond, body, .. } => expr_has_float_literal(cond) || block_has_float_literal(body),
        Expr::For { iter, body, .. } => expr_has_float_literal(iter) || block_has_float_literal(body),
        Expr::Loop { body, .. } => block_has_float_literal(body),
        Expr::BlockE { block } => block_has_float_literal(block),
        Expr::Return { value, .. } => value.as_deref().is_some_and(expr_has_float_literal),
        Expr::Break { value, .. } => value.as_deref().is_some_and(expr_has_float_literal),
        Expr::Field { obj, .. } => expr_has_float_literal(obj),
        Expr::Index { obj, index, .. } => {
            expr_has_float_literal(obj) || expr_has_float_literal(index)
        }
        Expr::Array { elems, repeat, .. } => {
            elems.iter().any(expr_has_float_literal)
                || repeat.as_deref().is_some_and(expr_has_float_literal)
        }
        Expr::Tuple { elems, .. } => elems.iter().any(expr_has_float_literal),
        Expr::Macro { args, .. } => args.iter().any(expr_has_float_literal),
        _ => false,
    }
}

fn lower_block(block: &Block) -> LowerResult<IrOp> {
    lower_stmts(&block.stmts)
}

fn lower_stmts(stmts: &[Stmt]) -> LowerResult<IrOp> {
    match stmts.split_first() {
        None => Ok(IrOp::unit()),
        Some((Stmt::Let { pat, init, .. }, rest)) => {
            let name = expect_bind_name(pat)?;
            let init_expr = init
                .as_ref()
                .ok_or_else(|| unsupported(format!("`let {name}` with no initializer")))?;
            let value = lower_expr(init_expr)?;
            let rest_op = lower_stmts(rest)?;
            Ok(IrOp::Let {
                name,
                ty: None,
                value: Box::new(value),
                rest: Box::new(rest_op),
            })
        }
        Some((Stmt::Expr { expr, .. }, rest)) => {
            let head = lower_expr(expr)?;
            if rest.is_empty() {
                Ok(head)
            } else {
                let rest_op = lower_stmts(rest)?;
                Ok(IrOp::Block(vec![head, rest_op]))
            }
        }
        Some((Stmt::Item(_), _)) => {
            Err(unsupported("a nested item declaration inside a function body"))
        }
    }
}

fn expect_bind_name(pat: &Pattern) -> LowerResult<String> {
    match pat {
        Pattern::Bind { name, .. } => Ok(name.clone()),
        other => Err(unsupported(format!(
            "`let` pattern {other:?} (only a plain identifier bind is supported)"
        ))),
    }
}

const SUPPORTED_MACROS: &[&str] = &["println", "print", "eprintln", "eprint", "format"];

fn lower_expr(e: &Expr) -> LowerResult<IrOp> {
    Ok(match e {
        Expr::Int { v, .. } => IrOp::lit_i64(*v),
        Expr::Float { v, .. } => IrOp::Lit(IrLit::F64(*v)),
        Expr::Str { v, .. } => IrOp::lit_str(v.clone()),
        Expr::Bool { v, .. } => IrOp::lit_bool(*v),
        Expr::Char { .. } => return Err(unsupported("char literals")),

        Expr::Path { segs, .. } => {
            if segs.len() != 1 {
                return Err(unsupported(format!("multi-segment path `{}`", segs.join("::"))));
            }
            IrOp::var(segs[0].clone())
        }

        Expr::Field { .. } => return Err(unsupported("field access (`a.b`)")),
        Expr::Index { .. } => return Err(unsupported("indexing (`a[i]`)")),

        Expr::Call { callee, args, .. } => {
            let lowered_args = args.iter().map(lower_expr).collect::<LowerResult<Vec<_>>>()?;
            let func = lower_expr(callee)?;
            IrOp::apply(func, lowered_args)
        }

        Expr::Method { .. } => return Err(unsupported("method calls (`recv.method(...)`)")),

        Expr::Unary { op, operand, .. } => {
            let kind = match op.as_str() {
                "-" => UnOpKind::Neg,
                "!" => UnOpKind::Not,
                other => return Err(unsupported(format!("unary operator `{other}`"))),
            };
            IrOp::UnOp {
                op: kind,
                expr: Box::new(lower_expr(operand)?),
            }
        }

        Expr::Binary { op, left, right, .. } => {
            let kind = match op.as_str() {
                "+" => BinOpKind::Add,
                "-" => BinOpKind::Sub,
                "*" => BinOpKind::Mul,
                "/" => BinOpKind::Div,
                "%" => BinOpKind::Rem,
                "==" => BinOpKind::Eq,
                "!=" => BinOpKind::Ne,
                "<" => BinOpKind::Lt,
                "<=" => BinOpKind::Le,
                ">" => BinOpKind::Gt,
                ">=" => BinOpKind::Ge,
                "&&" => BinOpKind::And,
                "||" => BinOpKind::Or,
                "&" => BinOpKind::BitAnd,
                "|" => BinOpKind::BitOr,
                "^" => BinOpKind::BitXor,
                "<<" => BinOpKind::Shl,
                ">>" => BinOpKind::Shr,
                other => return Err(unsupported(format!("binary operator `{other}`"))),
            };
            IrOp::BinOp {
                op: kind,
                lhs: Box::new(lower_expr(left)?),
                rhs: Box::new(lower_expr(right)?),
            }
        }

        Expr::Assign { .. } => return Err(unsupported("assignment (`=`, `+=`, ...)")),
        Expr::Range { .. } => return Err(unsupported("range expressions")),

        Expr::If { let_pat, cond, then, els, .. } => {
            if let_pat.is_some() {
                return Err(unsupported("`if let`"));
            }
            let cond_op = lower_expr(cond)?;
            let then_op = lower_block(then)?;
            let else_op = match els {
                None => IrOp::unit(),
                Some(e) => lower_expr(e)?,
            };
            IrOp::if_(cond_op, then_op, else_op)
        }

        Expr::Match { .. } => return Err(unsupported("`match`")),
        Expr::While { .. } => return Err(unsupported("`while`")),
        Expr::For { .. } => return Err(unsupported("`for`")),
        Expr::Loop { .. } => return Err(unsupported("`loop`")),

        Expr::BlockE { block } => lower_block(block)?,

        Expr::Return { value, .. } => {
            let v = match value {
                None => IrOp::unit(),
                Some(e) => lower_expr(e)?,
            };
            IrOp::Return(Box::new(v))
        }

        Expr::Break { .. } => return Err(unsupported("`break`")),
        Expr::Continue { .. } => return Err(unsupported("`continue`")),

        Expr::StructLit { .. } => return Err(unsupported("struct literals")),
        Expr::Array { .. } => return Err(unsupported("array literals")),
        Expr::Tuple { .. } => return Err(unsupported("tuple literals")),
        Expr::Closure { .. } => return Err(unsupported("closures")),
        Expr::Try { .. } => return Err(unsupported("`?`")),
        Expr::Cast { .. } => return Err(unsupported("`as` casts")),

        Expr::Macro { name, args, repeat, .. } => {
            if repeat.is_some() {
                return Err(unsupported(format!("macro `{name}!` with a repeat count (`; n`)")));
            }
            if !SUPPORTED_MACROS.contains(&name.as_str()) {
                return Err(unsupported(format!("macro `{name}!` (only println!/print!/eprintln!/eprint!/format! are lowered)")));
            }
            let lowered_args = args.iter().map(lower_expr).collect::<LowerResult<Vec<_>>>()?;
            IrOp::Macro {
                name: name.clone(),
                args: lowered_args,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::{Codegen, RustCodegen};

    fn lower(src: &str) -> IrModule {
        lower_source(src, "test.titan", "test").unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn lowers_hello_world() {
        let src = "pub fn main() {\n    println!(\"Hello, Omnisystem!\")\n}\n";
        let m = lower(src);
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.functions[0].name, "main");
        assert!(matches!(m.functions[0].body, IrOp::Macro { .. }));
        let mut cg = RustCodegen::new();
        let rust = cg.emit_module(&m).unwrap();
        assert!(rust.contains("println!(\"Hello, Omnisystem!\")"), "{rust}");
    }

    #[test]
    fn lowers_fib() {
        let src = "fn fib(n: i32) -> i32 {\n    if n < 2 { return n }\n    fib(n - 1) + fib(n - 2)\n}\npub fn main() {\n    println!(\"fib(10) = {}\", fib(10))\n}\n";
        let m = lower(src);
        assert_eq!(m.functions.len(), 2);
        let fib = m.get_fn("fib").unwrap();
        assert_eq!(fib.params[0].ty, IrType::I64);
        assert_eq!(fib.ret, IrType::I64);
        let mut cg = RustCodegen::new();
        let rust = cg.emit_module(&m).unwrap();
        assert!(rust.contains("pub fn fib(n: i64) -> i64"), "{rust}");
    }

    #[test]
    fn rejects_struct_items() {
        let err = lower_source("struct Point { x: i32 }\npub fn main() {}\n", "t.titan", "t")
            .unwrap_err();
        assert!(err.to_string().contains("struct"), "{err}");
    }

    #[test]
    fn rejects_methods_with_self() {
        let src = "struct P { x: i32 }\nimpl P { fn get(&self) -> i32 { self.x } }\npub fn main() {}\n";
        assert!(lower_source(src, "t.titan", "t").is_err());
    }
}
