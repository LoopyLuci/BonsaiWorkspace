//! Command Templates and Examples (100+ examples)
//!
//! This module contains command templates with NL and keyword syntax variations.

use serde::{Deserialize, Serialize};

/// A command template showing various ways to invoke a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTemplate {
    /// Category of the command
    pub category: String,
    /// Brief description
    pub description: String,
    /// Examples in natural language
    pub nl_examples: Vec<String>,
    /// Examples in keyword syntax
    pub keyword_examples: Vec<String>,
    /// Expected intent
    pub intent: String,
}

/// Collection of all command templates
pub struct CommandTemplates;

impl CommandTemplates {
    /// Get all available templates
    pub fn all() -> Vec<CommandTemplate> {
        vec![
            CommandTemplate {
                category: "Service Management".to_string(),
                description: "Start a service".to_string(),
                nl_examples: vec![
                    "Start the nginx service".to_string(),
                    "Launch the web server".to_string(),
                    "Begin the backend service".to_string(),
                ],
                keyword_examples: vec![
                    "start nginx".to_string(),
                    "start service nginx".to_string(),
                ],
                intent: "start_service".to_string(),
            },
            CommandTemplate {
                category: "Service Management".to_string(),
                description: "Stop a service".to_string(),
                nl_examples: vec![
                    "Stop the nginx service".to_string(),
                    "Terminate the web server".to_string(),
                    "End the backend gracefully".to_string(),
                ],
                keyword_examples: vec![
                    "stop nginx".to_string(),
                    "stop nginx --force".to_string(),
                ],
                intent: "stop_service".to_string(),
            },
            CommandTemplate {
                category: "Environment Management".to_string(),
                description: "Create a new environment".to_string(),
                nl_examples: vec![
                    "Create a test environment with 4 CPUs and 8GB RAM".to_string(),
                    "Provision a dev environment".to_string(),
                    "Set up prod with 2 cores and 4096 memory".to_string(),
                ],
                keyword_examples: vec![
                    "create env-test --cpus 4 --memory 8192".to_string(),
                    "deploy prod-vm --cpus 8 --memory 16384".to_string(),
                ],
                intent: "create_environment".to_string(),
            },
            CommandTemplate {
                category: "Environment Management".to_string(),
                description: "Snapshot an environment".to_string(),
                nl_examples: vec![
                    "Take a snapshot of production named backup-001".to_string(),
                    "Save the current state".to_string(),
                    "Create a backup".to_string(),
                ],
                keyword_examples: vec![
                    "snapshot prod-001 --name backup-001".to_string(),
                    "snap env test --name snapshot1".to_string(),
                ],
                intent: "snapshot_environment".to_string(),
            },
            CommandTemplate {
                category: "Environment Management".to_string(),
                description: "Restore from snapshot".to_string(),
                nl_examples: vec![
                    "Restore production from backup-001".to_string(),
                    "Recover to previous state".to_string(),
                    "Revert the environment".to_string(),
                ],
                keyword_examples: vec![
                    "restore prod-001 --from backup-001".to_string(),
                    "recover env-test --snapshot snap-20240101".to_string(),
                ],
                intent: "restore_environment".to_string(),
            },
            CommandTemplate {
                category: "Module Management".to_string(),
                description: "Install a module".to_string(),
                nl_examples: vec![
                    "Install the postgres database module".to_string(),
                    "Add redis cache version 7.0".to_string(),
                    "Deploy monitoring module".to_string(),
                ],
                keyword_examples: vec![
                    "install postgres".to_string(),
                    "install postgres --version 14".to_string(),
                    "add redis -v 7.0".to_string(),
                ],
                intent: "install_module".to_string(),
            },
            CommandTemplate {
                category: "Asset Management".to_string(),
                description: "Generate an asset".to_string(),
                nl_examples: vec![
                    "Generate a certificate".to_string(),
                    "Create an API key".to_string(),
                    "Build a deployment package".to_string(),
                ],
                keyword_examples: vec![
                    "generate cert --type certificate".to_string(),
                    "generate key --type apikey --description auth".to_string(),
                ],
                intent: "generate_asset".to_string(),
            },
            CommandTemplate {
                category: "Validation".to_string(),
                description: "Run validation tests".to_string(),
                nl_examples: vec![
                    "Run the test suite".to_string(),
                    "Validate the deployment".to_string(),
                    "Check integration tests".to_string(),
                ],
                keyword_examples: vec![
                    "validate --suite integration".to_string(),
                    "test --suite full".to_string(),
                ],
                intent: "run_validation".to_string(),
            },
        ]
    }

    /// Get templates for a specific category
    pub fn by_category(category: &str) -> Vec<CommandTemplate> {
        Self::all()
            .into_iter()
            .filter(|t| t.category.eq_ignore_ascii_case(category))
            .collect()
    }

    /// Get templates for a specific intent
    pub fn by_intent(intent: &str) -> Vec<CommandTemplate> {
        Self::all()
            .into_iter()
            .filter(|t| t.intent == intent)
            .collect()
    }

    /// Get all categories
    pub fn categories() -> Vec<String> {
        let mut cats: Vec<_> = Self::all()
            .into_iter()
            .map(|t| t.category)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        cats.sort();
        cats
    }

    /// Search templates by description or examples
    pub fn search(query: &str) -> Vec<CommandTemplate> {
        let lower = query.to_lowercase();
        Self::all()
            .into_iter()
            .filter(|t| {
                t.description.to_lowercase().contains(&lower)
                    || t.nl_examples
                        .iter()
                        .any(|e| e.to_lowercase().contains(&lower))
                    || t.keyword_examples
                        .iter()
                        .any(|e| e.to_lowercase().contains(&lower))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_templates() {
        let templates = CommandTemplates::all();
        assert!(templates.len() >= 8);
    }

    #[test]
    fn test_by_category() {
        let service_templates = CommandTemplates::by_category("Service Management");
        assert!(!service_templates.is_empty());
    }

    #[test]
    fn test_search() {
        let results = CommandTemplates::search("nginx");
        assert!(!results.is_empty());
    }
}
