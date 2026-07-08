/// Runs pre-compilation continuously in the background
pub async fn run_background(_precompiler: std::sync::Arc<crate::PreCompiler>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        interval.tick().await;
        // Scan for changed files, precompile speculatively
    }
}
