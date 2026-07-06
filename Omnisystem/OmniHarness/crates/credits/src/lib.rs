pub mod urv;
pub mod meter;
pub mod manager;
pub mod community;
pub mod estimator;
pub mod ledger;

/// Top-level error type for the credits crate.
#[derive(Debug)]
pub enum CreditError {
    Db(rusqlite::Error),
    Io(std::io::Error),
    Serde(serde_json::Error),
    InvalidArgument(String),
}

impl std::fmt::Display for CreditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreditError::Db(e) => write!(f, "database error: {}", e),
            CreditError::Io(e) => write!(f, "io error: {}", e),
            CreditError::Serde(e) => write!(f, "serde error: {}", e),
            CreditError::InvalidArgument(s) => write!(f, "invalid argument: {}", s),
        }
    }
}

impl std::error::Error for CreditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CreditError::Db(e) => Some(e),
            CreditError::Io(e) => Some(e),
            CreditError::Serde(e) => Some(e),
            CreditError::InvalidArgument(_) => None,
        }
    }
}

impl From<rusqlite::Error> for CreditError {
    fn from(e: rusqlite::Error) -> Self {
        CreditError::Db(e)
    }
}

impl From<std::io::Error> for CreditError {
    fn from(e: std::io::Error) -> Self {
        CreditError::Io(e)
    }
}

impl From<serde_json::Error> for CreditError {
    fn from(e: serde_json::Error) -> Self {
        CreditError::Serde(e)
    }
}
