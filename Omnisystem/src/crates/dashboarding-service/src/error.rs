//! Error types

#[derive(Debug, Clone)]
pub enum DashboardError {
    /// Referenced dashboard does not exist
    DashboardNotFound,
    /// Referenced widget does not exist
    WidgetNotFound,
    /// Other error
    Other(String),
}

impl std::fmt::Display for DashboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DashboardError::DashboardNotFound => write!(f, "dashboard not found"),
            DashboardError::WidgetNotFound => write!(f, "widget not found"),
            DashboardError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for DashboardError {}

/// Result type
pub type DashboardResult<T> = std::result::Result<T, DashboardError>;
