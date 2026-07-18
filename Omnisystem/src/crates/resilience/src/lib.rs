//! resilience
//!
//! Resilience patterns for distributed systems: circuit breaker, retry with
//! backoff, bulkhead isolation, backpressure control, and timeout enforcement.

pub mod backpressure;
pub mod bulkhead;
pub mod circuit_breaker;
pub mod core;
pub mod error;
pub mod retry;
pub mod timeout;
pub mod types;

pub use backpressure::{BackpressureController, BackpressureError, BackpressureGuard, BackpressureState};
pub use bulkhead::{Bulkhead, BulkheadError, BulkheadGuard, BulkheadState};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerDiagnostics, CircuitState};
pub use core::Core;
pub use error::{Error, Result};
pub use retry::{RetryContext, RetryPolicy, RetryStrategy};
pub use timeout::{TimeoutEnforcer, TimeoutPolicy};
pub use types::State;
