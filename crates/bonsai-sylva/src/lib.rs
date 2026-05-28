//! bonsai-sylva — Native Sylva language interpreter.
//!
//! Sylva is Bonsai's interactive scripting language (equivalent to Omnisystem's Sylva layer).
//! This crate implements:
//!   - Lexer (lexer.rs)
//!   - AST (ast.rs)
//!   - Parser (parser.rs)
//!   - Tree-walk interpreter / VM (vm.rs)
//!   - Standard library bindings (stdlib.rs)
//!   - Time-travel debugger (debugger.rs) — snapshot + rewind/replay
//!
//! The VM is designed to be embedded in `SylvaRuntime` (Tauri backend) to replace
//! the Lua VM. It exposes the same `bonsai.tool(name, args)` interface.

pub mod lexer;
pub mod ast;
pub mod parser;
pub mod vm;
pub mod stdlib;
pub mod debugger;

pub use vm::{SylvaVm, SylvaValue, VmError, VmResult};
pub use debugger::{Debugger, Snapshot, RewindError};
pub use parser::ParseError;
