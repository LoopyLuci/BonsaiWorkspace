use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ArrayError {
    #[error("rank error: expected rank {expected}, got {got}")]
    RankError { expected: usize, got: usize },
    #[error("length error: shapes {left:?} and {right:?} are not conformable")]
    LengthError { left: Vec<usize>, right: Vec<usize> },
    #[error("domain error: {0}")]
    DomainError(String),
    #[error("index error: index {idx} out of bounds for axis of length {len}")]
    IndexError { idx: i64, len: usize },
    #[error("syntax error: {0}")]
    SyntaxError(String),
    #[error("value error: {0}")]
    ValueError(String),
}
