//! Event type taxonomy for the FreeLLMAPI event log and webhook system.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    RequestStarted,
    RequestCompleted,
    RequestFailed,
    RateLimitExceeded,
    BudgetExceeded,
    WebhookDelivered,
    WebhookFailed,
}
