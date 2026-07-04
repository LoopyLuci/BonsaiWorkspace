use crate::{
    DnsError, DnsQuery, DnsRecord, DnsResponse, DnsResult, DomainName, GeoLocation, GeoRouter,
    QueryResolver, RecordManager, ZoneId, ZoneManager, MetricsCollector, HealthChecker,
};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use chrono::{Utc, Duration};

pub struct QueryCache {
    cache: Arc<DashMap<String, (DnsResponse, chrono::DateTime<chrono::Utc>)>>,
}

impl QueryCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }

    pub fn insert(&self, key: String, response: DnsResponse, ttl_secs: u64) {
        self.cache
            .insert(key, (response, Utc::now() + Duration::seconds(ttl_secs as i64)));
    }

    pub fn get(&self, key: &str) -> Option<DnsResponse> {
        if let Some(entry) = self.cache.get(key) {
            let (response, expiry) = entry.value();
            if Utc::now() < *expiry {
                return Some(response.clone());
            } else {
                drop(entry);
                self.cache.remove(key);
            }
        }
        None
    }

    pub fn size(&self) -> usize {
        self.cache.len()
    }

    pub fn clear(&self) {
        self.cache.clear();
    }
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DefaultQueryResolver {
    zone_manager: Arc<dyn ZoneManager>,
    record_manager: Arc<dyn RecordManager>,
    geo_router: Arc<GeoRouter>,
    cache: Arc<QueryCache>,
    metrics: Arc<dyn MetricsCollector>,
    #[allow(dead_code)]
    health_checker: Arc<dyn HealthChecker>,
    query_count: Arc<AtomicU64>,
}

