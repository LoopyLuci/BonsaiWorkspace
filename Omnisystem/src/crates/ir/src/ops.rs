//! UniIR — Universal Intermediate Representation types.
//!
//! `IrOp` is the typed expression language. `IrFunction` and `IrModule` are
//! the compilation units. The effect system in `effects.rs` is the effect type.

use crate::effects::BonsaiEffect;
use serde::{Deserialize, Serialize};

// ── Types ─────────────────────────────────────────────────────────────────────

/// Graded modality — how many times a value is used.
/// ZERO = erased (type-level only), ONE = linear (consumed once), MANY = unrestricted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Modality {
    #[default]
    Many,
    One,
    Zero,
}

/// Compute device target for a function or operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DeviceTarget {
    #[default]
    Cpu,
    Gpu,
    Fpga,
    Tpu,
    Auto,
}

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
    /// Function type with graded modality on arguments.
    Fn {
        params: Vec<IrType>,
        ret: Box<IrType>,
        modality: Modality,
    },
    /// Monadic effect wrapper.
    Effect {
        inner: Box<IrType>,
        effect_type: EffectType,
    },
    /// Named struct/enum defined in an `IrModule`.
    Named(String),
    /// A typed actor reference — can receive messages of type `msg`.
    ActorRef(Box<IrType>),
    /// Polars-backed DataFrame.
    DataFrame,
    /// Rank-polymorphic array (APL/J array model); element type T.
    NDArray(Box<IrType>),
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
    Variant {
        name: String,
        fields: Vec<IrPattern>,
    },
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
    FieldAccess {
        expr: Box<IrOp>,
        field: String,
    },
    IndexAccess {
        expr: Box<IrOp>,
        index: Box<IrOp>,
    },

    // ── Primitives ────────────────────────────────────────────────────────────
    BinOp {
        op: BinOpKind,
        lhs: Box<IrOp>,
        rhs: Box<IrOp>,
    },
    UnOp {
        op: UnOpKind,
        expr: Box<IrOp>,
    },

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

    // ── Actor concurrency (Aether layer) ─────────────────────────────────────
    /// Spawn a new actor with an initial message of type `msg_ty`.
    Spawn {
        actor_ty: IrType,
        init_msg: Box<IrOp>,
    },
    /// Send a message to an actor reference (fire-and-forget).
    Send {
        actor_ref: Box<IrOp>,
        msg: Box<IrOp>,
    },
    /// Receive the next message from the current actor's mailbox.
    Receive {
        msg_ty: IrType,
    },
    /// Synchronous ask: send `msg` and await a reply of `reply_ty`.
    Ask {
        actor_ref: Box<IrOp>,
        msg: Box<IrOp>,
        reply_ty: IrType,
    },

    // ── Device annotation (UniIR targeting) ──────────────────────────────────
    /// Annotate an expression with a compute-device target.
    DeviceAnnotation {
        target: DeviceTarget,
        expr: Box<IrOp>,
    },

    // ── SQL ───────────────────────────────────────────────────────────────────
    /// Execute a SQL query string with positional bind parameters.
    SqlQuery {
        query: String,
        params: Vec<IrOp>,
    },

    // ── DataFrame ops (Polars) ────────────────────────────────────────────────
    DataFrameOp {
        op: DataFrameOpKind,
        args: Vec<IrOp>,
    },

    // ── Array language ops (APL/J) ────────────────────────────────────────────
    ArrayOp {
        op: ArrayOpKind,
        args: Vec<IrOp>,
    },
}

/// DataFrame operations (mirrors Polars lazy API).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataFrameOpKind {
    LoadCsv,    // args: [path_str]
    LoadJson,   // args: [path_str]
    Filter,     // args: [df, predicate_expr]
    Select,     // args: [df, col_name...]
    GroupBy,    // args: [df, col_name]
    Agg,        // args: [grouped_df, agg_fn]
    Join,       // args: [df_left, df_right, on_col]
    Sort,       // args: [df, col_name, descending_bool]
    WithColumn, // args: [df, name_str, expr]
    Collect,    // args: [lazy_df]
    ToJson,     // args: [df]
}

