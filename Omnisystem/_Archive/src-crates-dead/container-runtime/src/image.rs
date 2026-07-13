use crate::{
    ContainerError, ContainerImage, ContainerResult, ImageConfig, ImageFormat, ImageId,
    ImageLayer, ImagePullOptions, ImagePushOptions,
};
use chrono::Utc;
use dashmap::DashMap;
use sha2::{Sha256, Digest};
use std::sync::Arc;

pub struct ImageManager {
    images: Arc<DashMap<String, ContainerImage>>,
}

impl ImageManager {
    pub fn new() -> Self {
        Self {
            images: Arc::new(DashMap::new()),
        }
    }

    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    fn compute_image_digest(image_name: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(image_name);
        let result = hasher.finalize();
        format!("sha256:{}", hex::encode(result))
    }

    pub async fn create_image(
        &self,
        name: String,
        tag: String,
    ) -> ContainerResult<ContainerImage> {
        let image_key = format!("{}:{}", name, tag);

        if self.images.contains_key(&image_key) {
            return Err(ContainerError::ImageAlreadyExists(image_key));
        }

        let digest = Self::compute_image_digest(&image_key);
        let image = ContainerImage {
            id: ImageId(uuid::Uuid::new_v4().to_string()),
            name,
            tag,
            digest,
            format: ImageFormat::Oci,
            size_bytes: 0,
            created_at: Utc::now(),
            layers: vec![ImageLayer {
                digest: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                size_bytes: 0,
                media_type: "application/vnd.oci.image.layer.v1.tar+gzip".to_string(),
            }],
            config: ImageConfig {
                entrypoint: vec![],
                cmd: vec![],
                env: std::collections::HashMap::new(),
                working_dir: "/".to_string(),
                user: "root".to_string(),
                exposed_ports: vec![],
                volumes: vec![],
            },
        };

        self.images.insert(image_key, image.clone());
        Ok(image)
    }

    pub async fn get_image(&self, image_id: &ImageId) -> ContainerResult<ContainerImage> {
        for entry in self.images.iter() {
            if entry.value().id == *image_id {
                return Ok(entry.value().clone());
            }
        }
        Err(ContainerError::ImageNotFound(image_id.0.clone()))
    }

    pub async fn list_images(&self) -> ContainerResult<Vec<ContainerImage>> {
        Ok(self
            .images
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    pub async fn delete_image(&self, image_id: &ImageId) -> ContainerResult<()> {
        let mut found = false;
        self.images.retain(|_, v| {
            if v.id == *image_id {
                found = true;
                false
            } else {
                true
            }
        });

        if found {
            Ok(())
        } else {
            Err(ContainerError::ImageNotFound(image_id.0.clone()))
        }
    }

    pub async fn pull_image(
        &self,
        image_name: &str,
        _options: &ImagePullOptions,
    ) -> ContainerResult<ImageId> {
        let parts: Vec<&str> = image_name.split(':').collect();
        let name = parts[0].to_string();
        let tag = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            "latest".to_string()
        };

        self.create_image(name, tag).await.map(|img| img.id)
    }

    pub async fn push_image(
        &self,
        image_id: &ImageId,
        _options: &ImagePushOptions,
    ) -> ContainerResult<()> {
        if !self
            .images
            .iter()
            .any(|entry| entry.value().id == *image_id)
        {
            return Err(ContainerError::ImageNotFound(image_id.0.clone()));
        }
        Ok(())
    }

    pub async fn tag_image(&self, image_id: &ImageId, new_tag: &str) -> ContainerResult<()> {
        if let Some(entry) = self.images.iter().find(|e| e.value().id == *image_id) {
            let old_key = format!("{}:{}", entry.value().name, entry.value().tag);
            let new_key = format!("{}:{}", entry.value().name, new_tag);

            if let Some((_, mut image)) = self.images.remove(&old_key) {
                image.tag = new_tag.to_string();
                self.images.insert(new_key, image);
                Ok(())
            } else {
                Err(ContainerError::ImageNotFound(image_id.0.clone()))
            }
        } else {
            Err(ContainerError::ImageNotFound(image_id.0.clone()))
        }
    }
}

impl Default for ImageManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_image() {
        let manager = ImageManager::new();
        let image = manager
            .create_image("ubuntu".to_string(), "20.04".to_string())
            .await
            .unwrap();

        assert_eq!(image.name, "ubuntu");
        assert_eq!(image.tag, "20.04");
        assert_eq!(manager.image_count(), 1);
    }

    #[tokio::test]
    async fn test_get_image() {
        let manager = ImageManager::new();
        let created = manager
            .create_image("alpine".to_string(), "latest".to_string())
            .await
            .unwrap();

        let retrieved = manager.get_image(&created.id).await.unwrap();
        assert_eq!(retrieved.id, created.id);
    }

    #[tokio::test]
    async fn test_list_images() {
        let manager = ImageManager::new();

        manager
            .create_image("ubuntu".to_string(), "20.04".to_string())
            .await
            .unwrap();

        manager
            .create_image("alpine".to_string(), "latest".to_string())
            .await
            .unwrap();

        let images = manager.list_images().await.unwrap();
        assert_eq!(images.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_image() {
        let manager = ImageManager::new();
        let image = manager
            .create_image("ubuntu".to_string(), "20.04".to_string())
            .await
            .unwrap();

        assert_eq!(manager.image_count(), 1);
        manager.delete_image(&image.id).await.unwrap();
        assert_eq!(manager.image_count(), 0);
    }

    #[tokio::test]
    async fn test_pull_image() {
        let manager = ImageManager::new();
        let options = ImagePullOptions {
            registry: crate::RegistryUrl("docker.io".to_string()),
            username: None,
            password: None,
            timeout_secs: 300,
        };

        let _ = manager.pull_image("ubuntu:20.04", &options).await.unwrap();
        assert_eq!(manager.image_count(), 1);
    }

    #[tokio::test]
    async fn test_tag_image() {
        let manager = ImageManager::new();
        let image = manager
            .create_image("ubuntu".to_string(), "20.04".to_string())
            .await
            .unwrap();

        manager.tag_image(&image.id, "focal").await.unwrap();

        let images = manager.list_images().await.unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].tag, "focal");
    }

    #[tokio::test]
    async fn test_duplicate_image_error() {
        let manager = ImageManager::new();

        manager
            .create_image("ubuntu".to_string(), "20.04".to_string())
            .await
            .unwrap();

        let result = manager
            .create_image("ubuntu".to_string(), "20.04".to_string())
            .await;

        assert!(result.is_err());
    }
}
