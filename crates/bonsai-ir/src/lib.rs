//! Bonsai IR — intermediate representation and effect system.

pub mod codegen;
pub mod effects;
pub mod ops;
pub mod parser;

pub use ops::{
    IrModule, IrFunction, IrParam, IrType, IrOp, IrLit, IrPattern,
    IrTypeDef, IrTypeDefKind, IrProof,
    EffectType, BinOpKind, UnOpKind, EffectHandler,
    Modality, DeviceTarget, DataFrameOpKind, ArrayOpKind,
};
pub use parser::{parse, parse_expr, ParseError, ParseResult};