/// Array/APL operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArrayOpKind {
    Shape,        // monadic: shape of array
    Reshape,      // dyadic: reshape lhs to shape rhs
    Rank,         // monadic: rank (number of dimensions)
    Reduce,       // dyadic: f/ a  (reduce with function)
    Scan,         // dyadic: f\ a  (prefix scan)
    OuterProduct, // triadic: f∘.  (outer product)
    InnerProduct, // triadic: f.g  (inner product)
    Rotate,       // dyadic: n⌽a
    Take,         // dyadic: n↑a
    Drop,         // dyadic: n↓a
    Iota,         // monadic: ⍳n
    Ravel,        // monadic: ,a (flatten to vector)
    Transpose,    // monadic: ⍉a
    Enclose,      // monadic: ⊂a
    Each,         // dyadic: f¨ a
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
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Concat, // string/array concatenation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnOpKind {
    Neg,
    Not,
    Ref,
    Deref,
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
    Struct {
        fields: Vec<(String, IrType)>,
    },
    Enum {
        variants: Vec<(String, Option<IrType>)>,
    },
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
        self.functions
            .iter()
            .filter(|f| self.exports.contains(&f.name))
            .collect()
    }
}

// ── Builder helpers ───────────────────────────────────────────────────────────

impl IrOp {
    pub fn lit_i64(n: i64) -> Self {
        Self::Lit(IrLit::I64(n))
    }
    pub fn lit_str(s: impl Into<String>) -> Self {
        Self::Lit(IrLit::Str(s.into()))
    }
    pub fn lit_bool(b: bool) -> Self {
        Self::Lit(IrLit::Bool(b))
    }
    pub fn unit() -> Self {
        Self::Lit(IrLit::Unit)
    }
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var(name.into())
    }

    pub fn block(ops: impl IntoIterator<Item = IrOp>) -> Self {
        let v: Vec<IrOp> = ops.into_iter().collect();
        if v.len() == 1 {
            v.into_iter().next().unwrap()
        } else {
            Self::Block(v)
        }
    }

    pub fn if_(cond: IrOp, then: IrOp, else_: IrOp) -> Self {
        Self::If {
            cond: Box::new(cond),
            then: Box::new(then),
            else_: Box::new(else_),
        }
    }

    pub fn apply(func: IrOp, args: Vec<IrOp>) -> Self {
        Self::Apply {
            func: Box::new(func),
            args,
        }
    }

    pub fn tool_call(name: impl Into<String>, args: IrOp) -> Self {
        Self::ToolCall {
            tool_name: name.into(),
            args: Box::new(args),
        }
    }

    pub fn spawn(actor_ty: IrType, init_msg: IrOp) -> Self {
        Self::Spawn {
            actor_ty,
            init_msg: Box::new(init_msg),
        }
    }

    pub fn send(actor_ref: IrOp, msg: IrOp) -> Self {
        Self::Send {
            actor_ref: Box::new(actor_ref),
            msg: Box::new(msg),
        }
    }

    pub fn on_device(target: DeviceTarget, expr: IrOp) -> Self {
        Self::DeviceAnnotation {
            target,
            expr: Box::new(expr),
        }
    }

    pub fn sql(query: impl Into<String>, params: Vec<IrOp>) -> Self {
        Self::SqlQuery {
            query: query.into(),
            params,
        }
    }
}

impl IrType {
    /// Unrestricted function type (most common case).
    pub fn fun(params: Vec<IrType>, ret: IrType) -> Self {
        Self::Fn {
            params,
            ret: Box::new(ret),
            modality: Modality::Many,
        }
    }
    /// Linear function type (consumes argument exactly once).
    pub fn linear_fun(params: Vec<IrType>, ret: IrType) -> Self {
        Self::Fn {
            params,
            ret: Box::new(ret),
            modality: Modality::One,
        }
    }
    pub fn actor_ref(msg_ty: IrType) -> Self {
        Self::ActorRef(Box::new(msg_ty))
    }
    pub fn array_of(elem: IrType) -> Self {
        Self::Array(Box::new(elem))
    }
    pub fn ndarray_of(elem: IrType) -> Self {
        Self::NDArray(Box::new(elem))
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
