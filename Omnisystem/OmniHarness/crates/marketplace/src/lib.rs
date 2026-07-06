pub mod registry;
pub mod listing;
pub mod reservation;
pub mod free_tier;

pub use registry::DeviceRegistry;
pub use reservation::ReservationManager;
pub use free_tier::FreeTierOrchestrator;

use std::sync::Arc;

/// Top-level error type for the marketplace crate.
#[derive(Debug)]
pub enum MarketplaceError {
    NotFound(uuid::Uuid),
    AlreadyExists(uuid::Uuid),
    InvalidArgument(String),
    Credits(credits::CreditError),
}

impl std::fmt::Display for MarketplaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketplaceError::NotFound(id) => write!(f, "not found: {}", id),
            MarketplaceError::AlreadyExists(id) => write!(f, "already exists: {}", id),
            MarketplaceError::InvalidArgument(s) => write!(f, "invalid argument: {}", s),
            MarketplaceError::Credits(e) => write!(f, "credit error: {}", e),
        }
    }
}

impl std::error::Error for MarketplaceError {}

impl From<credits::CreditError> for MarketplaceError {
    fn from(e: credits::CreditError) -> Self {
        MarketplaceError::Credits(e)
    }
}

/// Composite state object suitable for storage in Tauri app state.
pub struct MarketplaceState {
    pub registry: Arc<DeviceRegistry>,
    pub reservations: Arc<ReservationManager>,
    pub free_tier: Arc<FreeTierOrchestrator>,
}

impl MarketplaceState {
    pub fn new() -> Self {
        let registry = Arc::new(DeviceRegistry::new());
        let reservations = Arc::new(ReservationManager::new());
        let free_tier = Arc::new(FreeTierOrchestrator::new(Arc::clone(&registry)));
        MarketplaceState {
            registry,
            reservations,
            free_tier,
        }
    }
}

impl Default for MarketplaceState {
    fn default() -> Self {
        Self::new()
    }
}
