//! Error types for the array/APL evaluator

/// Errors produced by [`crate::array::NdArray`] operations and the
/// [`crate::eval::AplEval`] tokenizer/parser
#[derive(Debug, Clone)]
pub enum ArrayError {
    /// Shapes/lengths are incompatible for the requested operation
    LengthError { left: Vec<usize>, right: Vec<usize> },
    /// Rank mismatch (e.g. an operation that requires matching ranks)
    RankError { expected: usize, got: usize },
    /// A value is outside the domain of the requested operation
    DomainError(String),
    /// A tokenizer/parser error in an APL/J expression string
    SyntaxError(String),
}

impl std::fmt::Display for ArrayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArrayError::LengthError { left, right } => {
                write!(f, "length error: shapes {:?} and {:?} are incompatible", left, right)
            }
            ArrayError::RankError { expected, got } => {
                write!(f, "rank error: expected rank {}, got {}", expected, got)
            }
            ArrayError::DomainError(msg) => write!(f, "domain error: {}", msg),
            ArrayError::SyntaxError(msg) => write!(f, "syntax error: {}", msg),
        }
    }
}

impl std::error::Error for ArrayError {}

/// Result type
pub type ArrayResult<T> = std::result::Result<T, ArrayError>;
