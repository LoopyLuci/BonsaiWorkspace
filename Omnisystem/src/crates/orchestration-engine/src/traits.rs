use async_trait::async_trait;
use crate::{
    Deployment, DeploymentId, HealthProbe, Node, NodeId, OrchestrationResult, Pod, PodId, PodSpec,
    ScalingPolicy, Service, ServiceId,
};

#[async_trait]
pub trait PodOperations: Send + Sync {
    async fn create_pod(&self, spec: &PodSpec) -> OrchestrationResult<PodId>;

    async fn get_pod(&self, pod_id: &PodId) -> OrchestrationResult<Pod>;

    async fn list_pods(&self) -> OrchestrationResult<Vec<Pod>>;

    async fn delete_pod(&self, pod_id: &PodId) -> OrchestrationResult<()>;

    async fn list_deployment_pods(&self, deployment_id: &DeploymentId) -> OrchestrationResult<Vec<Pod>>;
}

#[async_trait]
pub trait DeploymentOperations: Send + Sync {
    async fn create_deployment(&self, spec: &PodSpec) -> OrchestrationResult<DeploymentId>;

    async fn get_deployment(&self, deployment_id: &DeploymentId) -> OrchestrationResult<Deployment>;

    async fn list_deployments(&self) -> OrchestrationResult<Vec<Deployment>>;

    async fn update_deployment(
        &self,
        deployment_id: &DeploymentId,
        spec: &PodSpec,
    ) -> OrchestrationResult<()>;

    async fn delete_deployment(&self, deployment_id: &DeploymentId) -> OrchestrationResult<()>;

    async fn scale_deployment(
        &self,
        deployment_id: &DeploymentId,
        replicas: u32,
    ) -> OrchestrationResult<()>;

    async fn rollout_deployment(&self, deployment_id: &DeploymentId) -> OrchestrationResult<()>;

    async fn rollback_deployment(&self, deployment_id: &DeploymentId) -> OrchestrationResult<()>;
}

#[async_trait]
pub trait ServiceOperations: Send + Sync {
    async fn create_service(&self, name: &str, selector: &std::collections::HashMap<String, String>) -> OrchestrationResult<ServiceId>;

    async fn get_service(&self, service_id: &ServiceId) -> OrchestrationResult<Service>;

    async fn list_services(&self) -> OrchestrationResult<Vec<Service>>;

    async fn delete_service(&self, service_id: &ServiceId) -> OrchestrationResult<()>;
}

#[async_trait]
pub trait NodeOperations: Send + Sync {
    async fn register_node(&self, name: &str, cpu_millicores: u64, memory_bytes: u64) -> OrchestrationResult<NodeId>;

    async fn get_node(&self, node_id: &NodeId) -> OrchestrationResult<Node>;

    async fn list_nodes(&self) -> OrchestrationResult<Vec<Node>>;

    async fn get_node_by_name(&self, name: &str) -> OrchestrationResult<Node>;

    async fn drain_node(&self, node_id: &NodeId) -> OrchestrationResult<()>;

    async fn uncordon_node(&self, node_id: &NodeId) -> OrchestrationResult<()>;
}

#[async_trait]
pub trait HealthCheckOperations: Send + Sync {
    async fn add_health_probe(&self, pod_id: &PodId, probe: &HealthProbe) -> OrchestrationResult<()>;

    async fn check_pod_health(&self, pod_id: &PodId) -> OrchestrationResult<bool>;

    async fn get_pod_readiness(&self, pod_id: &PodId) -> OrchestrationResult<bool>;
}

#[async_trait]
pub trait ScalingOperations: Send + Sync {
    async fn set_scaling_policy(&self, deployment_id: &DeploymentId, policy: &ScalingPolicy) -> OrchestrationResult<()>;

    async fn get_scaling_policy(&self, deployment_id: &DeploymentId) -> OrchestrationResult<ScalingPolicy>;

    async fn evaluate_scaling(&self, deployment_id: &DeploymentId) -> OrchestrationResult<Option<u32>>;

    async fn apply_scaling(&self, deployment_id: &DeploymentId, replicas: u32) -> OrchestrationResult<()>;
}
