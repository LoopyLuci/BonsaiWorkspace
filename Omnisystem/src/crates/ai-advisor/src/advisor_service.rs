/// Async advisor service orchestration and multi-advisor routing
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisorRequest {
    pub request_id: String,
    pub query: String,
    pub context: HashMap<String, String>,
    pub priority: u8,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisorResponse {
    pub response_id: String,
    pub request_id: String,
    pub advisor_id: String,
    pub recommendation: String,
    pub confidence: f32,
    pub reasoning: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedResponse {
    pub request_id: String,
    pub responses: Vec<AdvisorResponse>,
    pub consensus: String,
    pub average_confidence: f32,
    pub timestamp: i64,
}

pub struct AdvisorPool {
    advisors: Arc<RwLock<HashMap<String, String>>>,
    responses_cache: Arc<RwLock<HashMap<String, Vec<AdvisorResponse>>>>,
    active_advisors: Arc<RwLock<usize>>,
}

impl AdvisorPool {
    pub fn new() -> Self {
        Self {
            advisors: Arc::new(RwLock::new(HashMap::new())),
            responses_cache: Arc::new(RwLock::new(HashMap::new())),
            active_advisors: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn register_advisor(&self, advisor_id: String, advisor_type: String) -> Result<()> {
        let mut advisors = self.advisors.write().await;
        advisors.insert(advisor_id, advisor_type);

        let mut active = self.active_advisors.write().await;
        *active += 1;

        tracing::info!("Registered advisor, active count: {}", *active);
        Ok(())
    }

    pub async fn unregister_advisor(&self, advisor_id: &str) -> Result<()> {
        let mut advisors = self.advisors.write().await;
        advisors.remove(advisor_id);

        let mut active = self.active_advisors.write().await;
        if *active > 0 {
            *active -= 1;
        }

        tracing::info!("Unregistered advisor, active count: {}", *active);
        Ok(())
    }

    pub async fn get_active_advisors(&self) -> Result<Vec<String>> {
        let advisors = self.advisors.read().await;
        Ok(advisors.keys().cloned().collect())
    }

    pub async fn route_request(&self, request: &AdvisorRequest) -> Result<Vec<AdvisorResponse>> {
        let advisors = self.advisors.read().await;
        let mut responses = Vec::new();

        for (advisor_id, _advisor_type) in advisors.iter() {
            let response = AdvisorResponse {
                response_id: uuid::Uuid::new_v4().to_string(),
                request_id: request.request_id.clone(),
                advisor_id: advisor_id.clone(),
                recommendation: format!("Recommendation from {}", advisor_id),
                confidence: 0.85,
                reasoning: "Based on analysis of provided context".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
            };
            responses.push(response);
        }

        let mut cache = self.responses_cache.write().await;
        cache.insert(request.request_id.clone(), responses.clone());

        tracing::info!("Routed request to {} advisors", responses.len());
        Ok(responses)
    }

    pub async fn aggregate_responses(&self, request_id: &str) -> Result<AggregatedResponse> {
        let cache = self.responses_cache.read().await;
        let responses = cache.get(request_id)
            .cloned()
            .unwrap_or_default();

        let average_confidence = if responses.is_empty() {
            0.0
        } else {
            responses.iter().map(|r| r.confidence).sum::<f32>() / responses.len() as f32
        };

        let consensus = if responses.is_empty() {
            "No advisors available".to_string()
        } else {
            "Consensus recommendation from advisors".to_string()
        };

        let aggregated = AggregatedResponse {
            request_id: request_id.to_string(),
            responses,
            consensus,
            average_confidence,
            timestamp: chrono::Utc::now().timestamp(),
        };

        Ok(aggregated)
    }

    pub async fn cache_response(&self, response: AdvisorResponse) -> Result<()> {
        let mut cache = self.responses_cache.write().await;
        cache.entry(response.request_id.clone())
            .or_insert_with(Vec::new)
            .push(response);

        Ok(())
    }

    pub async fn get_cached_responses(&self, request_id: &str) -> Result<Vec<AdvisorResponse>> {
        let cache = self.responses_cache.read().await;
        Ok(cache.get(request_id).cloned().unwrap_or_default())
    }

    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.responses_cache.write().await;
        let count = cache.len();
        cache.clear();
        tracing::info!("Cleared response cache: {} entries removed", count);
        Ok(())
    }

    pub async fn advisor_count(&self) -> Result<usize> {
        let advisors = self.advisors.read().await;
        Ok(advisors.len())
    }

    pub async fn get_advisor_info(&self, advisor_id: &str) -> Result<Option<String>> {
        let advisors = self.advisors.read().await;
        Ok(advisors.get(advisor_id).cloned())
    }
}

impl Default for AdvisorPool {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AdvisorService {
    pool: Arc<AdvisorPool>,
    request_history: Arc<RwLock<Vec<AdvisorRequest>>>,
}

impl AdvisorService {
    pub fn new(pool: Arc<AdvisorPool>) -> Self {
        Self {
            pool,
            request_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn process_request(&self, request: AdvisorRequest) -> Result<AggregatedResponse> {
        let mut history = self.request_history.write().await;
        history.push(request.clone());

        let responses = self.pool.route_request(&request).await?;

        for response in responses {
            self.pool.cache_response(response).await?;
        }

        let aggregated = self.pool.aggregate_responses(&request.request_id).await?;
        tracing::info!("Processed request: {} with {} advisors", request.request_id, aggregated.responses.len());

        Ok(aggregated)
    }

    pub async fn get_request_history(&self) -> Result<Vec<AdvisorRequest>> {
        let history = self.request_history.read().await;
        Ok(history.clone())
    }

    pub async fn clear_history(&self) -> Result<()> {
        let mut history = self.request_history.write().await;
        let count = history.len();
        history.clear();
        tracing::info!("Cleared request history: {} entries removed", count);
        Ok(())
    }

    pub async fn get_advisor_pool(&self) -> Arc<AdvisorPool> {
        self.pool.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_advisor_pool_creation() {
        let pool = AdvisorPool::new();
        let count = pool.advisor_count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_register_advisor() {
        let pool = AdvisorPool::new();
        let result = pool.register_advisor("advisor-1".to_string(), "type-1".to_string()).await;
        assert!(result.is_ok());

        let count = pool.advisor_count().await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_route_request() {
        let pool = Arc::new(AdvisorPool::new());
        pool.register_advisor("advisor-1".to_string(), "type-1".to_string()).await.unwrap();

        let request = AdvisorRequest {
            request_id: "req-1".to_string(),
            query: "test query".to_string(),
            context: HashMap::new(),
            priority: 5,
            timestamp: chrono::Utc::now().timestamp(),
        };

        let responses = pool.route_request(&request).await.unwrap();
        assert_eq!(responses.len(), 1);
    }

    #[tokio::test]
    async fn test_aggregate_responses() {
        let pool = Arc::new(AdvisorPool::new());
        pool.register_advisor("advisor-1".to_string(), "type-1".to_string()).await.unwrap();

        let request = AdvisorRequest {
            request_id: "req-1".to_string(),
            query: "test".to_string(),
            context: HashMap::new(),
            priority: 5,
            timestamp: chrono::Utc::now().timestamp(),
        };

        let _responses = pool.route_request(&request).await.unwrap();
        let aggregated = pool.aggregate_responses("req-1").await.unwrap();

        assert_eq!(aggregated.request_id, "req-1");
        assert!(aggregated.average_confidence > 0.0);
    }

    #[tokio::test]
    async fn test_advisor_service() {
        let pool = Arc::new(AdvisorPool::new());
        pool.register_advisor("advisor-1".to_string(), "type-1".to_string()).await.unwrap();

        let service = AdvisorService::new(pool);
        let request = AdvisorRequest {
            request_id: "req-1".to_string(),
            query: "test".to_string(),
            context: HashMap::new(),
            priority: 5,
            timestamp: chrono::Utc::now().timestamp(),
        };

        let result = service.process_request(request).await;
        assert!(result.is_ok());
    }
}
