/// High-level advisory engine orchestrating advisors, arbitration, and metrics
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdviceRequest {
    pub request_id: String,
    pub domain: String,
    pub query: String,
    pub context: HashMap<String, String>,
    pub priority: u8,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdviceResponse {
    pub response_id: String,
    pub request_id: String,
    pub advice: String,
    pub confidence: f32,
    pub sources: Vec<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdvisorHealth {
    Healthy,
    Degraded,
    Quarantined,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisorStatus {
    pub advisor_id: String,
    pub health: AdvisorHealth,
    pub last_check: i64,
    pub response_count: usize,
    pub error_count: usize,
}

pub struct AdvisoryEngine {
    advisors: Arc<RwLock<HashMap<String, AdvisorStatus>>>,
    responses: Arc<RwLock<Vec<AdviceResponse>>>,
    request_queue: Arc<RwLock<Vec<AdviceRequest>>>,
    domain_mapping: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl AdvisoryEngine {
    pub fn new() -> Self {
        Self {
            advisors: Arc::new(RwLock::new(HashMap::new())),
            responses: Arc::new(RwLock::new(Vec::new())),
            request_queue: Arc::new(RwLock::new(Vec::new())),
            domain_mapping: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_advisor(&self, advisor_id: String, domain: String) -> Result<()> {
        let mut advisors = self.advisors.write().await;
        advisors.insert(
            advisor_id.clone(),
            AdvisorStatus {
                advisor_id: advisor_id.clone(),
                health: AdvisorHealth::Healthy,
                last_check: chrono::Utc::now().timestamp(),
                response_count: 0,
                error_count: 0,
            },
        );

        let mut mapping = self.domain_mapping.write().await;
        mapping.entry(domain).or_insert_with(Vec::new).push(advisor_id);

        tracing::info!("Registered advisor");
        Ok(())
    }

    pub async fn unregister_advisor(&self, advisor_id: &str) -> Result<()> {
        let mut advisors = self.advisors.write().await;
        advisors.remove(advisor_id);

        let mut mapping = self.domain_mapping.write().await;
        for advisors_list in mapping.values_mut() {
            advisors_list.retain(|a| a != advisor_id);
        }

        tracing::info!("Unregistered advisor: {}", advisor_id);
        Ok(())
    }

    pub async fn check_advisor_health(&self, advisor_id: &str) -> Result<Option<AdvisorHealth>> {
        let advisors = self.advisors.read().await;
        Ok(advisors.get(advisor_id).map(|s| s.health))
    }

    pub async fn update_advisor_health(&self, advisor_id: &str, health: AdvisorHealth) -> Result<()> {
        let mut advisors = self.advisors.write().await;
        if let Some(status) = advisors.get_mut(advisor_id) {
            status.health = health;
            status.last_check = chrono::Utc::now().timestamp();
        }

        tracing::info!("Updated advisor health: {:?}", health);
        Ok(())
    }

    pub async fn submit_request(&self, request: AdviceRequest) -> Result<String> {
        let request_id = request.request_id.clone();

        let mut queue = self.request_queue.write().await;
        queue.push(request);

        tracing::info!("Submitted advice request: {}", request_id);
        Ok(request_id)
    }

    pub async fn process_request(&self, request_id: &str) -> Result<AdviceResponse> {
        let queue = self.request_queue.read().await;
        let request = queue
            .iter()
            .find(|r| r.request_id == request_id)
            .cloned();
        drop(queue);

        if let Some(request) = request {
            let mapping = self.domain_mapping.read().await;
            let advisors = mapping
                .get(&request.domain)
                .cloned()
                .unwrap_or_default();
            drop(mapping);

            let response = AdviceResponse {
                response_id: uuid::Uuid::new_v4().to_string(),
                request_id: request.request_id.clone(),
                advice: format!("Advice from {} advisors", advisors.len()),
                confidence: 0.85,
                sources: advisors,
                timestamp: chrono::Utc::now().timestamp(),
            };

            let mut responses = self.responses.write().await;
            responses.push(response.clone());

            tracing::info!("Processed request: {}", request_id);
            Ok(response)
        } else {
            Err(anyhow::anyhow!("Request not found: {}", request_id))
        }
    }

    pub async fn get_advisors_for_domain(&self, domain: &str) -> Result<Vec<String>> {
        let mapping = self.domain_mapping.read().await;
        Ok(mapping.get(domain).cloned().unwrap_or_default())
    }

    pub async fn get_advisor_status(&self, advisor_id: &str) -> Result<Option<AdvisorStatus>> {
        let advisors = self.advisors.read().await;
        Ok(advisors.get(advisor_id).cloned())
    }

    pub async fn get_all_advisors(&self) -> Result<Vec<AdvisorStatus>> {
        let advisors = self.advisors.read().await;
        Ok(advisors.values().cloned().collect())
    }

    pub async fn get_response(&self, response_id: &str) -> Result<Option<AdviceResponse>> {
        let responses = self.responses.read().await;
        Ok(responses.iter().find(|r| r.response_id == response_id).cloned())
    }

    pub async fn record_success(&self, advisor_id: &str) -> Result<()> {
        let mut advisors = self.advisors.write().await;
        if let Some(status) = advisors.get_mut(advisor_id) {
            status.response_count += 1;
        }
        Ok(())
    }

    pub async fn record_error(&self, advisor_id: &str) -> Result<()> {
        let mut advisors = self.advisors.write().await;
        if let Some(status) = advisors.get_mut(advisor_id) {
            status.error_count += 1;

            let error_rate = if status.response_count + status.error_count > 0 {
                status.error_count as f32 / (status.response_count + status.error_count) as f32
            } else {
                0.0
            };

            if error_rate > 0.5 {
                status.health = AdvisorHealth::Quarantined;
            } else if error_rate > 0.2 {
                status.health = AdvisorHealth::Degraded;
            }
        }
        Ok(())
    }

    pub async fn get_healthy_advisors(&self) -> Result<Vec<String>> {
        let advisors = self.advisors.read().await;
        let healthy: Vec<String> = advisors
            .iter()
            .filter(|(_, status)| status.health == AdvisorHealth::Healthy)
            .map(|(id, _)| id.clone())
            .collect();
        Ok(healthy)
    }

    pub async fn get_request_history(&self, limit: usize) -> Result<Vec<AdviceRequest>> {
        let queue = self.request_queue.read().await;
        let history: Vec<AdviceRequest> = queue
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect();
        Ok(history)
    }

    pub async fn get_response_history(&self, limit: usize) -> Result<Vec<AdviceResponse>> {
        let responses = self.responses.read().await;
        let history: Vec<AdviceResponse> = responses
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect();
        Ok(history)
    }

    pub async fn get_engine_stats(&self) -> Result<HashMap<String, String>> {
        let advisors = self.advisors.read().await;
        let responses = self.responses.read().await;
        let queue = self.request_queue.read().await;

        let mut stats = HashMap::new();
        stats.insert("total_advisors".to_string(), advisors.len().to_string());
        stats.insert("total_responses".to_string(), responses.len().to_string());
        stats.insert("pending_requests".to_string(), queue.len().to_string());

        let healthy_count = advisors.values().filter(|s| s.health == AdvisorHealth::Healthy).count();
        stats.insert("healthy_advisors".to_string(), healthy_count.to_string());

        let total_response_count: usize = advisors.values().map(|s| s.response_count).sum();
        stats.insert("total_advisor_responses".to_string(), total_response_count.to_string());

        Ok(stats)
    }

    pub async fn clear_request_queue(&self) -> Result<()> {
        let mut queue = self.request_queue.write().await;
        let count = queue.len();
        queue.clear();
        tracing::info!("Cleared request queue: {} entries", count);
        Ok(())
    }
}

impl Default for AdvisoryEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = AdvisoryEngine::new();
        let stats = engine.get_engine_stats().await.unwrap();
        assert_eq!(stats.get("total_advisors").map(|s| s.as_str()), Some("0"));
    }

    #[tokio::test]
    async fn test_register_advisor() {
        let engine = AdvisoryEngine::new();
        let result = engine.register_advisor("advisor-1".to_string(), "domain-1".to_string()).await;
        assert!(result.is_ok());

        let advisors = engine.get_advisors_for_domain("domain-1").await.unwrap();
        assert!(advisors.contains(&"advisor-1".to_string()));
    }

    #[tokio::test]
    async fn test_advisor_health() {
        let engine = AdvisoryEngine::new();
        engine.register_advisor("advisor-1".to_string(), "domain-1".to_string()).await.unwrap();

        let health = engine.check_advisor_health("advisor-1").await.unwrap();
        assert_eq!(health, Some(AdvisorHealth::Healthy));

        engine.update_advisor_health("advisor-1", AdvisorHealth::Degraded).await.unwrap();
        let health = engine.check_advisor_health("advisor-1").await.unwrap();
        assert_eq!(health, Some(AdvisorHealth::Degraded));
    }

    #[tokio::test]
    async fn test_submit_and_process_request() {
        let engine = AdvisoryEngine::new();
        engine.register_advisor("advisor-1".to_string(), "domain-1".to_string()).await.unwrap();

        let request = AdviceRequest {
            request_id: "req-1".to_string(),
            domain: "domain-1".to_string(),
            query: "test query".to_string(),
            context: HashMap::new(),
            priority: 5,
            timestamp: chrono::Utc::now().timestamp(),
        };

        engine.submit_request(request).await.unwrap();
        let response = engine.process_request("req-1").await.unwrap();

        assert_eq!(response.request_id, "req-1");
        assert!(!response.advice.is_empty());
    }

    #[tokio::test]
    async fn test_error_tracking() {
        let engine = AdvisoryEngine::new();
        engine.register_advisor("advisor-1".to_string(), "domain-1".to_string()).await.unwrap();

        for _ in 0..3 {
            engine.record_error("advisor-1").await.unwrap();
        }

        let status = engine.get_advisor_status("advisor-1").await.unwrap().unwrap();
        assert_eq!(status.error_count, 3);
    }
}
