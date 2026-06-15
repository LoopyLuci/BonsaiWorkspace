// OMNISYSTEM ADVANCED FRAMEWORK FEATURES
// Production-grade utilities and extensions

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant};

// ============================================================================
// RATE LIMITING & THROTTLING
// ============================================================================

pub struct RateLimiter {
    pub max_requests: usize,
    pub window: Duration,
    pub requests: Arc<Mutex<VecDeque<Instant>>>,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        RateLimiter {
            max_requests,
            window: Duration::from_secs(window_secs),
            requests: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn allow_request(&self) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();

        // Remove old requests outside the window
        while let Some(&req_time) = requests.front() {
            if now.duration_since(req_time) > self.window {
                requests.pop_front();
            } else {
                break;
            }
        }

        if requests.len() < self.max_requests {
            requests.push_back(now);
            println!("✅ Request allowed ({}/{})", requests.len(), self.max_requests);
            true
        } else {
            println!("❌ Rate limit exceeded");
            false
        }
    }

    pub fn current_rate(&self) -> f64 {
        let requests = self.requests.lock().unwrap();
        requests.len() as f64 / self.window.as_secs_f64()
    }
}

// ============================================================================
// RETRY LOGIC WITH BACKOFF
// ============================================================================

pub enum BackoffStrategy {
    Linear,
    Exponential,
    Fibonacci,
}

pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub strategy: BackoffStrategy,
}

impl RetryPolicy {
    pub fn new(max_attempts: u32) -> Self {
        RetryPolicy {
            max_attempts,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            strategy: BackoffStrategy::Exponential,
        }
    }

    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = match self.strategy {
            BackoffStrategy::Linear => self.initial_delay.as_millis() as u32 * attempt,
            BackoffStrategy::Exponential => {
                self.initial_delay.as_millis() as u32 * 2_u32.pow(attempt)
            }
            BackoffStrategy::Fibonacci => {
                let mut a = 1u32;
                let mut b = 1u32;
                for _ in 0..attempt {
                    let temp = a + b;
                    a = b;
                    b = temp;
                }
                self.initial_delay.as_millis() as u32 * a
            }
        };

        Duration::from_millis(delay_ms.min(self.max_delay.as_millis() as u32) as u64)
    }

    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

// ============================================================================
// REQUEST TRACING & CORRELATION
// ============================================================================

pub struct RequestTracer {
    pub traces: Arc<RwLock<HashMap<String, RequestTrace>>>,
}

pub struct RequestTrace {
    pub id: String,
    pub parent_id: Option<String>,
    pub events: Vec<TraceEvent>,
    pub start_time: Instant,
}

pub struct TraceEvent {
    pub name: String,
    pub timestamp: Instant,
    pub duration: Option<Duration>,
    pub metadata: HashMap<String, String>,
}

impl RequestTracer {
    pub fn new() -> Self {
        RequestTracer {
            traces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn start_trace(&self, id: &str) -> RequestTrace {
        RequestTrace {
            id: id.to_string(),
            parent_id: None,
            events: Vec::new(),
            start_time: Instant::now(),
        }
    }

    pub fn add_event(&self, trace_id: &str, event_name: &str) {
        let event = TraceEvent {
            name: event_name.to_string(),
            timestamp: Instant::now(),
            duration: None,
            metadata: HashMap::new(),
        };

        println!("📝 Trace event: {} -> {}", trace_id, event_name);
    }

    pub fn get_trace(&self, id: &str) -> Option<RequestTrace> {
        self.traces.read().unwrap().get(id).cloned()
    }
}

// ============================================================================
// REQUEST/RESPONSE VALIDATION
// ============================================================================

pub struct ValidationRule {
    pub name: String,
    pub validator: Arc<dyn Fn(&str) -> bool + Send + Sync>,
}

pub struct RequestValidator {
    pub rules: HashMap<String, ValidationRule>,
}

impl RequestValidator {
    pub fn new() -> Self {
        RequestValidator {
            rules: HashMap::new(),
        }
    }

    pub fn add_rule<F>(&mut self, name: &str, validator: F)
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.rules.insert(
            name.to_string(),
            ValidationRule {
                name: name.to_string(),
                validator: Arc::new(validator),
            },
        );
        println!("✅ Validation rule added: {}", name);
    }

    pub fn validate(&self, data: &str) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for (name, rule) in &self.rules {
            if !(rule.validator)(data) {
                errors.push(format!("Validation failed: {}", name));
            }
        }

        if errors.is_empty() {
            println!("✅ All validations passed");
            Ok(())
        } else {
            println!("❌ Validation errors: {}", errors.len());
            Err(errors)
        }
    }
}

// ============================================================================
// METRICS & OBSERVABILITY
// ============================================================================

pub struct MetricsCollector {
    pub counters: Arc<RwLock<HashMap<String, u64>>>,
    pub gauges: Arc<RwLock<HashMap<String, f64>>>,
    pub histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        MetricsCollector {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn increment_counter(&self, name: &str) {
        let mut counters = self.counters.write().unwrap();
        *counters.entry(name.to_string()).or_insert(0) += 1;
    }

    pub fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write().unwrap();
        gauges.insert(name.to_string(), value);
    }

    pub fn record_histogram(&self, name: &str, value: f64) {
        let mut histograms = self.histograms.write().unwrap();
        histograms.entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(value);
    }

