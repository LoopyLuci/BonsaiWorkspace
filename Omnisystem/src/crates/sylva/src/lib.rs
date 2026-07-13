//! Sylva — an embedded scripting language: lexer, parser, tree-walk VM, and stdlib.

pub mod ast;
pub mod debugger;
pub mod lexer;
pub mod parser;
pub mod stdlib;
pub mod vm;

pub use ast::{Expr, ExprKind, Item, SylvaModule, SylvaType};
pub use debugger::Debugger;
pub use lexer::{lex, LexError, Token};
pub use parser::{parse_expr, parse_module, ParseError};
pub use vm::{SylvaValue, SylvaVm, VmError};
