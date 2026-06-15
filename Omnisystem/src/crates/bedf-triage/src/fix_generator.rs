use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFix {
    pub description: String,
    pub code_change: String,
    pub confidence: f64,
}

pub struct FixGenerator;

impl FixGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_fix(&self, stack_trace: &str) -> Option<GeneratedFix> {
        if stack_trace.contains("index out of bounds") {
            return Some(GeneratedFix {
                description: "Add bounds checking before array access".to_string(),
                code_change: "if index < array.len() { ... }".to_string(),
                confidence: 0.95,
            });
        }

        if stack_trace.contains("null pointer") {
            return Some(GeneratedFix {
                description: "Add null check before dereferencing".to_string(),
                code_change: "if ptr != null { ... }".to_string(),
                confidence: 0.92,
            });
        }

        if stack_trace.contains("use after free") {
            return Some(GeneratedFix {
                description: "Ensure pointer is valid before use".to_string(),
                code_change: "let ptr = allocate(); ... use ptr".to_string(),
                confidence: 0.88,
            });
        }

        if stack_trace.contains("deadlock") {
            return Some(GeneratedFix {
                description: "Reorder lock acquisitions".to_string(),
                code_change: "lock_a.lock(); lock_b.lock();".to_string(),
                confidence: 0.85,
            });
        }

        None
    }
}

impl Default for FixGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_check_fix() {
        let gen = FixGenerator::new();
        let fix = gen.generate_fix("thread panicked at 'index out of bounds'");
        assert!(fix.is_some());
        let f = fix.unwrap();
        assert!(f.description.contains("bounds"));
    }

    #[test]
    fn test_null_check_fix() {
        let gen = FixGenerator::new();
        let fix = gen.generate_fix("null pointer dereference");
        assert!(fix.is_some());
    }

    #[test]
    fn test_unknown_error_no_fix() {
        let gen = FixGenerator::new();
        let fix = gen.generate_fix("unknown error ABC123");
        assert!(fix.is_none());
    }

    #[test]
    fn test_confidence_scores() {
        let gen = FixGenerator::new();
        let fix = gen.generate_fix("index out of bounds");
        assert!(fix.is_some());
        assert!(fix.unwrap().confidence > 0.9);
    }
}
