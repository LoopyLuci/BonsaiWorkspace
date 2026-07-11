//! Omni-Bot REST/WebSocket API Client
//!
//! Handles communication with Omni-Bot backend via REST API and WebSocket.
//! Includes connection pooling, request/response handling, and subscription management.

use crate::error::{Error, Result};
use async_trait::async_trait;
use chrono::Utc;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use omni_bot_core::{Action, ApiResponse, ServiceInfo, RequestId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_tungstenite::{
    connect_async, tungstenite::Message, WebSocketStream,
};

type WsStream = WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;
type WsSink = SplitSink<WsStream, Message>;
type WsSource = SplitStream<WsStream>;

/// Subscription callback for WebSocket updates
#[async_trait]
pub trait UpdateCallback: Send + Sync {
    async fn on_update(&self, update: ServiceUpdate);
}

/// Service update event from WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceUpdate {
    pub service_name: String,
    pub event_type: String,
    pub data: Value,
    pub timestamp: String,
}

/// API Client configuration
#[derive(Debug, Clone)]
pub struct ApiClientConfig {
    pub base_url: String,
    pub ws_url: String,
    pub timeout_seconds: u64,
    pub max_retries: usize,
}

impl Default for ApiClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8000".to_string(),
            ws_url: "ws://localhost:8000/ws".to_string(),
            timeout_seconds: 30,
            max_retries: 3,
        }
    }
}

