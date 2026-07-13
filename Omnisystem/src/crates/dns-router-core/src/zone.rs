use crate::{
    DnsError, DnsResult, DomainName, RecordManager, RecordType, Zone, ZoneId, ZoneManager,
    DnsRecord, RecordId,
};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use std::collections::HashMap;

pub struct DnsZone {
    zones: Arc<DashMap<String, Zone>>,
    zone_by_name: Arc<DashMap<String, ZoneId>>,
    records: Arc<DashMap<String, HashMap<String, Vec<DnsRecord>>>>,
}

impl DnsZone {
    pub fn new() -> Self {
        Self {
            zones: Arc::new(DashMap::new()),
            zone_by_name: Arc::new(DashMap::new()),
            records: Arc::new(DashMap::new()),
        }
    }

    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    pub fn record_count(&self, zone_id: &ZoneId) -> usize {
        self.records
            .get(&zone_id.0.to_string())
            .map(|entry| {
                entry
                    .iter()
                    .map(|(_, records)| records.len())
                    .sum()
            })
            .unwrap_or(0)
    }
}

impl Default for DnsZone {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ZoneManager for DnsZone {
    async fn create_zone(&self, zone: Zone) -> DnsResult<ZoneId> {
        let zone_id = zone.id.clone();
        let zone_name = zone.name.clone();

        if self.zone_by_name.contains_key(&zone_name.0) {
            return Err(DnsError::ZoneAlreadyExists(zone_name.0));
        }

        self.zones.insert(zone_id.0.to_string(), zone);
        self.zone_by_name.insert(zone_name.0, zone_id.clone());
        self.records
            .insert(zone_id.0.to_string(), HashMap::new());

        Ok(zone_id)
    }

    async fn get_zone(&self, id: &ZoneId) -> DnsResult<Zone> {
        self.zones
            .get(&id.0.to_string())
            .map(|entry| entry.clone())
            .ok_or_else(|| DnsError::ZoneNotFound(id.0.to_string()))
    }

    async fn get_zone_by_name(&self, name: &DomainName) -> DnsResult<Zone> {
        let zone_id = self
            .zone_by_name
            .get(&name.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| DnsError::ZoneNotFound(name.0.clone()))?;

        self.get_zone(&zone_id).await
    }

    async fn update_zone(&self, id: &ZoneId, mut zone: Zone) -> DnsResult<()> {
        if !self.zones.contains_key(&id.0.to_string()) {
            return Err(DnsError::ZoneNotFound(id.0.to_string()));
        }

        zone.increment_serial();
        self.zones.insert(id.0.to_string(), zone);
        Ok(())
    }

    async fn delete_zone(&self, id: &ZoneId) -> DnsResult<()> {
        self.zones.remove(&id.0.to_string());
        self.records.remove(&id.0.to_string());
        Ok(())
    }

    async fn list_zones(&self) -> DnsResult<Vec<Zone>> {
        Ok(self
            .zones
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }
}

#[async_trait]
impl RecordManager for DnsZone {
    async fn create_record(&self, zone_id: &ZoneId, record: DnsRecord) -> DnsResult<RecordId> {
        if !self.zones.contains_key(&zone_id.0.to_string()) {
            return Err(DnsError::ZoneNotFound(zone_id.0.to_string()));
        }

        let record_id = record.id.clone();
        let record_key = format!("{}-{}", record.name, record.record_type.to_string());

        let mut records_map = self
            .records
            .entry(zone_id.0.to_string())
            .or_insert_with(HashMap::new);

        records_map
            .entry(record_key)
            .or_insert_with(Vec::new)
            .push(record);

        Ok(record_id)
    }

    async fn get_record(&self, zone_id: &ZoneId, record_id: &RecordId) -> DnsResult<DnsRecord> {
        let records_map = self
            .records
            .get(&zone_id.0.to_string())
            .ok_or_else(|| DnsError::ZoneNotFound(zone_id.0.to_string()))?;

        for (_, records) in records_map.iter() {
            for record in records {
                if record.id == *record_id {
                    return Ok(record.clone());
                }
            }
        }

        Err(DnsError::RecordNotFound(record_id.0.to_string()))
    }

    async fn get_records_by_name(
        &self,
        zone_id: &ZoneId,
        name: &str,
    ) -> DnsResult<Vec<DnsRecord>> {
        let records_map = self
            .records
            .get(&zone_id.0.to_string())
            .ok_or_else(|| DnsError::ZoneNotFound(zone_id.0.to_string()))?;

        let mut results = Vec::new();
        for (key, records) in records_map.iter() {
            if key.starts_with(name) {
                results.extend(records.clone());
            }
        }

        Ok(results)
    }

