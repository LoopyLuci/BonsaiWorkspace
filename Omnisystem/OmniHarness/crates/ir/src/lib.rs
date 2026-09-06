//! UniIR — Universal Intermediate Representation.
//!
//! `parser` turns Sylva-subset surface syntax into an `IrModule`, `ops` defines
//! the typed IR itself, `effects` is the capability effect system, and
//! `codegen` lowers an `IrModule` to Rust source. `IrCompiler` wires parse and
//! codegen together into a single cached pipeline.

pub mod codegen;
pub mod core;
pub mod effects;
pub mod error;
pub mod ops;
pub mod parser;
pub mod titan_lower;
pub mod types;

pub use codegen::{Codegen, CodegenError, RustCodegen};
pub use core::Core;
pub use effects::{BonsaiEffect, EffectPolicy, TrustGuard, TrustLevel};
pub use error::{Error, Result};
pub use ir_repr::Ir;
pub use ops::{IrFunction, IrModule, IrOp, IrType};
pub use parser::{parse, parse_expr, ParseError};
pub use titan_lower::{lower_source as lower_titan, LowerError as TitanLowerError};
pub use types::State;

/// Parses Sylva-subset source and lowers it to Rust, caching generated
/// output keyed by (module_name, source) so identical input skips the
/// parse + codegen pipeline on repeat calls.
pub struct IrCompiler {
    cache: Core,
}

impl IrCompiler {
    pub fn new() -> Self {
        Self { cache: Core::new() }
    }

    /// Parse `src` and lower it directly to Rust source.
    pub fn compile_to_rust(&self, src: &str, module_name: &str) -> Result<String> {
        let key = format!("{module_name}\0{src}");
        if let Some(cached) = self.cache.get(&key) {
            return Ok(cached);
        }

        let module = parse(src, module_name)?;
        let rust_src = RustCodegen::new().emit_module(&module)?;

        self.cache.set(key, rust_src.clone());
        Ok(rust_src)
    }

    /// Parse Titan source (the `titan`/`bootstrap-rs` surface language) and
    /// lower it directly to Rust source, through the same `IrModule` and
    /// `RustCodegen` the Sylva-subset path uses. Covers only the subset
    /// documented in `titan_lower` — see that module before assuming a
    /// Titan feature is handled.
    pub fn compile_titan_to_rust(&self, src: &str, file: &str, module_name: &str) -> Result<String> {
        let key = format!("titan\0{module_name}\0{src}");
        if let Some(cached) = self.cache.get(&key) {
            return Ok(cached);
        }

        let module = titan_lower::lower_source(src, file, module_name)?;
        let rust_src = RustCodegen::new().emit_module(&module)?;

        self.cache.set(key, rust_src.clone());
        Ok(rust_src)
    }

    /// Status snapshot — number of cached compilations plus a timestamp.
    pub fn stats(&self) -> State {
        State::now(format!("ok: {} cached modules", self.cache.len()))
    }
}

impl Default for IrCompiler {
    fn default() -> Self {
        Self::new()
    }
}
