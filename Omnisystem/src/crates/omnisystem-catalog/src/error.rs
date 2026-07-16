//! Error types for the module catalog.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// An entry with this id is already registered.
    AlreadyExists(String),
    /// No entry with this id was found.
    NotFound(String),
    /// Any other catalog error.
    Other(String),
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::AlreadyExists(id) => write!(f, "catalog entry already exists: {}", id),
            CatalogError::NotFound(id) => write!(f, "catalog entry not found: {}", id),
            CatalogError::Other(msg) => write!(f, "catalog error: {}", msg),
        }
    }
}

impl std::error::Error for CatalogError {}

/// Result type used throughout the catalog.
pub type Result<T> = std::result::Result<T, CatalogError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_messages() {
        assert_eq!(
            CatalogError::NotFound("abc".into()).to_string(),
            "catalog entry not found: abc"
        );
    }
}
