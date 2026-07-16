//! Error types for titan-opencv

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A matrix was requested with zero rows or columns.
    InvalidDimensions { rows: usize, cols: usize },
    /// An unrecognized pixel depth code was supplied.
    UnknownDepth(u8),
    /// A pixel access (get or set) fell outside the matrix bounds.
    IndexOutOfBounds {
        row: usize,
        col: usize,
        channel: u8,
        rows: usize,
        cols: usize,
        channels: u8,
    },
    /// Other error
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidDimensions { rows, cols } => {
                write!(f, "matrix dimensions must be positive, got {}x{}", rows, cols)
            }
            Error::UnknownDepth(depth) => write!(f, "unknown pixel depth: {}", depth),
            Error::IndexOutOfBounds { row, col, channel, rows, cols, channels } => write!(
                f,
                "index out of bounds: ({}, {}, {}) in {}x{}x{}",
                row, col, channel, rows, cols, channels
            ),
            Error::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

/// Result type
pub type Result<T> = std::result::Result<T, Error>;
