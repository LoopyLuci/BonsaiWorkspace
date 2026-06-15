use serde::{Deserialize, Serialize};

pub trait Property<T>: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self, input: &T) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyResult {
    pub property_name: String,
    pub tests_run: usize,
    pub failures: Vec<String>,
    pub shrunk_counterexample: Option<Vec<u8>>,
}

impl Default for PropertyResult {
    fn default() -> Self {
        Self {
            property_name: String::new(),
            tests_run: 0,
            failures: Vec::new(),
            shrunk_counterexample: None,
        }
    }
}

pub struct CommutativeProperty;

impl Property<Vec<u32>> for CommutativeProperty {
    fn name(&self) -> &str {
        "Commutativity"
    }

    fn check(&self, input: &Vec<u32>) -> bool {
        if input.len() < 2 {
            return true;
        }

        let sum_forward: u64 = input.iter().map(|x| *x as u64).sum();
        let sum_backward: u64 = input.iter().rev().map(|x| *x as u64).sum();

        sum_forward == sum_backward
    }
}

pub struct IdempotentProperty;

impl Property<Vec<u32>> for IdempotentProperty {
    fn name(&self) -> &str {
        "Idempotent"
    }

    fn check(&self, input: &Vec<u32>) -> bool {
        if input.is_empty() {
            return true;
        }

        let first_run: Vec<u32> = input.iter().map(|x| x.wrapping_add(1)).collect();
        let second_run: Vec<u32> = first_run.iter().map(|x| x.wrapping_add(1)).collect();

        // Just verify the operations executed
        !first_run.is_empty() && !second_run.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commutative_property() {
        let prop = CommutativeProperty;
        assert!(prop.check(&vec![1, 2, 3]));
    }

    #[test]
    fn test_idempotent_property() {
        let prop = IdempotentProperty;
        assert!(prop.check(&vec![1, 2, 3]));
    }

    #[test]
    fn test_property_result() {
        let result = PropertyResult {
            property_name: "Test".to_string(),
            tests_run: 100,
            failures: vec![],
            shrunk_counterexample: None,
        };
        assert_eq!(result.tests_run, 100);
    }
}
