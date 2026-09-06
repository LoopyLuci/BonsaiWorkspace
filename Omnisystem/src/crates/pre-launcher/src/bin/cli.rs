//! CLI for pre-launcher — exercises the crate's real environment detection
//! and dependency-checking logic, instead of the dead generic Component
//! template. Read-only: it reports status but does not install anything.

use pre_launcher::{DependencyManager, EnvironmentInfo};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_info = EnvironmentInfo::detect()?;
    env_info.print_summary();

    let mut deps = DependencyManager::new();
    let check = deps.check_all().await?;
    println!("{}", deps.get_summary());
    println!(
        "all dependencies satisfied: {} (missing: {}, out of date: {})",
        check.all_satisfied,
        check.missing.len(),
        check.out_of_date.len()
    );

    Ok(())
}
