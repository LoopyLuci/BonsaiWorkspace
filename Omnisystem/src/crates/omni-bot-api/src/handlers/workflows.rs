//! Workflow orchestration and execution API handlers
//! Provides endpoints for workflow DAG management and real-time step execution tracking

use crate::error::{ApiError, ApiResult};
use crate::models::*;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Workflow execution engine
#[derive(Clone)]
pub struct WorkflowEngine {
    workflows: Arc<RwLock<HashMap<String, WorkflowDefinition>>>,
    executions: Arc<RwLock<HashMap<String, ExecutionContext>>>,
    execution_results: Arc<RwLock<HashMap<String, ExecutionResult>>>,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            execution_results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_workflow(&self, id: &str) -> Option<WorkflowDefinition> {
        self.workflows.read().await.get(id).cloned()
    }

    async fn save_workflow(&self, workflow: WorkflowDefinition) {
        self.workflows
            .write()
            .await
            .insert(workflow.id.clone(), workflow);
    }

    async fn list_all_workflows(&self, page: usize, per_page: usize) -> (Vec<WorkflowDefinition>, usize) {
        let workflows = self.workflows.read().await;
        let total = workflows.len();
        let start = page * per_page;
        let end = (start + per_page).min(total);

        let mut items: Vec<_> = workflows.values().cloned().collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        (items[start..end].to_vec(), total)
    }

    async fn filter_workflows(
        &self,
        category: &str,
        tags: &[String],
        page: usize,
        per_page: usize,
    ) -> (Vec<WorkflowDefinition>, usize) {
        let workflows = self.workflows.read().await;

        let mut filtered: Vec<_> = workflows
            .values()
            .filter(|w| {
                let matches_category = category.is_empty() || w.category == category;
                let matches_tags = tags.is_empty() || tags.iter().any(|t| w.tags.contains(t));
                matches_category && matches_tags
            })
            .cloned()
            .collect();

        let total = filtered.len();
        filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let start = page * per_page;
        let end = (start + per_page).min(total);

        (filtered[start..end].to_vec(), total)
    }

    async fn create_execution(&self, context: ExecutionContext) {
        self.executions
            .write()
            .await
            .insert(context.execution_id.clone(), context);
    }

    async fn get_execution(&self, id: &str) -> Option<ExecutionContext> {
        self.executions.read().await.get(id).cloned()
    }

    async fn update_execution(&self, context: ExecutionContext) {
        self.executions
            .write()
            .await
            .insert(context.execution_id.clone(), context);
    }

    async fn save_execution_result(&self, result: ExecutionResult) {
        self.execution_results
            .write()
            .await
            .insert(result.context.execution_id.clone(), result);
    }

    async fn get_execution_result(&self, id: &str) -> Option<ExecutionResult> {
        self.execution_results.read().await.get(id).cloned()
    }
}

