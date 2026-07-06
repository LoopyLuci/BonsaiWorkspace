//! Library facade over the existing modules so a second binary (the LSP)
//! can reuse the real lexer/parser/diagnostics without duplicating them.
pub mod ast;
pub mod diag;
pub mod lexer;
pub mod parser;
