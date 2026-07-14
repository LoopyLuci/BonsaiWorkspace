//! Error types for aion-agents.

#[derive(Debug, Clone)]
pub enum AgentError {
    /// Catch-all agent error.
    Other(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::Other(msg) => write!(f, "agent error: {}", msg),
        }
    }
}

impl std::error::Error for AgentError {}

/// Result type used throughout aion-agents.
pub type Result<T> = std::result::Result<T, AgentError>;