/// Omni-Bot API Client
pub struct ApiClient {
    config: ApiClientConfig,
    http_client: reqwest::Client,
    ws_sink: Arc<RwLock<Option<WsSink>>>,
    callbacks: Arc<RwLock<Vec<Arc<dyn UpdateCallback>>>>,
    subscriptions: Arc<RwLock<HashMap<String, bool>>>,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(config: ApiClientConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_default();

        Self {
            config,
            http_client,
            ws_sink: Arc::new(RwLock::new(None)),
            callbacks: Arc::new(RwLock::new(Vec::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to WebSocket for real-time updates
    pub async fn connect_websocket(&self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.config.ws_url)
            .await
            .map_err(|e| Error::WebSocket(e.to_string()))?;

        let (sink, source) = ws_stream.split();
        *self.ws_sink.write().await = Some(sink);

        // Spawn task to handle incoming messages
        let callbacks = Arc::clone(&self.callbacks);
        tokio::spawn(async move {
            let mut source = source;
            while let Some(message) = source.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(update) = serde_json::from_str::<ServiceUpdate>(&text) {
                            for callback in callbacks.read().await.iter() {
                                callback.on_update(update.clone()).await;
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Disconnect from WebSocket
    pub async fn disconnect_websocket(&self) -> Result<()> {
        if let Some(mut sink) = self.ws_sink.write().await.take() {
            sink.close().await
                .map_err(|e| Error::WebSocket(e.to_string()))?;
        }
        Ok(())
    }

    /// Register a callback for updates
    pub async fn register_callback(&self, callback: Arc<dyn UpdateCallback>) {
        self.callbacks.write().await.push(callback);
    }

    /// Get service information
    pub async fn get_service(&self, name: &str) -> Result<ServiceInfo> {
        let url = format!("{}/api/services/{}", self.config.base_url, name);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::ApiRequest(e.to_string()))?;

        let api_response: ApiResponse<ServiceInfo> = response
            .json()
            .await
            .map_err(|e| Error::ApiRequest(e.to_string()))?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| Error::ApiRequest("No data in response".to_string()))
        } else {
            Err(Error::ApiRequest(
                api_response.error.unwrap_or_default(),
            ))
        }
    }

    /// List all services
    pub async fn list_services(&self) -> Result<Vec<ServiceInfo>> {
        let url = format!("{}/api/services", self.config.base_url);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::ApiRequest(e.to_string()))?;

        let api_response: ApiResponse<Vec<ServiceInfo>> = response
            .json()
            .await
            .map_err(|e| Error::ApiRequest(e.to_string()))?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| Error::ApiRequest("No data in response".to_string()))
        } else {
            Err(Error::ApiRequest(
                api_response.error.unwrap_or_default(),
            ))
        }
    }

    /// Start a service
    pub async fn start_service(&self, name: &str, config: Option<Value>) -> Result<ServiceInfo> {
        let url = format!("{}/api/services/{}/start", self.config.base_url, name);
        let body = json!({ "config": config });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::ApiRequest(e.to_string()))?;

        let api_response: ApiResponse<ServiceInfo> = response
            .json()
            .await
            .map_err(|e| Error::ApiRequest(e.to_string()))?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| Error::ApiRequest("No data in response".to_string()))
        } else {
            Err(Error::ApiRequest(
                api_response.error.unwrap_or_default(),
            ))
        }
    }

    /// Stop a service
    pub async fn stop_service(&self, name: &str, force: bool) -> Result<ServiceInfo> {
        let url = format!("{}/api/services/{}/stop", self.config.base_url, name);
        let body = json!({ "force": force });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::ApiRequest(e.to_string()))?;

        let api_response: ApiResponse<ServiceInfo> = response
            .json()
            .await
            .map_err(|e| Error::ApiRequest(e.to_string()))?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| Error::ApiRequest("No data in response".to_string()))
        } else {
            Err(Error::ApiRequest(
                api_response.error.unwrap_or_default(),
            ))
        }
    }

    /// Execute an action
    pub async fn execute_action(&self, action: Action) -> Result<Value> {
        let request_id = RequestId::new();
        let url = format!("{}/api/actions", self.config.base_url);
        let body = json!({
            "request_id": request_id,
            "action": action,
            "timestamp": Utc::now(),
        });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::ApiRequest(e.to_string()))?;

        let api_response: ApiResponse<Value> = response
            .json()
            .await
            .map_err(|e| Error::ApiRequest(e.to_string()))?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| Error::ApiRequest("No data in response".to_string()))
        } else {
            Err(Error::ApiRequest(
                api_response.error.unwrap_or_default(),
            ))
        }
    }

    /// Get search results for modules
    pub async fn search_modules(&self, query: &str) -> Result<Vec<Value>> {
        let url = format!("{}/api/modules/search", self.config.base_url);
        let body = json!({ "query": query });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::ApiRequest(e.to_string()))?;

        let api_response: ApiResponse<Vec<Value>> = response
            .json()
            .await
            .map_err(|e| Error::ApiRequest(e.to_string()))?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| Error::ApiRequest("No data in response".to_string()))
        } else {
            Err(Error::ApiRequest(
                api_response.error.unwrap_or_default(),
            ))
        }
    }

    /// Subscribe to service updates
    pub async fn subscribe_service(&self, name: &str) -> Result<()> {
        self.subscriptions.write().await.insert(name.to_string(), true);

        if let Some(sink) = self.ws_sink.write().await.as_mut() {
            let msg = json!({
                "action": "subscribe",
                "service": name,
            });
            sink.send(Message::Text(msg.to_string()))
                .await
                .map_err(|e| Error::WebSocket(e.to_string()))?;
        }

        Ok(())
    }

    /// Unsubscribe from service updates
    pub async fn unsubscribe_service(&self, name: &str) -> Result<()> {
        self.subscriptions.write().await.remove(name);

        if let Some(sink) = self.ws_sink.write().await.as_mut() {
            let msg = json!({
                "action": "unsubscribe",
                "service": name,
            });
            sink.send(Message::Text(msg.to_string()))
                .await
                .map_err(|e| Error::WebSocket(e.to_string()))?;
        }

        Ok(())
    }

    /// Check API health
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.config.base_url);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .map_err(|e| Error::ApiConnection(e.to_string()))?;

        Ok(response)
    }

    /// Retry logic wrapper for API calls
    async fn retry_call<F, T>(&self, mut f: F) -> Result<T>
    where
        F: FnMut() -> futures::future::BoxFuture<'static, Result<T>>,
    {
        let mut retries = 0;
        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(_e) if retries < self.config.max_retries => {
                    retries += 1;
                    let backoff = std::time::Duration::from_millis(100 * 2_u64.pow(retries as u32));
                    tokio::time::sleep(backoff).await;
                    log::warn!("API call failed, retry {}/{}", retries, self.config.max_retries);
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = ApiClientConfig::default();
        let client = ApiClient::new(config.clone());
        assert_eq!(client.config.base_url, "http://localhost:8000");
    }

    #[test]
    fn test_service_update_serialization() {
        let update = ServiceUpdate {
            service_name: "p2p".to_string(),
            event_type: "started".to_string(),
            data: json!({"status": "running"}),
            timestamp: Utc::now().to_rfc3339(),
        };

        let json = serde_json::to_string(&update).unwrap();
        let decoded: ServiceUpdate = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.service_name, "p2p");
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = ApiClientConfig {
            base_url: "http://localhost:9999".to_string(),
            ws_url: "ws://localhost:9999/ws".to_string(),
            timeout_seconds: 1,
            max_retries: 1,
        };
        let client = ApiClient::new(config);

        // This will fail (no server), but tests the method exists
        let result = client.health_check().await;
        assert!(result.is_err());
    }
}
