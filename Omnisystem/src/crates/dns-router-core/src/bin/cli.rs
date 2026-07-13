//! Minimal demo CLI: creates a zone, adds an A record, then resolves it
//! through the real DefaultQueryResolver pipeline.

use std::net::Ipv4Addr;
use std::sync::Arc;

use dns_router_core::{
    DefaultHealthChecker, DefaultMetricsCollector, DefaultQueryResolver, DnsQuery, DnsRecord,
    DnsZone, DomainName, GeoRouter, QueryResolver, RecordData, RecordManager, RecordType,
    Zone, ZoneManager,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zone_store = Arc::new(DnsZone::new());
    let geo_router = Arc::new(GeoRouter::new());
    let metrics = Arc::new(DefaultMetricsCollector::new());
    let health_checker = Arc::new(DefaultHealthChecker::new());

    let resolver = DefaultQueryResolver::new(
        zone_store.clone(),
        zone_store.clone(),
        geo_router,
        metrics,
        health_checker,
    );

    let domain = DomainName("example.test".to_string());
    let zone_id = zone_store.create_zone(Zone::new(domain.clone())).await?;
    println!("Created zone {} for {}", zone_id.0, domain.0);

    let record = DnsRecord::new(
        domain.0.clone(),
        RecordType::A,
        RecordData::A(Ipv4Addr::new(93, 184, 216, 34)),
        3600,
    );
    zone_store.create_record(&zone_id, record).await?;
    println!("Added A record for {}", domain.0);

    let query = DnsQuery {
        domain: domain.clone(),
        record_type: RecordType::A,
        client_ip: None,
    };
    let response = resolver.resolve(&query).await?;
    println!(
        "Resolved {} -> {} record(s), authoritative={}",
        domain.0,
        response.records.len(),
        response.authoritative
    );
    for rec in &response.records {
        println!("  {:?}", rec.data);
    }

    Ok(())
}
