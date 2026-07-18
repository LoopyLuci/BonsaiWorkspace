//! CLI demo: exercises a CircuitBreaker and a RetryPolicy together.

use resilience::{CircuitBreaker, RetryPolicy};

fn main() {
    let cb = CircuitBreaker::new(3, 2, 30);
    println!("Circuit breaker initial state: {}", cb.state());

    for i in 1..=3 {
        cb.record_failure();
        println!("Recorded failure {} -> state: {}", i, cb.state());
    }

    println!("Can execute? {}", cb.can_execute());

    let retry = RetryPolicy::exponential(5, 100, 5000);
    for attempt in 1..=4 {
        let backoff = retry.backoff_for_attempt(attempt);
        println!(
            "Retry attempt {}: backoff = {}ms, should_retry = {}",
            attempt,
            backoff.as_millis(),
            retry.should_retry(attempt)
        );
    }

    let diagnostics = cb.diagnostics();
    println!(
        "Diagnostics: failures={} successes={} threshold={}",
        diagnostics.failure_count, diagnostics.success_count, diagnostics.failure_threshold
    );
}
