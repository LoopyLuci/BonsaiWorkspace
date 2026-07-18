//! model-registry CLI.
//!
//! Demonstrates the registry end-to-end: registers a model, creates a
//! version, promotes it to Production, starts+completes a training job,
//! and (if a manifest path is given as the first argument) parses and
//! validates a Bluebonnet inference manifest.

use chrono::Utc;
use model_registry::{
    BluebonnetManifest, JobStatus, ModelRegistry, ModelStage, ModelVersion, RegisteredModel,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ModelRegistry::new();

    let model_id = Uuid::new_v4();
    let model = RegisteredModel {
        model_id,
        name: "bert-classifier".to_string(),
        current_version: "1.0.0".to_string(),
        owner: "ml-team".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    registry.register_model(&model).await?;
    println!("Registered model {} ({})", model.name, model.model_id);

    let version = ModelVersion {
        version_id: Uuid::new_v4(),
        model_id,
        version: "1.0.0".to_string(),
        stage: ModelStage::Development,
        metrics: vec![("accuracy".to_string(), 0.94)],
        created_at: Utc::now(),
    };
    registry.create_version(&version).await?;
    println!("Created version {} (stage: {:?})", version.version, version.stage);

    registry
        .promote_version(version.version_id, ModelStage::Production)
        .await?;
    let promoted = registry.get_version(version.version_id).await?;
    println!("Promoted version {} to {:?}", promoted.version, promoted.stage);

    let job_id = registry.create_training_job(model_id).await?;
    registry
        .update_job_status(job_id, JobStatus::Completed)
        .await?;
    let job = registry.get_training_job(job_id).await?;
    println!("Training job {} finished with status {:?}", job.job_id, job.status);

    println!(
        "Registry now holds {} model(s), {} version(s)",
        registry.model_count(),
        registry.version_count()
    );

    if let Some(manifest_path) = std::env::args().nth(1) {
        let text = std::fs::read_to_string(&manifest_path)?;
        let manifest = BluebonnetManifest::parse(&text)?;
        println!(
            "Parsed manifest: model={} version={} quantization={}",
            manifest.model, manifest.version, manifest.quantization
        );
    } else {
        let default_manifest = BluebonnetManifest::default();
        println!(
            "No manifest path given; default manifest would use model={} quantization={}",
            default_manifest.model, default_manifest.quantization
        );
    }

    Ok(())
}