impl DefaultQueryResolver {
    pub fn new(
        zone_manager: Arc<dyn ZoneManager>,
        record_manager: Arc<dyn RecordManager>,
        geo_router: Arc<GeoRouter>,
        metrics: Arc<dyn MetricsCollector>,
        health_checker: Arc<dyn HealthChecker>,
    ) -> Self {
        Self {
            zone_manager,
            record_manager,
            geo_router,
            cache: Arc::new(QueryCache::new()),
            metrics,
            health_checker,
            query_count: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn query_count(&self) -> u64 {
        self.query_count.load(Ordering::Relaxed)
    }

    pub fn cache_size(&self) -> usize {
        self.cache.size()
    }

    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

#[async_trait]
impl QueryResolver for DefaultQueryResolver {
    async fn resolve(&self, query: &DnsQuery) -> DnsResult<DnsResponse> {
        self.query_count.fetch_add(1, Ordering::Relaxed);

        let cache_key = format!("{}-{}", query.domain.0, query.record_type.to_string());

        if let Some(cached) = self.cache.get(&cache_key) {
            let _ = self.metrics.record_query(true, 0.1).await;
            return Ok(cached);
        }

        let start = std::time::Instant::now();

        let zone = self
            .zone_manager
            .get_zone_by_name(&query.domain)
            .await?;

        let records = self
            .record_manager
            .get_records_by_type(&zone.id, query.record_type)
            .await?;

        if records.is_empty() {
            let _ = self.metrics.record_query(false, start.elapsed().as_millis() as f64).await;
            return Err(DnsError::RecordNotFound(query.domain.0.clone()));
        }

        let response = DnsResponse {
            query: query.clone(),
            records,
            authoritative: true,
            recursion_available: false,
        };

        let response_time = start.elapsed().as_millis() as f64;
        let _ = self.metrics.record_query(true, response_time).await;

        let min_ttl = response.records.iter().map(|r| r.ttl).min().unwrap_or(3600);
        self.cache.insert(cache_key, response.clone(), min_ttl as u64);

        Ok(response)
    }

    async fn resolve_recursive(&self, query: &DnsQuery) -> DnsResult<DnsResponse> {
        self.resolve(query).await
    }

    async fn resolve_with_geo(
        &self,
        query: &DnsQuery,
        location: &GeoLocation,
    ) -> DnsResult<DnsResponse> {
        let mut response = self.resolve(query).await?;

        if let Some(target) = self
            .geo_router
            .select_route(&query.domain.0, location)
            .await
            .ok()
        {
            response.records.sort_by(|a, b| {
                let a_priority = if a.name.contains(&target) { 1 } else { 0 };
                let b_priority = if b.name.contains(&target) { 1 } else { 0 };
                b_priority.cmp(&a_priority)
            });
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DnsMetrics, HealthStatus, RecordData, RecordType};
    use std::net::Ipv4Addr;

    struct MockZoneManager;

    #[async_trait]
    impl ZoneManager for MockZoneManager {
        async fn create_zone(&self, _zone: crate::Zone) -> DnsResult<ZoneId> {
            unimplemented!()
        }
        async fn get_zone(&self, _id: &ZoneId) -> DnsResult<crate::Zone> {
            unimplemented!()
        }
        async fn get_zone_by_name(&self, name: &DomainName) -> DnsResult<crate::Zone> {
            Ok(crate::Zone::new(name.clone()))
        }
        async fn update_zone(&self, _id: &ZoneId, _zone: crate::Zone) -> DnsResult<()> {
            unimplemented!()
        }
        async fn delete_zone(&self, _id: &ZoneId) -> DnsResult<()> {
            unimplemented!()
        }
        async fn list_zones(&self) -> DnsResult<Vec<crate::Zone>> {
            unimplemented!()
        }
    }

    struct MockRecordManager;

    #[async_trait]
    impl RecordManager for MockRecordManager {
        async fn create_record(
            &self,
            _zone_id: &ZoneId,
            _record: DnsRecord,
        ) -> DnsResult<crate::RecordId> {
            unimplemented!()
        }
        async fn get_record(
            &self,
            _zone_id: &ZoneId,
            _record_id: &crate::RecordId,
        ) -> DnsResult<DnsRecord> {
            unimplemented!()
        }
        async fn get_records_by_name(
            &self,
            _zone_id: &ZoneId,
            _name: &str,
        ) -> DnsResult<Vec<DnsRecord>> {
            unimplemented!()
        }
        async fn get_records_by_type(
            &self,
            _zone_id: &ZoneId,
            _record_type: RecordType,
        ) -> DnsResult<Vec<DnsRecord>> {
            Ok(vec![DnsRecord::new(
                "www".to_string(),
                RecordType::A,
                RecordData::A(Ipv4Addr::new(192, 0, 2, 1)),
                3600,
            )])
        }
        async fn update_record(
            &self,
            _zone_id: &ZoneId,
            _record_id: &crate::RecordId,
            _record: DnsRecord,
        ) -> DnsResult<()> {
            unimplemented!()
        }
        async fn delete_record(
            &self,
            _zone_id: &ZoneId,
            _record_id: &crate::RecordId,
        ) -> DnsResult<()> {
            unimplemented!()
        }
        async fn list_records(&self, _zone_id: &ZoneId) -> DnsResult<Vec<DnsRecord>> {
            unimplemented!()
        }
    }

    struct MockMetricsCollector;

    #[async_trait]
    impl MetricsCollector for MockMetricsCollector {
        async fn record_query(&self, _success: bool, _response_time_ms: f64) -> DnsResult<()> {
            Ok(())
        }
        async fn get_metrics(&self) -> DnsResult<DnsMetrics> {
            Ok(DnsMetrics::default())
        }
        async fn reset_metrics(&self) -> DnsResult<()> {
            Ok(())
        }
    }

    struct MockHealthChecker;

    #[async_trait]
    impl crate::HealthChecker for MockHealthChecker {
        async fn check_health(&self, _server: &str) -> DnsResult<HealthStatus> {
            Ok(HealthStatus::Healthy)
        }
        async fn get_server_health(&self, _server: &str) -> DnsResult<HealthStatus> {
            Ok(HealthStatus::Healthy)
        }
        async fn mark_healthy(&self, _server: &str) -> DnsResult<()> {
            Ok(())
        }
        async fn mark_unhealthy(&self, _server: &str) -> DnsResult<()> {
            Ok(())
        }
        async fn get_healthy_servers(&self, _zone_id: &ZoneId) -> DnsResult<Vec<String>> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_resolve_query() {
        let resolver = DefaultQueryResolver::new(
            Arc::new(MockZoneManager),
            Arc::new(MockRecordManager),
            Arc::new(GeoRouter::new()),
            Arc::new(MockMetricsCollector),
            Arc::new(MockHealthChecker),
        );

        let query = DnsQuery {
            domain: DomainName("example.com".to_string()),
            record_type: RecordType::A,
            client_ip: None,
        };

        let response = resolver.resolve(&query).await.unwrap();
        assert_eq!(response.records.len(), 1);
    }

    #[tokio::test]
    async fn test_query_caching() {
        let resolver = DefaultQueryResolver::new(
            Arc::new(MockZoneManager),
            Arc::new(MockRecordManager),
            Arc::new(GeoRouter::new()),
            Arc::new(MockMetricsCollector),
            Arc::new(MockHealthChecker),
        );

        let query = DnsQuery {
            domain: DomainName("example.com".to_string()),
            record_type: RecordType::A,
            client_ip: None,
        };

        let _response1 = resolver.resolve(&query).await.unwrap();
        assert_eq!(resolver.cache_size(), 1);

        let _response2 = resolver.resolve(&query).await.unwrap();
        assert_eq!(resolver.query_count(), 2);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let resolver = DefaultQueryResolver::new(
            Arc::new(MockZoneManager),
            Arc::new(MockRecordManager),
            Arc::new(GeoRouter::new()),
            Arc::new(MockMetricsCollector),
            Arc::new(MockHealthChecker),
        );

        let query = DnsQuery {
            domain: DomainName("example.com".to_string()),
            record_type: RecordType::A,
            client_ip: None,
        };

        let _response = resolver.resolve(&query).await.unwrap();
        assert_eq!(resolver.cache_size(), 1);

        resolver.clear_cache();
        assert_eq!(resolver.cache_size(), 0);
    }
}
