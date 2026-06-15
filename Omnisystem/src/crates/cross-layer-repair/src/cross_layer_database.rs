//! Cross-layer repair history database

use crate::SystemLayer;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossLayerRepairRecord {
    pub id: String,
    pub timestamp: String,
    pub origin_layer: String,
    pub affected_layers: Vec<String>,
    pub repairs_applied: Vec<String>,
    pub cascade_depth: usize,
    pub success: bool,
    pub rollback_count: usize,
}

pub struct CrossLayerDatabase {
    path: String,
    records: Vec<CrossLayerRepairRecord>,
}

impl CrossLayerDatabase {
    pub fn new(db_path: &str) -> Result<Self> {
        let records = if std::path::Path::new(db_path).exists() {
            let content = fs::read_to_string(db_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self {
            path: db_path.to_string(),
            records,
        })
    }

    pub fn add_record(&mut self, record: CrossLayerRepairRecord) -> Result<()> {
        self.records.push(record);
        self.save()?;
        Ok(())
    }

    pub fn get_records_by_origin(&self, origin: &SystemLayer) -> Vec<&CrossLayerRepairRecord> {
        let origin_str = format!("{:?}", origin);
        self.records
            .iter()
            .filter(|r| r.origin_layer == origin_str)
            .collect()
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.records)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn get_statistics(&self) -> DatabaseStatistics {
        let total = self.records.len() as u64;
        let successful = self.records.iter().filter(|r| r.success).count() as u64;
        let failed = total - successful;
        let avg_cascade = if !self.records.is_empty() {
            self.records.iter().map(|r| r.cascade_depth as f64).sum::<f64>()
                / self.records.len() as f64
        } else {
            0.0
        };

        DatabaseStatistics {
            total_repairs: total,
            successful: successful,
            failed: failed,
            avg_cascade_depth: avg_cascade,
        }
    }
}

#[derive(Debug)]
pub struct DatabaseStatistics {
    pub total_repairs: u64,
    pub successful: u64,
    pub failed: u64,
    pub avg_cascade_depth: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() -> Result<()> {
        let db = CrossLayerDatabase::new(".omnisystem/test_cross_layer.db")?;
        assert_eq!(db.records.len(), 0);
        Ok(())
    }

    #[test]
    fn test_add_record() -> Result<()> {
        let mut db = CrossLayerDatabase::new(".omnisystem/test_cross_layer_2.db")?;
        let record = CrossLayerRepairRecord {
            id: "test-1".to_string(),
            timestamp: "2026-06-10T00:00:00Z".to_string(),
            origin_layer: "UOSC".to_string(),
            affected_layers: vec!["Omnisystem".to_string()],
            repairs_applied: vec!["fix-1".to_string()],
            cascade_depth: 1,
            success: true,
            rollback_count: 0,
        };

        db.add_record(record)?;
        assert_eq!(db.records.len(), 1);
        Ok(())
    }
}
