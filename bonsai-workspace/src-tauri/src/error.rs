//! Unified error type for Bonsai Workspace Tauri commands.
//!
//! `BonsaiError` implements `serde::Serialize` so it can be returned directly
//! from `#[tauri::command]` functions as a structured JSON error payload
//! instead of an opaque `String`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Structured error type for all Tauri command boundaries.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum BonsaiError {
    /// Filesystem or OS-level I/O failure.
    Io(String),
    /// JSON / TOML / YAML (de)serialization failure.
    Serde(String),
    /// Model loading, slot management, or inference failure.
    Orchestrator(String),
    /// Tool execution failure (built-in or user-defined).
    Tool(String),
    /// Model registry lookup or metadata failure.
    Model(String),
    /// Authentication or keychain failure.
    Auth(String),
    /// Network or HTTP call failure.
    Network(String),
    /// MCP bridge or MCP server failure.
    Mcp(String),
    /// Configuration parsing or validation failure.
    Config(String),
    /// Database / WAL failure.
    Database(String),
    /// Request cancelled by the user.
    Cancelled(String),
    /// Any error that does not fit a specific category.
    Internal(String),
}

impl fmt::Display for BonsaiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(m)           => write!(f, "I/O error: {m}"),
            Self::Serde(m)        => write!(f, "Serialization error: {m}"),
            Self::Orchestrator(m) => write!(f, "Orchestrator error: {m}"),
            Self::Tool(m)         => write!(f, "Tool error: {m}"),
            Self::Model(m)        => write!(f, "Model error: {m}"),
            Self::Auth(m)         => write!(f, "Auth error: {m}"),
            Self::Network(m)      => write!(f, "Network error: {m}"),
            Self::Mcp(m)          => write!(f, "MCP error: {m}"),
            Self::Config(m)       => write!(f, "Config error: {m}"),
            Self::Database(m)     => write!(f, "Database error: {m}"),
            Self::Cancelled(m)    => write!(f, "Cancelled: {m}"),
            Self::Internal(m)     => write!(f, "Internal error: {m}"),
        }
    }
}

impl std::error::Error for BonsaiError {}

// ── Conversions from standard error types ────────────────────────────────────

impl From<std::io::Error> for BonsaiError {
    fn from(e: std::io::Error) -> Self { Self::Io(e.to_string()) }
}

impl From<serde_json::Error> for BonsaiError {
    fn from(e: serde_json::Error) -> Self { Self::Serde(e.to_string()) }
}

impl From<reqwest::Error> for BonsaiError {
    fn from(e: reqwest::Error) -> Self { Self::Network(e.to_string()) }
}

/// Convenience: promote any `String` error as `Internal`.
impl From<String> for BonsaiError {
    fn from(s: String) -> Self { Self::Internal(s) }
}

/// Convenience: promote any `&str` error as `Internal`.
impl From<&str> for BonsaiError {
    fn from(s: &str) -> Self { Self::Internal(s.to_owned()) }
}
