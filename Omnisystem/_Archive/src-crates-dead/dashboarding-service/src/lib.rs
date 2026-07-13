mod error;
mod types;
mod service;

pub use error::{DashboardError, DashboardResult};
pub use types::{Dashboard, Widget, WidgetType, WidgetData, RealTimeUpdate};
pub use service::DashboardingService;
