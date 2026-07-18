//! Repair history database

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairRecord {
    pub id: String,
    pub timestamp: String,
    pub file_path: String,
    pub error_type: String,
    pub repair_applied: String,
    pub confidence: f64,
    pub success: bool,
}

pub struct RepairDatabase {
    path: String,
    records: Vec<RepairRecord>,
}

impl RepairDatabase {
    pub fn new(db_path: &str) -> Result<Self> {
        // Ensure directory exists
        if let Some(parent) = Path::new(db_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let records = if Path::new(db_path).exists() {
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

    pub fn add_record(&mut self, record: RepairRecord) -> Result<()> {
        self.records.push(record);
        self.save()?;
        Ok(())
    }

    pub fn get_records_for_file(&self, file_path: &str) -> Vec<&RepairRecord> {
        self.records.iter().filter(|r| r.file_path == file_path).collect()
    }

    pub fn get_all_records(&self) -> &[RepairRecord] {
        &self.records
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.records)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn get_statistics(&self) -> RepairStatistics {
        let total = self.records.len() as u64;
        let successful = self.records.iter().filter(|r| r.success).count() as u64;
        let failed = total - successful;
        let avg_confidence = if !self.records.is_empty() {
            self.records.iter().map(|r| r.confidence).sum::<f64>() / self.records.len() as f64
        } else {
            0.0
        };

        RepairStatistics {
            total_repairs: total,
            successful_repairs: successful,
            failed_repairs: failed,
            average_confidence: avg_confidence,
            most_common_error: self.find_most_common_error(),
        }
    }

    fn find_most_common_error(&self) -> Option<String> {
        if self.records.is_empty() {
            return None;
        }

        let mut error_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for record in &self.records {
            *error_counts.entry(record.error_type.clone()).or_insert(0) += 1;
        }

        error_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(error, _)| error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairStatistics {
    pub total_repairs: u64,
    pub successful_repairs: u64,
    pub failed_repairs: u64,
    pub average_confidence: f64,
    pub most_common_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repair_database_creation() -> Result<()> {
        let db = RepairDatabase::new(".omnisystem/test_repair.db")?;
        let stats = db.get_statistics();
        assert_eq!(stats.total_repairs, 0);
        Ok(())
    }

    #[test]
    fn test_add_record() -> Result<()> {
        let mut db = RepairDatabase::new(".omnisystem/test_repair2.db")?;
        let record = RepairRecord {
            id: "test-1".to_string(),
            timestamp: "2026-06-10T00:00:00Z".to_string(),
            file_path: "test.rs".to_string(),
            error_type: "UnusedVariable".to_string(),
            repair_applied: "unused_var_prefix".to_string(),
            confidence: 0.95,
            success: true,
        };
        db.add_record(record)?;
        assert_eq!(db.get_all_records().len(), 1);
        Ok(())
    }
}
