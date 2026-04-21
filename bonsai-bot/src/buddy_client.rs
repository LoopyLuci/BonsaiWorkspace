use std::sync::Arc;
use std::time::Duration;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::sleep;

use crate::health::CircuitBreaker;
use crate::metrics::SharedMetrics;

pub struct BuddyClient {
    http:    Client,
    base:    String,
    breaker: Arc<CircuitBreaker>,
    metrics: SharedMetrics,
}

impl BuddyClient {
    pub fn new(base_url: String, breaker: Arc<CircuitBreaker>, metrics: SharedMetrics) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest client");
        Self { http, base: base_url, breaker, metrics }
    }

    /// Send a chat/completions request to Buddy. Returns the raw JSON response.
    pub async fn chat(&self, body: Value) -> Result<Value, String> {
        if self.breaker.is_open() {
            return Err("circuit_open".to_string());
        }

        let url = format!("{}/v1/chat/completions", self.base);
        let mut last_err = String::new();

        for attempt in 0u32..3 {
            if attempt > 0 {
                sleep(Duration::from_secs(2u64.pow(attempt - 1))).await;
            }

            self.metrics.buddy_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            match self.http.post(&url).json(&body).send().await {
                Err(e) => {
                    last_err = e.to_string();
                    self.metrics.buddy_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    self.breaker.record_failure();
                    continue;
                }
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        match resp.json::<Value>().await {
                            Ok(v) => {
                                self.breaker.record_success();
                                return Ok(v);
                            }
                            Err(e) => {
                                last_err = format!("json parse: {e}");
                                self.breaker.record_failure();
                                continue;
                            }
                        }
                    } else {
                        last_err = format!("HTTP {status}");
                        self.metrics.buddy_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        self.breaker.record_failure();
                        continue;
                    }
                }
            }
        }

        Err(last_err)
    }

    /// Build an OpenAI-compatible chat request body from a list of messages.
    pub fn build_request(messages: Vec<Value>, model: Option<&str>) -> Value {
        json!({
            "model": model.unwrap_or("bonsai-buddy"),
            "messages": messages,
            "stream": false,
        })
    }

    /// Check if Buddy /health is reachable.
    #[allow(dead_code)]
    pub async fn is_healthy(&self) -> bool {
        let url = format!("{}/health", self.base);
        self.http.get(&url).send().await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}
