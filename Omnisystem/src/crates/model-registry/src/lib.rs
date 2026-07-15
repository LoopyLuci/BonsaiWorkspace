//! model-registry: in-memory ML model registry (models, versions, training
//! jobs, deployments) plus real readers for Bluebonnet inference manifests
//! and Crystal image (compressed model artifact) metadata.

mod crystal;
mod error;
mod manifest;
mod registry;
mod types;

pub use crystal::{ComponentInfo, CompressionAlgorithm, CrystalImage, CrystalMetadata};
pub use error::{RegistryError, RegistryResult};
pub use manifest::{BluebonnetManifest, InferenceParameters, SecurityConfig, ToolConfig, ToolExecutionMode};
pub use registry::ModelRegistry;
pub use types::{
    DeploymentStatus, JobStatus, ModelDeployment, ModelMetadata, ModelStage, ModelVersion,
    RegisteredModel, TrainingJob,
};