/// Query parameters for workflow listing
#[derive(Debug, Deserialize)]
pub struct WorkflowListQuery {
    pub category: Option<String>,
    pub tags: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

/// Initialize workflow engine
pub fn init_workflow_engine() -> WorkflowEngine {
    log::info!("Initializing Workflow Engine");
    WorkflowEngine::new()
}

/// List workflows with filtering (GET /workflows)
/// Filter by category, tags, and pagination support
pub async fn list_workflows(
    State(engine): State<WorkflowEngine>,
    Query(params): Query<WorkflowListQuery>,
) -> ApiResult<Json<WorkflowListResponse>> {
    let page = params.page.unwrap_or(0);
    let per_page = params.per_page.unwrap_or(20).min(100);
    let category = params.category.unwrap_or_default();

    let tags: Vec<String> = params
        .tags
        .as_ref()
        .map(|t| t.split(',').map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let (workflows, total) = engine.filter_workflows(&category, &tags, page, per_page).await;

    Ok(Json(WorkflowListResponse {
        workflows,
        total,
        page,
        per_page,
    }))
}

/// Execute workflow (POST /workflows/{id}/execute)
/// Triggers workflow with parameter substitution and DAG execution
pub async fn execute_workflow(
    State(engine): State<WorkflowEngine>,
    Path(workflow_id): Path<String>,
    Json(exec_req): Json<WorkflowExecutionRequest>,
) -> ApiResult<(StatusCode, Json<ExecutionContext>)> {
    // Get workflow definition
    let workflow = engine
        .get_workflow(&workflow_id)
        .await
        .ok_or_else(|| ApiError::WorkflowNotFound(workflow_id.clone()))?;

    // Validate DAG
    workflow
        .dag
        .validate()
        .map_err(|e| ApiError::InvalidWorkflowDAG(e))?;

    // Validate parameters
    validate_parameters(&workflow.parameters, &exec_req.parameters)?;

    let execution_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let context = ExecutionContext {
        execution_id: execution_id.clone(),
        workflow_id: workflow.id.clone(),
        workflow_version: workflow.version.clone(),
        parameters: exec_req.parameters.clone(),
        status: ExecutionStatus::Running,
        current_step: None,
        completed_steps: Vec::new(),
        failed_steps: Vec::new(),
        started_at: now,
        completed_at: None,
        duration_ms: None,
    };

    engine.create_execution(context.clone()).await;

    // Spawn async execution task
    let engine_clone = engine.clone();
    let workflow_clone = workflow.clone();
    let timeout = exec_req.timeout_secs.unwrap_or(3600);

    tokio::spawn(async move {
        execute_workflow_async(engine_clone, workflow_clone, execution_id, timeout).await
    });

    log::info!(
        "Started workflow execution: {} (workflow: {})",
        context.execution_id,
        workflow_id
    );

    Ok((StatusCode::ACCEPTED, Json(context)))
}

/// Get execution status (GET /workflows/{id}/executions/{exec_id})
/// Returns step-by-step progress and results
pub async fn get_execution_status(
    State(engine): State<WorkflowEngine>,
    Path((workflow_id, exec_id)): Path<(String, String)>,
) -> ApiResult<Json<ExecutionResult>> {
    // Verify workflow exists
    engine
        .get_workflow(&workflow_id)
        .await
        .ok_or_else(|| ApiError::WorkflowNotFound(workflow_id))?;

    // Get execution result
    engine
        .get_execution_result(&exec_id)
        .await
        .map(Json)
        .ok_or_else(|| ApiError::ExecutionFailed("Execution not found or still running".to_string()))
}

/// Create new workflow definition (POST /workflows/create)
/// Validates DAG structure and parameter definitions
pub async fn create_workflow(
    State(engine): State<WorkflowEngine>,
    Json(create_req): Json<WorkflowCreateRequest>,
) -> ApiResult<(StatusCode, Json<WorkflowDefinition>)> {
    // Validate DAG
    create_req
        .dag
        .validate()
        .map_err(|e| ApiError::InvalidWorkflowDAG(e))?;

    let workflow_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let workflow = WorkflowDefinition {
        id: workflow_id.clone(),
        name: create_req.name.clone(),
        description: create_req.description.clone(),
        version: "1.0.0".to_string(),
        category: create_req.category.clone(),
        tags: create_req.tags.clone(),
        dag: create_req.dag,
        parameters: create_req.parameters,
        created_at: now,
        updated_at: now,
    };

    engine.save_workflow(workflow.clone()).await;

    log::info!("Created workflow: {}", workflow_id);
    Ok((StatusCode::CREATED, Json(workflow)))
}

// ============================================================================
// Async Execution Engine
// ============================================================================

/// Execute workflow asynchronously with DAG traversal and rollback support
async fn execute_workflow_async(
    engine: WorkflowEngine,
    workflow: WorkflowDefinition,
    execution_id: String,
    timeout_secs: u32,
) {
    let mut context = engine
        .get_execution(&execution_id)
        .await
        .expect("Execution context must exist");

    let start_time = std::time::Instant::now();
    let mut step_results = Vec::new();

    // Execute steps in DAG order
    let execution_order = topological_sort(&workflow.dag);

    for step_id in execution_order {
        if start_time.elapsed().as_secs() > timeout_secs as u64 {
            context.status = ExecutionStatus::Failed;
            context.failed_steps.push(step_id.clone());
            break;
        }

        // Find step definition
        let step = match workflow.dag.steps.iter().find(|s| s.id == step_id) {
            Some(s) => s,
            None => continue,
        };

        context.current_step = Some(step_id.clone());
        engine.update_execution(context.clone()).await;

        // Execute step with retry logic
        let result = execute_step_with_retries(step, &context.parameters).await;

        match result {
            Ok(output) => {
                let step_result = StepResult {
                    step_id: step.id.clone(),
                    step_name: step.name.clone(),
                    status: StepExecutionStatus::Completed,
                    output: Some(output),
                    error: None,
                    started_at: Utc::now(),
                    completed_at: Some(Utc::now()),
                    duration_ms: None,
                    retries_used: 0,
                };
                context.completed_steps.push(step.id.clone());
                step_results.push(step_result);
            }
            Err(err) => {
                context.failed_steps.push(step.id.clone());
                step_results.push(StepResult {
                    step_id: step.id.clone(),
                    step_name: step.name.clone(),
                    status: StepExecutionStatus::Failed,
                    output: None,
                    error: Some(err.clone()),
                    started_at: Utc::now(),
                    completed_at: Some(Utc::now()),
                    duration_ms: None,
                    retries_used: 0,
                });

                // Handle failure action
                match step.on_failure {
                    Some(FailureAction::Rollback) => {
                        context.status = ExecutionStatus::RolledBack;
                        log::warn!("Workflow rolled back due to step failure: {}", step.id);
                        break;
                    }
                    Some(FailureAction::Halt) => {
                        context.status = ExecutionStatus::Failed;
                        break;
                    }
                    Some(FailureAction::Continue) | None => {
                        log::warn!("Continuing workflow after step failure: {}", step.id);
                    }
                }
            }
        }
    }

    // Finalize execution
    let now = Utc::now();
    context.completed_at = Some(now);
    context.duration_ms = Some(start_time.elapsed().as_millis() as u64);

    if context.status == ExecutionStatus::Running {
        context.status = if context.failed_steps.is_empty() {
            ExecutionStatus::Completed
        } else {
            ExecutionStatus::Failed
        };
    }

    let result = ExecutionResult {
        context: context.clone(),
        steps: step_results,
        final_output: None,
    };

    engine.save_execution_result(result).await;
    engine.update_execution(context).await;

    log::info!("Completed workflow execution: {}", execution_id);
}

/// Execute individual step with retry policy
async fn execute_step_with_retries(
    step: &WorkflowStep,
    parameters: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let retry_policy = step.retry_policy.as_ref();
    let max_retries = retry_policy.map(|p| p.max_retries).unwrap_or(0);

    for attempt in 0..=max_retries {
        match execute_step_action(step, parameters).await {
            Ok(output) => return Ok(output),
            Err(err) => {
                if attempt < max_retries {
                    let delay_ms = retry_policy
                        .map(|p| {
                            (p.initial_delay_ms as f32 * p.backoff_multiplier.powi(attempt as i32))
                                as u64
                        })
                        .unwrap_or(100);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                } else {
                    return Err(err);
                }
            }
        }
    }

    Err("Max retries exceeded".to_string())
}

/// Execute single step action
async fn execute_step_action(
    step: &WorkflowStep,
    parameters: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    // Substitute parameters in config
    let mut config = step.config.clone();
    for (key, value) in parameters {
        config.insert(key.clone(), value.clone());
    }

    // Simulate action execution (would be actual service calls)
    match step.action.as_str() {
        "generate" => Ok(serde_json::json!({"status": "generated", "config": config})),
        "transform" => Ok(serde_json::json!({"status": "transformed", "config": config})),
        "validate" => Ok(serde_json::json!({"status": "validated", "config": config})),
        "publish" => Ok(serde_json::json!({"status": "published", "config": config})),
        _ => Err(format!("Unknown action: {}", step.action)),
    }
}

/// Topological sort of workflow DAG
fn topological_sort(dag: &WorkflowDAG) -> Vec<String> {
    let mut sorted = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut visiting = std::collections::HashSet::new();

    for step in &dag.steps {
        if !visited.contains(&step.id) {
            visit_step(&step.id, dag, &mut visited, &mut visiting, &mut sorted);
        }
    }

    sorted
}

fn visit_step(
    id: &str,
    dag: &WorkflowDAG,
    visited: &mut std::collections::HashSet<String>,
    visiting: &mut std::collections::HashSet<String>,
    sorted: &mut Vec<String>,
) {
    visiting.insert(id.to_string());

    for edge in &dag.edges {
        if edge.from == id && !visited.contains(&edge.to) {
            visit_step(&edge.to, dag, visited, visiting, sorted);
        }
    }

    visiting.remove(id);
    visited.insert(id.to_string());
    sorted.push(id.to_string());
}

/// Validate execution parameters against workflow parameter definitions
fn validate_parameters(
    definitions: &[WorkflowParameter],
    provided: &HashMap<String, serde_json::Value>,
) -> ApiResult<()> {
    for param_def in definitions {
        if param_def.required && !provided.contains_key(&param_def.name) {
            if param_def.default_value.is_none() {
                return Err(ApiError::InvalidParameter(format!(
                    "Required parameter missing: {}",
                    param_def.name
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort() {
        let dag = WorkflowDAG {
            steps: vec![
                WorkflowStep {
                    id: "step1".to_string(),
                    name: "Step 1".to_string(),
                    action: "generate".to_string(),
                    description: None,
                    step_type: StepType::Sequential,
                    config: HashMap::new(),
                    timeout_secs: None,
                    retry_policy: None,
                    on_failure: None,
                },
                WorkflowStep {
                    id: "step2".to_string(),
                    name: "Step 2".to_string(),
                    action: "validate".to_string(),
                    description: None,
                    step_type: StepType::Sequential,
                    config: HashMap::new(),
                    timeout_secs: None,
                    retry_policy: None,
                    on_failure: None,
                },
            ],
            edges: vec![WorkflowEdge {
                from: "step1".to_string(),
                to: "step2".to_string(),
                condition: None,
            }],
        };

        let sorted = topological_sort(&dag);
        assert_eq!(sorted.len(), 2);
        assert!(sorted.iter().position(|s| s == "step1") < sorted.iter().position(|s| s == "step2"));
    }

    #[test]
    fn test_dag_validation() {
        let dag = WorkflowDAG {
            steps: vec![WorkflowStep {
                id: "step1".to_string(),
                name: "Step 1".to_string(),
                action: "action".to_string(),
                description: None,
                step_type: StepType::Sequential,
                config: HashMap::new(),
                timeout_secs: None,
                retry_policy: None,
                on_failure: None,
            }],
            edges: vec![],
        };

        assert!(dag.validate().is_ok());
    }

    #[test]
    fn test_parameter_validation() {
        let params = vec![WorkflowParameter {
            name: "input".to_string(),
            param_type: "string".to_string(),
            description: None,
            default_value: None,
            required: true,
        }];

        let provided = HashMap::new();
        let result = validate_parameters(&params, &provided);
        assert!(result.is_err());
    }
}
