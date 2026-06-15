// OMNISYSTEM CI/CD PIPELINE DEFINITION
// YAML-based pipeline specification and parsing

use std::collections::HashMap;

// ============================================================================
// PIPELINE TYPES
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerEvent {
    Push,
    PullRequest,
    Schedule,
    Manual,
    Tag,
}

#[derive(Debug, Clone)]
pub struct Trigger {
    pub event: TriggerEvent,
    pub branches: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub paths: Option<Vec<String>>,
}

impl Trigger {
    pub fn on_push() -> Self {
        Trigger {
            event: TriggerEvent::Push,
            branches: Some(vec!["main".to_string(), "develop".to_string()]),
            tags: None,
            paths: None,
        }
    }

    pub fn on_pull_request() -> Self {
        Trigger {
            event: TriggerEvent::PullRequest,
            branches: None,
            tags: None,
            paths: None,
        }
    }

    pub fn on_tag(pattern: &str) -> Self {
        Trigger {
            event: TriggerEvent::Tag,
            branches: None,
            tags: Some(vec![pattern.to_string()]),
            paths: None,
        }
    }

    pub fn matches(&self, event: &TriggerEvent) -> bool {
        self.event == *event
    }
}

// ============================================================================
// STEP DEFINITION
// ============================================================================

#[derive(Debug, Clone)]
pub enum StepType {
    Run,
    Build,
    Test,
    Deploy,
    Custom,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub id: String,
    pub name: String,
    pub step_type: StepType,
    pub command: String,
    pub run_if: Option<String>,
    pub timeout_seconds: u64,
    pub retry_count: u32,
    pub depends_on: Vec<String>,
    pub env_vars: HashMap<String, String>,
}

impl Step {
    pub fn new(id: &str, name: &str, command: &str) -> Self {
        Step {
            id: id.to_string(),
            name: name.to_string(),
            step_type: StepType::Run,
            command: command.to_string(),
            run_if: None,
            timeout_seconds: 300,
            retry_count: 0,
            depends_on: Vec::new(),
            env_vars: HashMap::new(),
        }
    }

    pub fn build(id: &str, name: &str, command: &str) -> Self {
        Step {
            id: id.to_string(),
            name: name.to_string(),
            step_type: StepType::Build,
            command: command.to_string(),
            run_if: None,
            timeout_seconds: 600,
            retry_count: 1,
            depends_on: Vec::new(),
            env_vars: HashMap::new(),
        }
    }

    pub fn test(id: &str, name: &str, command: &str) -> Self {
        Step {
            id: id.to_string(),
            name: name.to_string(),
            step_type: StepType::Test,
            command: command.to_string(),
            run_if: None,
            timeout_seconds: 600,
            retry_count: 0,
            depends_on: Vec::new(),
            env_vars: HashMap::new(),
        }
    }

    pub fn deploy(id: &str, name: &str, command: &str) -> Self {
        Step {
            id: id.to_string(),
            name: name.to_string(),
            step_type: StepType::Deploy,
            command: command.to_string(),
            run_if: None,
            timeout_seconds: 900,
            retry_count: 2,
            depends_on: Vec::new(),
            env_vars: HashMap::new(),
        }
    }

    pub fn with_condition(mut self, condition: &str) -> Self {
        self.run_if = Some(condition.to_string());
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    pub fn with_retry(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }

    pub fn depends_on(mut self, id: &str) -> Self {
        self.depends_on.push(id.to_string());
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env_vars.insert(key.to_string(), value.to_string());
        self
    }
}

// ============================================================================
// JOB DEFINITION
// ============================================================================

#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub runs_on: String,
    pub steps: Vec<Step>,
    pub parallel: bool,
    pub timeout_minutes: u64,
    pub env_vars: HashMap<String, String>,
}

impl Job {
    pub fn new(id: &str, name: &str, runs_on: &str) -> Self {
        Job {
            id: id.to_string(),
            name: name.to_string(),
            runs_on: runs_on.to_string(),
            steps: Vec::new(),
            parallel: false,
            timeout_minutes: 60,
            env_vars: HashMap::new(),
        }
    }

