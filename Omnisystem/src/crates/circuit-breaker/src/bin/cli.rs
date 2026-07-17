//! CLI demo: call a circuit breaker and inspect its status.

use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let breaker = CircuitBreaker::new(CircuitBreakerConfig {
        breaker_id: "payments-api".to_string(),
        failure_threshold: 3,
        success_threshold: 2,
        timeout_ms: 5000,
        half_open_max_calls: 1,
    });

    let result: CircuitBreakerResult<u32> = breaker.call(async { Ok(42) }).await;
    println!("Call result: {:?}", result);

    let status = breaker.get_status().await?;
    println!("Circuit state: {:?}", status.current_state);

    Ok(())
}