    async fn get_records_by_type(
        &self,
        zone_id: &ZoneId,
        record_type: RecordType,
    ) -> DnsResult<Vec<DnsRecord>> {
        let records_map = self
            .records
            .get(&zone_id.0.to_string())
            .ok_or_else(|| DnsError::ZoneNotFound(zone_id.0.to_string()))?;

        let type_str = record_type.to_string();
        let mut results = Vec::new();

        for (key, records) in records_map.iter() {
            if key.ends_with(type_str) {
                results.extend(records.clone());
            }
        }

        Ok(results)
    }

    async fn update_record(
        &self,
        zone_id: &ZoneId,
        record_id: &RecordId,
        new_record: DnsRecord,
    ) -> DnsResult<()> {
        if !self.zones.contains_key(&zone_id.0.to_string()) {
            return Err(DnsError::ZoneNotFound(zone_id.0.to_string()));
        }

        let mut records_map = self
            .records
            .get_mut(&zone_id.0.to_string())
            .ok_or_else(|| DnsError::ZoneNotFound(zone_id.0.to_string()))?;

        for (_, records) in records_map.iter_mut() {
            if let Some(idx) = records.iter().position(|r| r.id == *record_id) {
                records[idx] = new_record;
                return Ok(());
            }
        }

        Err(DnsError::RecordNotFound(record_id.0.to_string()))
    }

    async fn delete_record(&self, zone_id: &ZoneId, record_id: &RecordId) -> DnsResult<()> {
        let mut records_map = self
            .records
            .get_mut(&zone_id.0.to_string())
            .ok_or_else(|| DnsError::ZoneNotFound(zone_id.0.to_string()))?;

        for (_, records) in records_map.iter_mut() {
            if let Some(idx) = records.iter().position(|r| r.id == *record_id) {
                records.remove(idx);
                return Ok(());
            }
        }

        Err(DnsError::RecordNotFound(record_id.0.to_string()))
    }

    async fn list_records(&self, zone_id: &ZoneId) -> DnsResult<Vec<DnsRecord>> {
        let records_map = self
            .records
            .get(&zone_id.0.to_string())
            .ok_or_else(|| DnsError::ZoneNotFound(zone_id.0.to_string()))?;

        let mut results = Vec::new();
        for (_, records) in records_map.iter() {
            results.extend(records.clone());
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecordData;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_create_zone() {
        let zone_manager = DnsZone::new();
        let zone = Zone::new(DomainName("example.com".to_string()));

        let _zone_id = zone_manager.create_zone(zone).await.unwrap();
        assert_eq!(zone_manager.zone_count(), 1);
    }

    #[tokio::test]
    async fn test_get_zone_by_name() {
        let zone_manager = DnsZone::new();
        let zone = Zone::new(DomainName("example.com".to_string()));
        let domain = zone.name.clone();

        zone_manager.create_zone(zone).await.unwrap();
        let retrieved = zone_manager.get_zone_by_name(&domain).await.unwrap();
        assert_eq!(retrieved.name, domain);
    }

    #[tokio::test]
    async fn test_create_record() {
        let zone_manager = DnsZone::new();
        let zone = Zone::new(DomainName("example.com".to_string()));
        let zone_id = zone_manager.create_zone(zone).await.unwrap();

        let record = DnsRecord::new(
            "www".to_string(),
            RecordType::A,
            RecordData::A(Ipv4Addr::new(192, 0, 2, 1)),
            3600,
        );

        let record_id = zone_manager.create_record(&zone_id, record).await.unwrap();
        assert_eq!(zone_manager.record_count(&zone_id), 1);
    }

    #[tokio::test]
    async fn test_get_records_by_type() {
        let zone_manager = DnsZone::new();
        let zone = Zone::new(DomainName("example.com".to_string()));
        let zone_id = zone_manager.create_zone(zone).await.unwrap();

        let record1 = DnsRecord::new(
            "www".to_string(),
            RecordType::A,
            RecordData::A(Ipv4Addr::new(192, 0, 2, 1)),
            3600,
        );

        let record2 = DnsRecord::new(
            "mail".to_string(),
            RecordType::Cname,
            RecordData::Cname("mail.example.com".to_string()),
            3600,
        );

        let _ = zone_manager.create_record(&zone_id, record1).await.unwrap();
        let _ = zone_manager.create_record(&zone_id, record2).await.unwrap();

        let a_records = zone_manager.get_records_by_type(&zone_id, RecordType::A).await.unwrap();
        assert_eq!(a_records.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_record() {
        let zone_manager = DnsZone::new();
        let zone = Zone::new(DomainName("example.com".to_string()));
        let zone_id = zone_manager.create_zone(zone).await.unwrap();

        let record = DnsRecord::new(
            "www".to_string(),
            RecordType::A,
            RecordData::A(Ipv4Addr::new(192, 0, 2, 1)),
            3600,
        );

        let _record_id = zone_manager.create_record(&zone_id, record).await.unwrap();
        assert_eq!(zone_manager.record_count(&zone_id), 1);

        zone_manager.delete_record(&zone_id, &_record_id).await.unwrap();
        assert_eq!(zone_manager.record_count(&zone_id), 0);
    }
}
