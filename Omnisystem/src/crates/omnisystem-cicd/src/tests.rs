#[cfg(test)]
mod integration_tests {
    use crate::*;

    #[tokio::test]
    async fn test_full_ci_pipeline() {
        // Create pipeline
        let pipeline = pipeline::CIPipeline::new();
        let run_id = pipeline.start_run("main".to_string(), "abc123".to_string()).await;

        // Build
        let mut builder = builder::Builder::new();
        let _ = builder.build_workspace().await;

        // Test
        let mut tester = tester::Tester::new();
        let test_result = tester.run_tests().await.unwrap();
        assert_eq!(test_result.passed, 1674);

        // Update pipeline
        pipeline.update_run_status(run_id, pipeline::PipelineStatus::Passed).await;

        // Verify
        let run = pipeline.get_run(run_id).await.unwrap();
        assert_eq!(run.status, pipeline::PipelineStatus::Passed);
    }
}
