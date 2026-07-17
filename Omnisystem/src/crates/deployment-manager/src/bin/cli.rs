//! CLI demo: deploy a version and check its status.

use deployment_manager::DeploymentManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = DeploymentManager::new();

    let deployment = manager.deploy("1.0.0").await?;
    println!("Deployed {}: {}", deployment.deployment_id, deployment.version);

    let status = manager.get_status(&deployment.deployment_id).await?;
    println!("Status: {}", status);
    println!("Total deployments: {}", manager.deployment_count());

    Ok(())
}
