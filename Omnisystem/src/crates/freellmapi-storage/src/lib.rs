//! FreeLLMAPI Storage - SQLite-backed persistence for tenants, API keys, request
//! logs, and webhooks, implementing the `StorageRepository` trait from
//! `freellmapi-core`.

pub mod db;
pub mod repository;

pub use db::StorageManager;
