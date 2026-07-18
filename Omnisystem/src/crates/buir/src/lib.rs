//! BUIR - Basic Unified Intermediate Representation
//!
//! A language-agnostic intermediate representation used to move compiled
//! function/module definitions between the Omni-Languages (and common
//! host languages like Rust, Python, C, Go, ...) through a single SSA-based
//! IR. Modules can be content-hashed (via `hash`) and serialized to/from
//! bytes (via `serialize`) so they can be cached, diffed, or shipped across
//! process boundaries.

pub mod core;
pub mod error;
pub mod hash;
pub mod ir;
pub mod serialize;
pub mod types;

pub use core::Core;
pub use error::{Error, Result};
pub use hash::{hash_buir, hash_function, FunctionHash};
pub use ir::{
    BasicBlock, BuirFunction, BuirGlobal, BuirModule, BuirType, EffectSet, Instruction, Language,
    SsaBody, Terminator, Value,
};
pub use serialize::{deserialize_from_bytes, serialize_to_bytes};
pub use types::State;