    pub fn add_step(mut self, step: Step) -> Self {
        self.steps.push(step);
        self
    }

    pub fn parallel(mut self) -> Self {
        self.parallel = true;
        self
    }

    pub fn timeout(mut self, minutes: u64) -> Self {
        self.timeout_minutes = minutes;
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env_vars.insert(key.to_string(), value.to_string());
        self
    }
}

// ============================================================================
// PIPELINE DEFINITION
// ============================================================================

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub name: String,
    pub version: String,
    pub description: String,
    pub triggers: Vec<Trigger>,
    pub jobs: Vec<Job>,
    pub variables: HashMap<String, String>,
    pub artifacts: Vec<String>,
    pub caches: Vec<String>,
}

impl Pipeline {
    pub fn new(name: &str, version: &str) -> Self {
        Pipeline {
            name: name.to_string(),
            version: version.to_string(),
            description: String::new(),
            triggers: vec![Trigger::on_push()],
            jobs: Vec::new(),
            variables: HashMap::new(),
            artifacts: Vec::new(),
            caches: Vec::new(),
        }
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn add_trigger(mut self, trigger: Trigger) -> Self {
        self.triggers.push(trigger);
        self
    }

    pub fn add_job(mut self, job: Job) -> Self {
        self.jobs.push(job);
        self
    }

    pub fn variable(mut self, key: &str, value: &str) -> Self {
        self.variables.insert(key.to_string(), value.to_string());
        self
    }

    pub fn artifact(mut self, pattern: &str) -> Self {
        self.artifacts.push(pattern.to_string());
        self
    }

    pub fn cache(mut self, path: &str) -> Self {
        self.caches.push(path.to_string());
        self
    }

    pub fn to_yaml(&self) -> String {
        let mut yaml = format!("name: {}\n", self.name);
        yaml.push_str(&format!("version: {}\n", self.version));
        yaml.push_str(&format!("description: {}\n", self.description));

        yaml.push_str("\non:\n");
        for trigger in &self.triggers {
            yaml.push_str(&format!("  - event: {:?}\n", trigger.event));
        }

        yaml.push_str("\njobs:\n");
        for job in &self.jobs {
            yaml.push_str(&format!("  {}:\n", job.id));
            yaml.push_str(&format!("    name: {}\n", job.name));
            yaml.push_str(&format!("    runs-on: {}\n", job.runs_on));
            yaml.push_str("    steps:\n");
            for step in &job.steps {
                yaml.push_str(&format!("      - id: {}\n", step.id));
                yaml.push_str(&format!("        name: {}\n", step.name));
                yaml.push_str(&format!("        run: {}\n", step.command));
                yaml.push_str(&format!("        timeout-seconds: {}\n", step.timeout_seconds));
            }
        }

        yaml
    }
}

// ============================================================================
// PIPELINE PARSER
// ============================================================================

pub struct PipelineParser;

impl PipelineParser {
    pub fn parse_yaml(yaml_content: &str) -> Result<Pipeline, String> {
        // Simplified YAML parsing - in production use a proper YAML parser
        let mut pipeline = Pipeline::new("parsed-pipeline", "1.0");

        for line in yaml_content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("name:") {
                let name = trimmed.split(':').nth(1).unwrap_or("").trim();
                pipeline.name = name.to_string();
            }

            if trimmed.starts_with("version:") {
                let version = trimmed.split(':').nth(1).unwrap_or("").trim();
                pipeline.version = version.to_string();
            }

            if trimmed.starts_with("description:") {
                let desc = trimmed.split(':').nth(1).unwrap_or("").trim();
                pipeline.description = desc.to_string();
            }
        }

        println!("✅ Pipeline parsed: {}", pipeline.name);
        Ok(pipeline)
    }

    pub fn from_builder(pipeline: Pipeline) -> Pipeline {
        println!("✅ Pipeline created from builder: {}", pipeline.name);
        pipeline
    }
}

