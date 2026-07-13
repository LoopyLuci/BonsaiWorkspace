use async_trait::async_trait;
use crate::{
    DnsMetrics, DnsQuery, DnsRecord, DnsResponse, DnsResult, DomainName,
    GeoLocation, HealthStatus, RecordId, RecordType, Zone, ZoneId,
};

#[async_trait]
pub trait ZoneManager: Send + Sync {
    async fn create_zone(&self, zone: Zone) -> DnsResult<ZoneId>;
    async fn get_zone(&self, id: &ZoneId) -> DnsResult<Zone>;
    async fn get_zone_by_name(&self, name: &DomainName) -> DnsResult<Zone>;
    async fn update_zone(&self, id: &ZoneId, zone: Zone) -> DnsResult<()>;
    async fn delete_zone(&self, id: &ZoneId) -> DnsResult<()>;
    async fn list_zones(&self) -> DnsResult<Vec<Zone>>;
}

#[async_trait]
pub trait RecordManager: Send + Sync {
    async fn create_record(&self, zone_id: &ZoneId, record: DnsRecord) -> DnsResult<RecordId>;
    async fn get_record(&self, zone_id: &ZoneId, record_id: &RecordId) -> DnsResult<DnsRecord>;
    async fn get_records_by_name(
        &self,
        zone_id: &ZoneId,
        name: &str,
    ) -> DnsResult<Vec<DnsRecord>>;
    async fn get_records_by_type(
        &self,
        zone_id: &ZoneId,
        record_type: RecordType,
    ) -> DnsResult<Vec<DnsRecord>>;
    async fn update_record(
        &self,
        zone_id: &ZoneId,
        record_id: &RecordId,
        record: DnsRecord,
    ) -> DnsResult<()>;
    async fn delete_record(&self, zone_id: &ZoneId, record_id: &RecordId) -> DnsResult<()>;
    async fn list_records(&self, zone_id: &ZoneId) -> DnsResult<Vec<DnsRecord>>;
}

#[async_trait]
pub trait QueryResolver: Send + Sync {
    async fn resolve(&self, query: &DnsQuery) -> DnsResult<DnsResponse>;
    async fn resolve_recursive(&self, query: &DnsQuery) -> DnsResult<DnsResponse>;
    async fn resolve_with_geo(&self, query: &DnsQuery, location: &GeoLocation) -> DnsResult<DnsResponse>;
}

#[async_trait]
pub trait HealthChecker: Send + Sync {
    async fn check_health(&self, server: &str) -> DnsResult<HealthStatus>;
    async fn get_server_health(&self, server: &str) -> DnsResult<HealthStatus>;
    async fn mark_healthy(&self, server: &str) -> DnsResult<()>;
    async fn mark_unhealthy(&self, server: &str) -> DnsResult<()>;
    async fn get_healthy_servers(&self, zone_id: &ZoneId) -> DnsResult<Vec<String>>;
}

#[async_trait]
pub trait DynamicDnsUpdater: Send + Sync {
    async fn validate_update(
        &self,
        zone_id: &ZoneId,
        client_id: &str,
    ) -> DnsResult<bool>;
    async fn apply_update(&self, zone_id: &ZoneId, records: Vec<DnsRecord>) -> DnsResult<()>;
    async fn get_update_status(&self, zone_id: &ZoneId) -> DnsResult<chrono::DateTime<chrono::Utc>>;
}

#[async_trait]
pub trait MetricsCollector: Send + Sync {
    async fn record_query(&self, success: bool, response_time_ms: f64) -> DnsResult<()>;
    async fn get_metrics(&self) -> DnsResult<DnsMetrics>;
    async fn reset_metrics(&self) -> DnsResult<()>;
}

#[async_trait]
pub trait RateLimiter: Send + Sync {
    async fn check_rate_limit(&self, client_ip: &str) -> DnsResult<bool>;
    async fn allow_query(&self, client_ip: &str) -> DnsResult<()>;
    async fn reset_limit(&self, client_ip: &str) -> DnsResult<()>;
}
