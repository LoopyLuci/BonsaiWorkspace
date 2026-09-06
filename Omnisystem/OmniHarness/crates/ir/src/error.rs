//! Unified error type for the two-stage parse -> codegen pipeline.

use crate::codegen::CodegenError;
use crate::parser::ParseError;
use crate::titan_lower::LowerError as TitanLowerError;

#[derive(Debug, Clone)]
pub enum Error {
    /// Surface syntax failed to parse into an `IrModule`.
    Parse(String),
    /// A parsed `IrModule` failed to lower to Rust source.
    Codegen(String),
    /// Anything that doesn't fit the above.
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Parse(msg) => write!(f, "parse error: {}", msg),
            Error::Codegen(msg) => write!(f, "codegen error: {}", msg),
            Error::Other(msg) => write!(f, "error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::Parse(e.to_string())
    }
}

impl From<CodegenError> for Error {
    fn from(e: CodegenError) -> Self {
        Error::Codegen(e.to_string())
    }
}

impl From<TitanLowerError> for Error {
    fn from(e: TitanLowerError) -> Self {
        Error::Parse(e.to_string())
    }
}

/// Result type used by the crate-level compile pipeline.
pub type Result<T> = std::result::Result<T, Error>;
