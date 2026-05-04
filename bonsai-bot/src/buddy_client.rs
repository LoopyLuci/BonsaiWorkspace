use std::sync::Arc;
use std::time::Duration;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio::time::sleep;
use futures::StreamExt;

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

    /// Send a streaming chat/completions request. Tokens are sent to the returned receiver
    /// as they arrive. The receiver closes when the stream ends or on error.
    pub async fn chat_stream(
        &self,
        body: Value,
    ) -> Result<mpsc::UnboundedReceiver<String>, String> {
        if self.breaker.is_open() {
            return Err("circuit_open".to_string());
        }

        let mut streaming_body = body.clone();
        streaming_body["stream"] = json!(true);

        let url = format!("{}/v1/chat/completions", self.base);
        self.metrics.buddy_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let resp = self.http.post(&url).json(&streaming_body).send().await
            .map_err(|e| { self.breaker.record_failure(); e.to_string() })?;

        if !resp.status().is_success() {
            self.breaker.record_failure();
            return Err(format!("HTTP {}", resp.status()));
        }

        let breaker = self.breaker.clone();
        let (tx, rx) = mpsc::unbounded_channel::<String>();

        tokio::spawn(async move {
            let mut stream = resp.bytes_stream();
            let mut buf = String::new();
            let mut ok = false;

            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(_) => break,
                };
                buf.push_str(&String::from_utf8_lossy(&chunk));

                // SSE lines: "data: {...}\n\n" or "data: [DONE]\n\n"
                while let Some(pos) = buf.find('\n') {
                    let line = buf[..pos].trim().to_string();
                    buf = buf[pos + 1..].to_string();

                    if line == "data: [DONE]" {
                        ok = true;
                        break;
                    }
                    if let Some(json_str) = line.strip_prefix("data: ") {
                        if let Ok(v) = serde_json::from_str::<Value>(json_str) {
                            if let Some(token) = v["choices"][0]["delta"]["content"].as_str() {
                                if !token.is_empty() {
                                    let _ = tx.send(token.to_string());
                                }
                            }
                        }
                    }
                }
            }

            if ok {
                breaker.record_success();
            } else {
                breaker.record_failure();
            }
        });

        Ok(rx)
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
