//! UniIR — Universal Intermediate Representation types.
//!
//! `IrOp` is the typed expression language. `IrFunction` and `IrModule` are
//! the compilation units. The effect system in `effects.rs` is the effect type.

use serde::{Deserialize, Serialize};
use crate::effects::BonsaiEffect;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrType {
    Unit,
    Bool,
    I64,
    F64,
    Str,
    Bytes,
    Array(Box<IrType>),
    Map(Box<IrType>, Box<IrType>),
    Option(Box<IrType>),
    Result(Box<IrType>, Box<IrType>),
    Tuple(Vec<IrType>),
    Fn { params: Vec<IrType>, ret: Box<IrType> },
    /// Monadic effect wrapper — a computation that may perform effects of `effect_type`
    /// and returns a value of `inner`.
    Effect { inner: Box<IrType>, effect_type: EffectType },
    /// Named struct/enum defined in an `IrModule`.
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectType {
    Pure,
    IO,
    Model,
    Storage,
    Network,
    Any,
}

// ── Pattern matching ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrPattern {
    Wildcard,
    Bind(String),
    Lit(IrLit),
    Tuple(Vec<IrPattern>),
    Variant { name: String, fields: Vec<IrPattern> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrLit {
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    Unit,
}

// ── Core expression ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrOp {
    // ── Literals ──────────────────────────────────────────────────────────────
    Lit(IrLit),

    // ── Variable binding ──────────────────────────────────────────────────────
    /// `let name: ty = value; rest`
    Let {
        name: String,
        ty: Option<IrType>,
        value: Box<IrOp>,
        rest: Box<IrOp>,
    },
    Var(String),

    // ── Functions ─────────────────────────────────────────────────────────────
    /// Lambda literal
    Lambda {
        params: Vec<(String, IrType)>,
        ret: Option<IrType>,
        body: Box<IrOp>,
    },
    /// Function application
    Apply {
        func: Box<IrOp>,
        args: Vec<IrOp>,
    },

    // ── Control flow ──────────────────────────────────────────────────────────
    If {
        cond: Box<IrOp>,
        then: Box<IrOp>,
        else_: Box<IrOp>,
    },
    Match {
        scrutinee: Box<IrOp>,
        arms: Vec<(IrPattern, IrOp)>,
    },
    /// Infinite loop; `Break(value)` exits with a value
    Loop(Box<IrOp>),
    Break(Box<IrOp>),
    Continue,
    Return(Box<IrOp>),

    // ── Sequences ─────────────────────────────────────────────────────────────
    /// Ordered sequence of expressions; value is the last element
    Block(Vec<IrOp>),

    // ── Algebraic data ────────────────────────────────────────────────────────
    Tuple(Vec<IrOp>),
    Array(Vec<IrOp>),
    FieldAccess { expr: Box<IrOp>, field: String },
    IndexAccess { expr: Box<IrOp>, index: Box<IrOp> },

    // ── Primitives ────────────────────────────────────────────────────────────
    BinOp { op: BinOpKind, lhs: Box<IrOp>, rhs: Box<IrOp> },
    UnOp { op: UnOpKind, expr: Box<IrOp> },

    // ── Effects ───────────────────────────────────────────────────────────────
    /// Lift a `BonsaiEffect` into the IR — the computation *performs* this effect
    Perform(Box<BonsaiEffect>),
    /// Effect handler: run `expr`, intercept effects, resume with `handlers`
    Handle {
        expr: Box<IrOp>,
        handlers: Vec<EffectHandler>,
    },

    // ── Tools (UCR integration) ───────────────────────────────────────────────
    /// Call a named tool from the UCR by name at runtime
    ToolCall {
        tool_name: String,
        args: Box<IrOp>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectHandler {
    pub effect_kind: String,
    pub param: String,
    pub resume: String,
    pub body: IrOp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinOpKind {
    Add, Sub, Mul, Div, Rem,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    BitAnd, BitOr, BitXor, Shl, Shr,
    Concat,  // string/array concatenation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnOpKind {
    Neg, Not, Ref, Deref,
}

// ── Function and module ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrParam {
    pub name: String,
    pub ty: IrType,
    pub default: Option<IrLit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<IrParam>,
    pub ret: IrType,
    pub body: IrOp,
    /// Declared effects — must be a superset of effects actually used in `body`.
    pub effects: Vec<BonsaiEffect>,
    /// Optional Axiom proof for `TrustGuard` L3 elevation.
    pub proof: Option<IrProof>,
    /// JSON Schema for auto-registration in the UCR.
    pub schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrProof {
    /// The proposition being proved (serialized `Term` from `bonsai-verify`).
    pub statement: serde_json::Value,
    /// The proof term.
    pub proof_term: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrTypeDef {
    pub name: String,
    pub kind: IrTypeDefKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrTypeDefKind {
    Struct { fields: Vec<(String, IrType)> },
    Enum { variants: Vec<(String, Option<IrType>)> },
    Alias(IrType),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrModule {
    pub name: String,
    pub functions: Vec<IrFunction>,
    pub types: Vec<IrTypeDef>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    /// Source file this module was compiled from, if any.
    pub source_path: Option<String>,
}

impl IrModule {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            functions: vec![],
            types: vec![],
            imports: vec![],
            exports: vec![],
            source_path: None,
        }
    }

    pub fn get_fn(&self, name: &str) -> Option<&IrFunction> {
        self.functions.iter().find(|f| f.name == name)
    }

    pub fn exported_fns(&self) -> Vec<&IrFunction> {
        self.functions.iter()
            .filter(|f| self.exports.contains(&f.name))
            .collect()
    }
}

// ── Builder helpers ───────────────────────────────────────────────────────────

impl IrOp {
    pub fn lit_i64(n: i64) -> Self { Self::Lit(IrLit::I64(n)) }
    pub fn lit_str(s: impl Into<String>) -> Self { Self::Lit(IrLit::Str(s.into())) }
    pub fn lit_bool(b: bool) -> Self { Self::Lit(IrLit::Bool(b)) }
    pub fn unit() -> Self { Self::Lit(IrLit::Unit) }
    pub fn var(name: impl Into<String>) -> Self { Self::Var(name.into()) }

    pub fn block(ops: impl IntoIterator<Item = IrOp>) -> Self {
        let v: Vec<IrOp> = ops.into_iter().collect();
        if v.len() == 1 { v.into_iter().next().unwrap() } else { Self::Block(v) }
    }

    pub fn if_(cond: IrOp, then: IrOp, else_: IrOp) -> Self {
        Self::If { cond: Box::new(cond), then: Box::new(then), else_: Box::new(else_) }
    }

    pub fn apply(func: IrOp, args: Vec<IrOp>) -> Self {
        Self::Apply { func: Box::new(func), args }
    }

    pub fn tool_call(name: impl Into<String>, args: IrOp) -> Self {
        Self::ToolCall { tool_name: name.into(), args: Box::new(args) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_simple_fn() {
        let f = IrFunction {
            name: "add".into(),
            params: vec![
                IrParam { name: "a".into(), ty: IrType::I64, default: None },
                IrParam { name: "b".into(), ty: IrType::I64, default: None },
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
        };
        assert_eq!(f.name, "add");
        // Ensure round-trip through JSON
        let json = serde_json::to_string(&f).unwrap();
        let f2: IrFunction = serde_json::from_str(&json).unwrap();
        assert_eq!(f2.name, f.name);
    }

    #[test]
    fn module_get_fn() {
        let mut m = IrModule::new("test");
        m.functions.push(IrFunction {
            name: "foo".into(),
            params: vec![],
            ret: IrType::Unit,
            body: IrOp::unit(),
            effects: vec![],
            proof: None,
            schema: None,
        });
        m.exports.push("foo".into());
        assert!(m.get_fn("foo").is_some());
        assert_eq!(m.exported_fns().len(), 1);
    }
}
