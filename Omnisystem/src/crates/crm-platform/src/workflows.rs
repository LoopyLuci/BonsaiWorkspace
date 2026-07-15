//! Workflow Automation

use crate::cdp::Customer;

pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub steps: Vec<WorkflowStep>,
}

pub struct WorkflowStep {
    pub id: String,
    pub action: String,
    pub conditions: Vec<String>,
}

pub struct WorkflowEngine {
    workflows: std::sync::Arc<parking_lot::Mutex<Vec<WorkflowDefinition>>>,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            workflows: std::sync::Arc::new(parking_lot::Mutex::new(Vec::new())),
        }
    }

    pub fn register_workflow(&self, workflow: WorkflowDefinition) {
        self.workflows.lock().push(workflow);
    }

    pub fn execute(&self, customer: &Customer) -> Vec<WorkflowExecution> {
        let workflows = self.workflows.lock();
        workflows
            .iter()
            .map(|w| {
                let all_conditions_met = w
                    .steps
                    .iter()
                    .all(|step| Self::step_conditions_met(step, customer));

                WorkflowExecution {
                    workflow_id: w.id.clone(),
                    status: if all_conditions_met {
                        "completed".to_string()
                    } else {
                        "skipped".to_string()
                    },
                }
            })
            .collect()
    }

    fn step_conditions_met(step: &WorkflowStep, customer: &Customer) -> bool {
        step.conditions
            .iter()
            .all(|condition| Self::condition_met(condition, customer))
    }

    /// Evaluate a single condition string against a customer.
    ///
    /// Supported forms:
    /// - `"always"` — unconditionally passes.
    /// - `"segment:<name>"` — passes if the customer is in segment `<name>`.
    /// - `"<key>=<value>"` — passes if the customer's attribute `<key>` equals `<value>`.
    /// - any other bare string — treated as a segment name.
    fn condition_met(condition: &str, customer: &Customer) -> bool {
        if condition == "always" {
            return true;
        }

        if let Some(segment_name) = condition.strip_prefix("segment:") {
            return customer.is_in_segment(segment_name);
        }

        if let Some((key, value)) = condition.split_once('=') {
            return customer.get_attribute(key) == Some(value);
        }

        customer.is_in_segment(condition)
    }
}

pub struct WorkflowExecution {
    pub workflow_id: String,
    pub status: String,
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdp::{Customer, CustomerId, Segment};
    use std::collections::HashMap;

    #[test]
    fn test_workflow_engine() {
        let engine = WorkflowEngine::new();
        let workflow = WorkflowDefinition {
            id: "wf1".to_string(),
            name: "Welcome".to_string(),
            steps: vec![],
        };
        engine.register_workflow(workflow);
        assert_eq!(engine.workflows.lock().len(), 1);
    }

    #[test]
    fn test_workflow_with_no_steps_completes() {
        let engine = WorkflowEngine::new();
        engine.register_workflow(WorkflowDefinition {
            id: "wf-empty".to_string(),
            name: "No-op".to_string(),
            steps: vec![],
        });

        let customer = Customer::new(CustomerId::Email("test@example.com".to_string()));
        let executions = engine.execute(&customer);
        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].status, "completed");
    }

    #[test]
    fn test_workflow_skipped_when_segment_condition_not_met() {
        let engine = WorkflowEngine::new();
        engine.register_workflow(WorkflowDefinition {
            id: "wf-vip".to_string(),
            name: "VIP Perks".to_string(),
            steps: vec![WorkflowStep {
                id: "step1".to_string(),
                action: "send_email".to_string(),
                conditions: vec!["segment:vip".to_string()],
            }],
        });

        let customer = Customer::new(CustomerId::Email("nobody@example.com".to_string()));
        let executions = engine.execute(&customer);

        assert_eq!(executions.len(), 1);
        assert_ne!(executions[0].status, "completed");
        assert_eq!(executions[0].status, "skipped");
    }

    #[test]
    fn test_workflow_completed_when_segment_condition_met() {
        let engine = WorkflowEngine::new();
        engine.register_workflow(WorkflowDefinition {
            id: "wf-vip".to_string(),
            name: "VIP Perks".to_string(),
            steps: vec![WorkflowStep {
                id: "step1".to_string(),
                action: "send_email".to_string(),
                conditions: vec!["segment:vip".to_string()],
            }],
        });

        let mut customer = Customer::new(CustomerId::Email("vip@example.com".to_string()));
        customer.add_to_segment(Segment {
            name: "vip".to_string(),
            entered_at: 0,
            metadata: HashMap::new(),
        });

        let executions = engine.execute(&customer);
        assert_eq!(executions[0].status, "completed");
    }

    #[test]
    fn test_workflow_attribute_condition() {
        let engine = WorkflowEngine::new();
        engine.register_workflow(WorkflowDefinition {
            id: "wf-plan".to_string(),
            name: "Plan Upsell".to_string(),
            steps: vec![WorkflowStep {
                id: "step1".to_string(),
                action: "send_upsell".to_string(),
                conditions: vec!["plan=free".to_string()],
            }],
        });

        let mut customer = Customer::new(CustomerId::Email("free@example.com".to_string()));
        customer.set_attribute("plan".to_string(), "pro".to_string());

        let executions = engine.execute(&customer);
        assert_eq!(executions[0].status, "skipped");
    }
}
