use crate::{CoordinationError, CoordinationResult, ConflictResolution};
use dashmap::DashMap;
use std::sync::Arc;

pub struct ConflictResolver {
    resolutions: Arc<DashMap<String, ConflictResolution>>,
}

impl ConflictResolver {
    pub fn new() -> Self {
        Self {
            resolutions: Arc::new(DashMap::new()),
        }
    }

    pub async fn detect_conflict(
        &self,
        conflict_id: &str,
        service_id: &str,
        version_local: u32,
        version_remote: u32,
    ) -> CoordinationResult<bool> {
        Ok(version_local != version_remote)
    }

    pub async fn resolve_conflict_last_write_wins(
        &self,
        conflict_id: &str,
        service_id: &str,
        version_local: u32,
        version_remote: u32,
    ) -> CoordinationResult<ConflictResolution> {
        let resolution = if version_remote > version_local {
            "accept_remote"
        } else {
            "keep_local"
        };

        let conflict_resolution = ConflictResolution {
            conflict_id: conflict_id.to_string(),
            service_id: service_id.to_string(),
            version_local,
            version_remote,
            resolution: resolution.to_string(),
        };

        self.resolutions.insert(conflict_id.to_string(), conflict_resolution.clone());
        Ok(conflict_resolution)
    }

    pub async fn resolve_conflict_custom(
        &self,
        conflict_id: &str,
        service_id: &str,
        version_local: u32,
        version_remote: u32,
        resolution: &str,
    ) -> CoordinationResult<ConflictResolution> {
        let conflict_resolution = ConflictResolution {
            conflict_id: conflict_id.to_string(),
            service_id: service_id.to_string(),
            version_local,
            version_remote,
            resolution: resolution.to_string(),
        };

        self.resolutions.insert(conflict_id.to_string(), conflict_resolution.clone());
        Ok(conflict_resolution)
    }

    pub async fn get_resolution(&self, conflict_id: &str) -> CoordinationResult<ConflictResolution> {
        self.resolutions
            .get(conflict_id)
            .map(|entry| entry.clone())
            .ok_or(CoordinationError::ConflictDetected)
    }

    pub fn resolution_count(&self) -> usize {
        self.resolutions.len()
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_conflict() {
        let resolver = ConflictResolver::new();
        let has_conflict = resolver.detect_conflict("conflict-1", "service-1", 1, 2).await.unwrap();

        assert!(has_conflict);
    }

    #[tokio::test]
    async fn test_no_conflict() {
        let resolver = ConflictResolver::new();
        let has_conflict = resolver.detect_conflict("conflict-1", "service-1", 1, 1).await.unwrap();

        assert!(!has_conflict);
    }

    #[tokio::test]
    async fn test_resolve_last_write_wins_remote() {
        let resolver = ConflictResolver::new();
        let resolution = resolver
            .resolve_conflict_last_write_wins("conflict-1", "service-1", 1, 2)
            .await
            .unwrap();

        assert_eq!(resolution.resolution, "accept_remote");
    }

    #[tokio::test]
    async fn test_resolve_last_write_wins_local() {
        let resolver = ConflictResolver::new();
        let resolution = resolver
            .resolve_conflict_last_write_wins("conflict-1", "service-1", 5, 2)
            .await
            .unwrap();

        assert_eq!(resolution.resolution, "keep_local");
    }

    #[tokio::test]
    async fn test_resolve_conflict_custom() {
        let resolver = ConflictResolver::new();
        let resolution = resolver
            .resolve_conflict_custom("conflict-1", "service-1", 1, 2, "custom_merge")
            .await
            .unwrap();

        assert_eq!(resolution.resolution, "custom_merge");
    }

    #[tokio::test]
    async fn test_get_resolution() {
        let resolver = ConflictResolver::new();
        resolver
            .resolve_conflict_custom("conflict-1", "service-1", 1, 2, "custom_merge")
            .await
            .unwrap();

        let resolution = resolver.get_resolution("conflict-1").await.unwrap();
        assert_eq!(resolution.conflict_id, "conflict-1");
    }

    #[tokio::test]
    async fn test_resolution_not_found() {
        let resolver = ConflictResolver::new();
        let result = resolver.get_resolution("nonexistent").await;

        assert!(result.is_err());
    }
}
