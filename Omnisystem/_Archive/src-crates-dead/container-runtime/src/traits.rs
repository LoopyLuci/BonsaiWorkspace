use async_trait::async_trait;
use crate::{
    Container, ContainerConfig, ContainerId, ContainerImage,
    ContainerResult, ContainerStats, ImageId, ImagePullOptions, ImagePushOptions,
    RegistryConfig, RegistryUrl,
};

#[async_trait]
pub trait ImageOperations: Send + Sync {
    async fn pull_image(&self, image: &str, options: &ImagePullOptions) -> ContainerResult<ImageId>;

    async fn push_image(&self, image_id: &ImageId, options: &ImagePushOptions) -> ContainerResult<()>;

    async fn build_image(&self, dockerfile: &str, tag: &str) -> ContainerResult<ImageId>;

    async fn list_images(&self) -> ContainerResult<Vec<ContainerImage>>;

    async fn get_image(&self, image_id: &ImageId) -> ContainerResult<ContainerImage>;

    async fn delete_image(&self, image_id: &ImageId) -> ContainerResult<()>;

    async fn tag_image(&self, image_id: &ImageId, new_tag: &str) -> ContainerResult<()>;

    async fn inspect_image(&self, image_id: &ImageId) -> ContainerResult<ContainerImage>;
}

#[async_trait]
pub trait ContainerOperations: Send + Sync {
    async fn create_container(
        &self,
        image_id: &ImageId,
        config: &ContainerConfig,
    ) -> ContainerResult<ContainerId>;

    async fn start_container(&self, container_id: &ContainerId) -> ContainerResult<()>;

    async fn stop_container(&self, container_id: &ContainerId, timeout_secs: u64) -> ContainerResult<()>;

    async fn pause_container(&self, container_id: &ContainerId) -> ContainerResult<()>;

    async fn resume_container(&self, container_id: &ContainerId) -> ContainerResult<()>;

    async fn kill_container(&self, container_id: &ContainerId, signal: u32) -> ContainerResult<()>;

    async fn remove_container(&self, container_id: &ContainerId) -> ContainerResult<()>;

    async fn get_container(&self, container_id: &ContainerId) -> ContainerResult<Container>;

    async fn list_containers(&self) -> ContainerResult<Vec<Container>>;

    async fn get_container_stats(&self, container_id: &ContainerId) -> ContainerResult<ContainerStats>;
}

#[async_trait]
pub trait RegistryOperations: Send + Sync {
    async fn register_registry(&self, config: &RegistryConfig) -> ContainerResult<()>;

    async fn get_registry(&self, url: &RegistryUrl) -> ContainerResult<RegistryConfig>;

    async fn list_registries(&self) -> ContainerResult<Vec<RegistryConfig>>;

    async fn remove_registry(&self, url: &RegistryUrl) -> ContainerResult<()>;

    async fn test_registry_connection(&self, url: &RegistryUrl) -> ContainerResult<bool>;
}
