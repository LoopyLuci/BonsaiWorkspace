//! Repair engine for applying automatic fixes

use crate::analyzer::CompileError;
use crate::patterns::RepairPattern;
use anyhow::Result;
use std::fs;

#[derive(Debug, Clone)]
pub struct Repair {
    pub pattern_id: String,
    pub error: CompileError,
    pub suggested_fix: String,
    pub confidence: f64,
}

pub struct RepairEngine {
    db_path: String,
}

impl RepairEngine {
    pub fn new(db_path: &str) -> Result<Self> {
        Ok(Self {
            db_path: db_path.to_string(),
        })
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

        // Apply repairs in reverse line order to avoid offset issues
        let mut sorted_repairs = repairs.to_vec();
        sorted_repairs.sort_by(|a, b| b.error.line.cmp(&a.error.line));

        for repair in sorted_repairs {
            // Apply the repair
            if let Ok(new_source) = self.apply_single_repair(&source, &repair) {
                source = new_source;
                applied.push(repair.pattern_id);
            }
        }

        // Write back to file
        fs::write(source_path, source)?;

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

    /// Get repair statistics
    pub async fn get_statistics(&self) -> Result<crate::RepairStatistics> {
        Ok(crate::RepairStatistics {
            total_repairs: 0,
            successful_repairs: 0,
            failed_repairs: 0,
            average_confidence: 0.0,
            most_common_error: None,
        })
    }
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
}
