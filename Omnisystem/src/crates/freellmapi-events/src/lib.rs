//! FreeLLMAPI Events - append-only event log with type-filtered queries and an
//! HTTP webhook dispatcher, plus a higher-level `EventService` that ties the two
//! together and implements the `OmnisystemService` trait from `freellmapi-core`.

pub mod event_log;
pub mod service;
pub mod types;
pub mod webhook;

pub use event_log::{EventLog, EventRecord, InMemoryEventLog};
pub use service::EventService;
pub use types::EventType;
pub use webhook::{HttpWebhookDispatcher, WebhookDelivery, WebhookDispatcher, WebhookStatus};