    pub fn get_metrics_summary(&self) -> MetricsSummary {
        let counters = self.counters.read().unwrap().clone();
        let gauges = self.gauges.read().unwrap().clone();
        let histograms = self.histograms.read().unwrap();

        let mut histogram_stats = HashMap::new();
        for (name, values) in histograms.iter() {
            if !values.is_empty() {
                let sum: f64 = values.iter().sum();
                let mean = sum / values.len() as f64;
                histogram_stats.insert(name.clone(), mean);
            }
        }

        MetricsSummary {
            counter_count: counters.len(),
            gauge_count: gauges.len(),
            histogram_count: histogram_stats.len(),
        }
    }
}

pub struct MetricsSummary {
    pub counter_count: usize,
    pub gauge_count: usize,
    pub histogram_count: usize,
}

// ============================================================================
// DEPENDENCY INJECTION
// ============================================================================

pub struct ServiceContainer {
    pub services: Arc<RwLock<HashMap<String, Arc<dyn std::any::Any + Send + Sync>>>>,
}

impl ServiceContainer {
    pub fn new() -> Self {
        ServiceContainer {
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register<T: 'static + Send + Sync>(&self, name: &str, service: T) {
        let mut services = self.services.write().unwrap();
        services.insert(name.to_string(), Arc::new(service));
        println!("✅ Service registered: {}", name);
    }

    pub fn get_service_names(&self) -> Vec<String> {
        self.services.read().unwrap().keys().cloned().collect()
    }
}

// ============================================================================
// CONFIGURATION MANAGEMENT
// ============================================================================

pub struct ConfigBuilder {
    pub config: HashMap<String, String>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        ConfigBuilder {
            config: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.config.insert(key.to_string(), value.to_string());
        self
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.config.get(key).cloned()
    }

    pub fn build(self) -> Config {
        Config {
            values: self.config,
        }
    }
}

pub struct Config {
    pub values: HashMap<String, String>,
}

impl Config {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let required_keys = vec!["app_name", "environment"];
        let mut missing = Vec::new();

        for key in required_keys {
            if !self.values.contains_key(key) {
                missing.push(key.to_string());
            }
        }

        if missing.is_empty() {
            println!("✅ Configuration validated");
            Ok(())
        } else {
            Err(missing)
        }
    }
}

// ============================================================================
// EXAMPLE USAGE
// ============================================================================

pub fn example_advanced_features() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚀 ADVANCED FRAMEWORK FEATURES EXAMPLE\n");

    // Rate Limiting
    println!("1️⃣  Rate Limiting:");
    let limiter = RateLimiter::new(10, 60);
    for i in 0..12 {
        limiter.allow_request();
        println!("  Request {}: rate = {:.2} req/s", i + 1, limiter.current_rate());
    }
    println!();

    // Retry Policy
    println!("2️⃣  Retry with Backoff:");
    let policy = RetryPolicy::new(5);
    for attempt in 0..5 {
        let delay = policy.calculate_delay(attempt);
        println!("  Attempt {}: delay = {:?}", attempt + 1, delay);
    }
    println!();

    // Request Tracing
    println!("3️⃣  Request Tracing:");
    let tracer = RequestTracer::new();
    tracer.add_event("req-123", "start");
    tracer.add_event("req-123", "processing");
    tracer.add_event("req-123", "complete");
    println!();

    // Validation
    println!("4️⃣  Request Validation:");
    let mut validator = RequestValidator::new();
    validator.add_rule("length", |s| s.len() > 0);
    validator.add_rule("contains_data", |s| s.contains("data"));
    let result = validator.validate("test data");
    println!();

    // Metrics
    println!("5️⃣  Metrics Collection:");
    let metrics = MetricsCollector::new();
    metrics.increment_counter("requests");
    metrics.increment_counter("requests");
    metrics.set_gauge("cpu_usage", 45.2);
    metrics.record_histogram("response_time", 123.5);
    let summary = metrics.get_metrics_summary();
    println!("  Metrics: {} counters, {} gauges, {} histograms",
        summary.counter_count, summary.gauge_count, summary.histogram_count);
    println!();

    // Configuration
    println!("6️⃣  Configuration Management:");
    let mut config_builder = ConfigBuilder::new();
    config_builder
        .set("app_name", "Omnisystem")
        .set("environment", "production")
        .set("port", "8080");
    let config = config_builder.build();
    config.validate()?;
    println!();

    println!("✅ Advanced Features Example Complete\n");
    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(5, 1);
        assert!(limiter.allow_request());
        assert!(limiter.allow_request());
        assert!(limiter.allow_request());
        assert!(limiter.allow_request());
        assert!(limiter.allow_request());
        assert!(!limiter.allow_request());
    }

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::new(3);
        assert!(policy.should_retry(0));
        assert!(policy.should_retry(1));
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));
    }

    #[test]
    fn test_validator() {
        let mut validator = RequestValidator::new();
        validator.add_rule("not_empty", |s| !s.is_empty());
        assert!(validator.validate("test").is_ok());
        assert!(validator.validate("").is_err());
    }

    #[test]
    fn test_metrics() {
        let metrics = MetricsCollector::new();
        metrics.increment_counter("test");
        metrics.set_gauge("gauge", 42.0);
        let summary = metrics.get_metrics_summary();
        assert_eq!(summary.counter_count, 1);
        assert_eq!(summary.gauge_count, 1);
    }

    #[test]
    fn test_config() {
        let mut builder = ConfigBuilder::new();
        builder.set("app_name", "test").set("environment", "test");
        let config = builder.build();
        assert!(config.validate().is_ok());
    }
}
