//! FreeLLMAPI Rate Limit - per-tenant/per-model sliding-window RPM and TPM limiters.

pub mod service;

pub use service::RateLimitService;
