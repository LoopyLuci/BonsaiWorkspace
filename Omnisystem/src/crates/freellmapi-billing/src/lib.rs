//! FreeLLMAPI Billing - per-model pricing, cost calculation, budget enforcement,
//! and usage recording against the `StorageRepository` trait from `freellmapi-core`.

pub mod service;

pub use service::BillingService;
