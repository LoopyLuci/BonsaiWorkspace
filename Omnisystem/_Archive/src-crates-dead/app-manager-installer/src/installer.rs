use crate::{Result, InstallerError, InstallationContext, RollbackManager};
use app_manager_core::{AppId, Version, ModuleLifecycleManager, ModuleState};
use app_manager_repository::Repository as PackageRepository;
use std::path::PathBuf;

pub struct Installer {
    repository: PackageRepository,
    lifecycle_manager: ModuleLifecycleManager,
    rollback_manager: RollbackManager,
}

impl Installer {
    pub fn new(
        repository: PackageRepository,
        lifecycle_manager: ModuleLifecycleManager,
    ) -> Self {
        Installer {
            repository,
            lifecycle_manager,
            rollback_manager: RollbackManager::new(),
        }
    }

    pub async fn install(
        &self,
        app_id: &AppId,
        version: &Version,
        installation_path: PathBuf,
    ) -> Result<InstallationContext> {
        tracing::info!("Installing {} v{} to {:?}", app_id, version, installation_path);

        self.lifecycle_manager
            .register_module(app_id.clone())
            .map_err(|_| InstallerError::InstallationFailed("Failed to register module".to_string()))?;

        let mut context = InstallationContext::new(app_id.clone(), version.clone(), installation_path);

        self.lifecycle_manager
            .download(app_id)
            .await
            .map_err(|_| InstallerError::DownloadFailed("Failed to start download".to_string()))?;

        context.mark_phase("download_started");

        let manifest = self.repository
            .fetch_manifest(app_id, version)
            .await
            .map_err(|_| InstallerError::DownloadFailed("Failed to fetch manifest".to_string()))?;

        context.set_manifest(manifest);

        self.lifecycle_manager
            .verify(app_id)
            .await
            .map_err(|_| InstallerError::VerificationFailed("Failed to verify package".to_string()))?;

        context.mark_phase("verification_complete");

        self.lifecycle_manager
            .install(app_id)
            .await
            .map_err(|_| InstallerError::InstallationFailed("Failed to install package".to_string()))?;

        context.mark_phase("installation_complete");
        context.mark_installed();

        tracing::info!("Successfully installed {} v{}", app_id, version);

        Ok(context)
    }

    pub async fn uninstall(&self, app_id: &AppId) -> Result<()> {
        tracing::info!("Uninstalling {}", app_id);

        let state = self.lifecycle_manager
            .get_state(app_id)
            .map_err(|_| InstallerError::Internal("Module not found".to_string()))?;

        if matches!(state, ModuleState::Running) {
            self.lifecycle_manager
                .stop(app_id)
                .await
                .map_err(|_| InstallerError::Internal("Failed to stop module".to_string()))?;
        }

        self.lifecycle_manager
            .unload(app_id)
            .await
            .map_err(|_| InstallerError::Internal("Failed to unload module".to_string()))?;

        tracing::info!("Successfully uninstalled {}", app_id);

        Ok(())
    }

    pub async fn update(
        &self,
        app_id: &AppId,
        new_version: &Version,
        installation_path: PathBuf,
    ) -> Result<()> {
        tracing::info!("Updating {} to v{}", app_id, new_version);

        let current_state = self.lifecycle_manager.get_state(app_id)
            .map_err(|_| InstallerError::Internal("Module not found".to_string()))?;

        if matches!(current_state, ModuleState::Running) {
            self.lifecycle_manager
                .stop(app_id)
                .await
                .map_err(|_| InstallerError::Internal("Failed to stop module".to_string()))?;
        }

        self.rollback_manager.create_snapshot(app_id, &installation_path).await?;

        self.lifecycle_manager
            .download(app_id)
            .await
            .map_err(|_| InstallerError::DownloadFailed("Failed to download update".to_string()))?;

        self.lifecycle_manager
            .verify(app_id)
            .await
            .map_err(|e| {
                let _ = self.lifecycle_manager.mark_failed(app_id);
                InstallerError::VerificationFailed(format!("{:?}", e))
            })?;

        self.lifecycle_manager
            .install(app_id)
            .await
            .map_err(|e| {
                let _ = self.lifecycle_manager.mark_failed(app_id);
                InstallerError::InstallationFailed(format!("{:?}", e))
            })?;

        tracing::info!("Successfully updated {} to v{}", app_id, new_version);

        Ok(())
    }

    pub async fn rollback(&self, app_id: &AppId, installation_path: &PathBuf) -> Result<()> {
        tracing::info!("Rolling back {}", app_id);

        self.rollback_manager.restore_snapshot(app_id, installation_path).await?;

        tracing::info!("Successfully rolled back {}", app_id);

        Ok(())
    }

    pub fn get_lifecycle_manager(&self) -> &ModuleLifecycleManager {
        &self.lifecycle_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_installer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = app_manager_repository::RepositoryConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        let repo = PackageRepository::new(config);
        let lifecycle = ModuleLifecycleManager::new();

        let installer = Installer::new(repo, lifecycle);
        assert!(installer.get_lifecycle_manager().list_all_states().is_empty());
    }
}