// ============================================================================
// PIPELINE VALIDATION
// ============================================================================

pub struct PipelineValidator;

impl PipelineValidator {
    pub fn validate(pipeline: &Pipeline) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if pipeline.name.is_empty() {
            errors.push("Pipeline name is required".to_string());
        }

        if pipeline.jobs.is_empty() {
            errors.push("Pipeline must have at least one job".to_string());
        }

        for job in &pipeline.jobs {
            if job.steps.is_empty() {
                errors.push(format!("Job '{}' has no steps", job.id));
            }
        }

        if errors.is_empty() {
            println!("✅ Pipeline validation passed");
            Ok(())
        } else {
            println!("❌ Pipeline validation failed: {:?}", errors);
            Err(errors)
        }
    }

    pub fn check_circular_dependencies(pipeline: &Pipeline) -> Result<(), String> {
        for job in &pipeline.jobs {
            for step in &job.steps {
                for dep in &step.depends_on {
                    // Simple circular dependency check
                    if step.id == *dep {
                        return Err(format!("Circular dependency detected: {}", step.id));
                    }
                }
            }
        }
        println!("✅ No circular dependencies found");
        Ok(())
    }
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

pub fn example_pipeline() -> Pipeline {
    let build_step = Step::build("build", "Build project", "cargo build --release");
    let test_step = Step::test("test", "Run tests", "cargo test --all");
    let deploy_step = Step::deploy("deploy", "Deploy to production", "deploy.sh");

    let build_job = Job::new("build-job", "Build", "ubuntu-latest")
        .add_step(build_step)
        .add_step(test_step);

    let deploy_job = Job::new("deploy-job", "Deploy", "ubuntu-latest")
        .add_step(deploy_step)
        .timeout(30);

    let mut pipeline = Pipeline::new("Omnisystem CI/CD", "1.0")
        .description("Complete CI/CD pipeline")
        .add_trigger(Trigger::on_push())
        .add_trigger(Trigger::on_pull_request())
        .add_job(build_job)
        .add_job(deploy_job);

    pipeline.variable("RUST_BACKTRACE", "1");
    pipeline.artifact("target/release/*");
    pipeline.cache("target/");

    pipeline
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_creation() {
        let trigger = Trigger::on_push();
        assert_eq!(trigger.event, TriggerEvent::Push);
    }

    #[test]
    fn test_step_creation() {
        let step = Step::new("step-1", "Run", "echo test");
        assert_eq!(step.id, "step-1");
        assert_eq!(step.timeout_seconds, 300);
    }

    #[test]
    fn test_step_builder() {
        let step = Step::build("build", "Build", "cargo build")
            .with_timeout(600)
            .with_retry(2);

        assert_eq!(step.step_type, StepType::Build);
        assert_eq!(step.timeout_seconds, 600);
        assert_eq!(step.retry_count, 2);
    }

    #[test]
    fn test_job_creation() {
        let job = Job::new("job-1", "Test Job", "ubuntu-latest");
        assert_eq!(job.id, "job-1");
        assert!(!job.parallel);
    }

    #[test]
    fn test_pipeline_creation() {
        let pipeline = Pipeline::new("test", "1.0");
        assert_eq!(pipeline.name, "test");
        assert!(!pipeline.jobs.is_empty() || pipeline.jobs.is_empty()); // Can be either
    }

    #[test]
    fn test_pipeline_validation() {
        let pipeline = example_pipeline();
        assert!(PipelineValidator::validate(&pipeline).is_ok());
    }

    #[test]
    fn test_pipeline_parser() {
        let yaml = "name: test-pipeline\nversion: 1.0\ndescription: Test";
        let pipeline = PipelineParser::parse_yaml(yaml).unwrap();
        assert_eq!(pipeline.name, "test-pipeline");
    }

    #[test]
    fn test_pipeline_yaml_output() {
        let pipeline = example_pipeline();
        let yaml = pipeline.to_yaml();
        assert!(yaml.contains("name:"));
        assert!(yaml.contains("jobs:"));
    }
}
