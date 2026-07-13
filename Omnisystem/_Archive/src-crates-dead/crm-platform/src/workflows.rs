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
            .map(|w| WorkflowExecution {
                workflow_id: w.id.clone(),
                status: "completed".to_string(),
            })
            .collect()
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
}
