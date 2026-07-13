//! Parsing error types shared between NL and keyword parsers

use serde::{Deserialize, Serialize};

/// Errors that can occur during parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParseError {
    /// Input was empty
    EmptyInput,
    /// Unrecognized command or intent
    UnrecognizedCommand { input: String },
    /// Intent confidence was below threshold
    LowConfidence { confidence: f64, threshold: f64 },
    /// Missing required parameter
    MissingParameter(String),
    /// Invalid parameter value
    InvalidParameter { name: String, value: String },
    /// Syntax error in keyword parsing
    SyntaxError { message: String },
    /// Generic parsing error
    ParseFailed { reason: String },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::EmptyInput => write!(f, "Input cannot be empty"),
            ParseError::UnrecognizedCommand { input } => {
                write!(f, "Unrecognized command: '{}'", input)
            }
            ParseError::LowConfidence {
                confidence,
                threshold,
            } => {
                write!(
                    f,
                    "NL confidence {:.2} below threshold {:.2}",
                    confidence, threshold
                )
            }
            ParseError::MissingParameter(name) => {
                write!(f, "Missing required parameter: {}", name)
            }
            ParseError::InvalidParameter { name, value } => {
                write!(f, "Invalid parameter '{}' with value '{}'", name, value)
            }
            ParseError::SyntaxError { message } => {
                write!(f, "Syntax error: {}", message)
            }
            ParseError::ParseFailed { reason } => {
                write!(f, "Parse failed: {}", reason)
            }
        }
    }
}

impl std::error::Error for ParseError {}
