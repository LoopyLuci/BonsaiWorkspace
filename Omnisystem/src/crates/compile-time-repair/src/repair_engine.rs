//! Repair engine for applying automatic fixes

use crate::analyzer::CompileError;
use crate::database::{RepairDatabase, RepairRecord};
use anyhow::Result;
use std::fs;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Repair {
    pub pattern_id: String,
    pub error: CompileError,
    pub suggested_fix: String,
    pub confidence: f64,
}

pub struct RepairEngine {
    db: Mutex<RepairDatabase>,
}

impl RepairEngine {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = RepairDatabase::new(db_path)?;
        Ok(Self { db: Mutex::new(db) })
    }

    /// Find repair patterns for given errors
    pub fn find_repairs(&self, errors: &[CompileError]) -> Result<Vec<Repair>> {
        let mut repairs = Vec::new();

        for error in errors {
            let repair = self.create_repair(error)?;
            repairs.push(repair);
        }

        Ok(repairs)
    }

    /// Create a repair for an error
    fn create_repair(&self, error: &CompileError) -> Result<Repair> {
        use crate::analyzer::ErrorType;

        let (pattern_id, suggested_fix, confidence) = match error.error_type {
            ErrorType::UnusedVariable => {
                ("unused_var_prefix".to_string(),
                 format!("let _{} = ...;", error.code_snippet.trim()),
                 0.95)
            }
            ErrorType::MissingReturn => {
                ("missing_return_add".to_string(),
                 "Add 'return value;' before closing brace".to_string(),
                 0.85)
            }
            ErrorType::UnusedImport => {
                ("remove_unused_import".to_string(),
                 "Remove the import or add #[allow(unused_imports)]".to_string(),
                 0.90)
            }
            ErrorType::NullPointerDereference => {
                ("add_null_check".to_string(),
                 "Add if let Some(...) = ... or match pattern".to_string(),
                 0.75)
            }
            ErrorType::BufferOverflow => {
                ("add_bounds_check".to_string(),
                 "Add len() check before array access".to_string(),
                 0.70)
            }
            ErrorType::UndefinedFunction => {
                ("define_function".to_string(),
                 "Define the function or import it from another module".to_string(),
                 0.65)
            }
            ErrorType::TypeMismatch => {
                ("fix_type".to_string(),
                 "Cast or convert the value to the correct type".to_string(),
                 0.70)
            }
            ErrorType::LogicError => {
                ("review_logic".to_string(),
                 "Review the logic and fix the condition".to_string(),
                 0.55)
            }
            ErrorType::DeadCode => {
                ("remove_dead_code".to_string(),
                 "Remove unreachable code".to_string(),
                 0.80)
            }
            ErrorType::IncorrectDocComment => {
                ("fix_doc_comment".to_string(),
                 "Fix the documentation comment format".to_string(),
                 0.85)
            }
        };

        Ok(Repair {
            pattern_id,
            error: error.clone(),
            suggested_fix,
            confidence,
        })
    }

    /// Apply repairs to source file
    pub async fn apply_repairs(&self, source_path: &str, repairs: &[Repair]) -> Result<Vec<String>> {
        let mut source = fs::read_to_string(source_path)?;
        let mut applied = Vec::new();
        let mut records: Vec<RepairRecord> = Vec::new();

        // Apply repairs in reverse line order to avoid offset issues
        let mut sorted_repairs = repairs.to_vec();
        sorted_repairs.sort_by(|a, b| b.error.line.cmp(&a.error.line));

        for repair in sorted_repairs {
            // Apply the repair
            let outcome = self.apply_single_repair(&source, &repair);
            let success = outcome.is_ok();
            if let Ok(new_source) = outcome {
                source = new_source;
                applied.push(repair.pattern_id.clone());
            }

            records.push(RepairRecord {
                id: next_record_id(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                file_path: source_path.to_string(),
                error_type: format!("{:?}", repair.error.error_type),
                repair_applied: repair.pattern_id,
                confidence: repair.confidence,
                success,
            });
        }

        // Write back to file
        fs::write(source_path, source)?;

        // Record the applied repairs in the persistent history database.
        {
            let mut db = self
                .db
                .lock()
                .map_err(|_| anyhow::anyhow!("repair database lock poisoned"))?;
            for record in records {
                db.add_record(record)?;
            }
        }

        Ok(applied)
    }

    /// Apply a single repair to source
    fn apply_single_repair(&self, source: &str, repair: &Repair) -> Result<String> {
        let lines: Vec<&str> = source.lines().collect();
        let mut result = String::new();

        for (i, line) in lines.iter().enumerate() {
            if i + 1 == repair.error.line {
                // Apply the repair to this line
                let fixed_line = self.fix_line(line, repair)?;
                result.push_str(&fixed_line);
            } else {
                result.push_str(line);
            }
            result.push('\n');
        }

        Ok(result)
    }

    /// Fix a single line based on repair pattern
    fn fix_line(&self, line: &str, repair: &Repair) -> Result<String> {
        use crate::analyzer::ErrorType;

        match repair.error.error_type {
            ErrorType::UnusedVariable => {
                // Add underscore prefix to unused variable
                Ok(line.replace("let ", "let _"))
            }
            ErrorType::UnusedImport => {
                // Comment out the import
                Ok(format!("// {}", line))
            }
            ErrorType::DeadCode => {
                // Comment out dead code
                Ok(format!("/* {} */", line))
            }
            _ => {
                // For other errors, add a comment with suggestion
                Ok(format!("{} // TODO: {}", line, repair.suggested_fix))
            }
        }
    }

    /// Get repair statistics, computed from the persistent repair history.
    pub async fn get_statistics(&self) -> Result<crate::RepairStatistics> {
        let db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("repair database lock poisoned"))?;
        Ok(db.get_statistics())
    }
}

