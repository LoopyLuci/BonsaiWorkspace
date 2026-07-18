use crate::config::PropertyTestConfig;
use crate::generator::InputGenerator;
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

/// Checks that sorting is idempotent: `sort(sort(v)) == sort(v)` for any
/// input. (The original implementation here didn't check idempotence at
/// all -- it applied `wrapping_add(1)` twice and only asserted the two
/// resulting vectors were non-empty, which is true for any non-empty
/// input regardless of whether the underlying operation is idempotent.)
pub struct IdempotentProperty;

impl Property<Vec<u32>> for IdempotentProperty {
    fn name(&self) -> &str {
        "Idempotent"
    }

    fn check(&self, input: &Vec<u32>) -> bool {
        let mut once = input.clone();
        once.sort_unstable();

        let mut twice = once.clone();
        twice.sort_unstable();

        once == twice
    }
}

/// Runs a [`Property`] against `config.num_tests` randomly generated
/// inputs (via [`InputGenerator`]), collecting any failures and shrinking
/// the first counterexample found.
pub struct PropertyTester {
    config: PropertyTestConfig,
    generator: InputGenerator,
}

impl PropertyTester {
    pub fn new(config: PropertyTestConfig) -> Self {
        Self {
            config,
            generator: InputGenerator::new(),
        }
    }

    pub fn run(&self, property: &dyn Property<Vec<u32>>) -> PropertyResult {
        let mut failures = Vec::new();
        let mut shrunk_counterexample = None;

        for _ in 0..self.config.num_tests {
            let bytes = self.generator.generate("numbers");
            let input: Vec<u32> = bytes.iter().map(|b| *b as u32).collect();

            if !property.check(&input) {
                failures.push(format!("{:?}", input));
                if shrunk_counterexample.is_none() {
                    shrunk_counterexample = self.generator.shrink(&bytes);
                }
            }
        }

        PropertyResult {
            property_name: property.name().to_string(),
            tests_run: self.config.num_tests,
            failures,
            shrunk_counterexample,
        }
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
    fn test_idempotent_property_holds_for_sorting() {
        let prop = IdempotentProperty;
        assert!(prop.check(&vec![3, 1, 2]));
        assert!(prop.check(&vec![]));
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

    #[test]
    fn test_property_tester_runs_configured_test_count() {
        let config = PropertyTestConfig {
            num_tests: 25,
            ..PropertyTestConfig::default()
        };
        let tester = PropertyTester::new(config);

        let result = tester.run(&CommutativeProperty);
        assert_eq!(result.tests_run, 25);
        // CommutativeProperty (sum forward == sum backward) holds for any
        // input by construction, so a real run should find zero failures.
        assert!(result.failures.is_empty());
    }

    #[test]
    fn test_property_tester_reports_failures_for_a_broken_property() {
        struct AlwaysFails;
        impl Property<Vec<u32>> for AlwaysFails {
            fn name(&self) -> &str {
                "AlwaysFails"
            }
            fn check(&self, _input: &Vec<u32>) -> bool {
                false
            }
        }

        let config = PropertyTestConfig {
            num_tests: 5,
            ..PropertyTestConfig::default()
        };
        let tester = PropertyTester::new(config);
        let result = tester.run(&AlwaysFails);

        assert_eq!(result.tests_run, 5);
        assert_eq!(result.failures.len(), 5);
        assert!(result.shrunk_counterexample.is_some());
    }
}
