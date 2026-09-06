//! CLI for omnisystem-cicd — exercises the crate's real (simulated)
//! build/test/deploy pipeline, instead of the dead generic Component
//! template.

use omnisystem_cicd::{Builder, CIPipeline, Deployer, PipelineStatus, Tester};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pipeline = CIPipeline::new();
    let run_id = pipeline.start_run("main".to_string(), "HEAD".to_string()).await;
    println!("started pipeline run {run_id}");

    let mut builder = Builder::new();
    let crates_built = builder.build_workspace().await?;
    println!("built {crates_built} crates");

    let mut tester = Tester::new();
    let test_result = tester.run_tests().await?;
    println!(
        "tests: {}/{} passed ({:.1}% coverage)",
        test_result.passed, test_result.total_tests, test_result.coverage_percent
    );

    let status = if test_result.failed == 0 { PipelineStatus::Passed } else { PipelineStatus::Failed };
    pipeline.update_run_status(run_id, status).await;

    if status == PipelineStatus::Passed {
        let deployer = Deployer;
        let deployment = deployer.deploy("production", "1.0.0").await?;
        println!("deployed version {} at {}", deployment.version, deployment.timestamp);
    }

    let run = pipeline.get_run(run_id).await.expect("run was just created");
    println!("final run status: {:?}", run.status);

    Ok(())
}