/// Generate a reasonably-unique id for a repair record without pulling in a
/// UUID dependency: nanosecond timestamp + a process-local counter.
fn next_record_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    format!("repair-{nanos}-{seq}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{CompileError, ErrorType};

    #[test]
    fn test_repair_engine_creation() {
        let engine = RepairEngine::new(".omnisystem/test.db");
        assert!(engine.is_ok());
    }

    #[test]
    fn test_create_repair() -> Result<()> {
        let engine = RepairEngine::new(".omnisystem/test.db")?;
        let error = CompileError {
            error_type: ErrorType::UnusedVariable,
            line: 1,
            column: 0,
            message: "unused variable".to_string(),
            code_snippet: "let x = 5;".to_string(),
        };

        let repair = engine.create_repair(&error)?;
        assert!(repair.confidence > 0.0);
        Ok(())
    }

    #[test]
    fn test_fix_line() -> Result<()> {
        let engine = RepairEngine::new(".omnisystem/test.db")?;
        let error = CompileError {
            error_type: ErrorType::UnusedVariable,
            line: 1,
            column: 0,
            message: "unused variable".to_string(),
            code_snippet: "let x = 5;".to_string(),
        };

        let repair = engine.create_repair(&error)?;
        let fixed = engine.fix_line("let x = 5;", &repair)?;
        assert!(fixed.contains("_"));
        Ok(())
    }

    #[tokio::test]
    async fn test_apply_repairs_records_real_statistics() -> Result<()> {
        let unique = next_record_id();
        let tmp_dir = std::env::temp_dir().join(format!("ctr-stats-test-{unique}"));
        std::fs::create_dir_all(&tmp_dir)?;

        let db_path = tmp_dir.join("history.db").to_string_lossy().to_string();
        let source_path = tmp_dir.join("sample.rs").to_string_lossy().to_string();
        std::fs::write(&source_path, "let x = 5;\nfn main() {}\n")?;

        let engine = RepairEngine::new(&db_path)?;

        // Before any repairs, statistics must be all-zero.
        let before = engine.get_statistics().await?;
        assert_eq!(before.total_repairs, 0);

        let error = CompileError {
            error_type: ErrorType::UnusedVariable,
            line: 1,
            column: 0,
            message: "unused variable".to_string(),
            code_snippet: "let x = 5;".to_string(),
        };
        let repairs = engine.find_repairs(&[error])?;
        assert_eq!(repairs.len(), 1);

        let applied = engine.apply_repairs(&source_path, &repairs).await?;
        assert_eq!(applied.len(), 1);

        // Statistics must now reflect the applied repair -- not all zero.
        let after = engine.get_statistics().await?;
        assert_eq!(after.total_repairs, 1);
        assert_eq!(after.successful_repairs, 1);
        assert_eq!(after.failed_repairs, 0);
        assert!(after.average_confidence > 0.0);
        assert_eq!(after.most_common_error.as_deref(), Some("UnusedVariable"));

        // A fresh RepairEngine reading the same db_path sees the persisted history.
        let reopened = RepairEngine::new(&db_path)?;
        let reopened_stats = reopened.get_statistics().await?;
        assert_eq!(reopened_stats.total_repairs, 1);

        std::fs::remove_dir_all(&tmp_dir).ok();
        Ok(())
    }
}
