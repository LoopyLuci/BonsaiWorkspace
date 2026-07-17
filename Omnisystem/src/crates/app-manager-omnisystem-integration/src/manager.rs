//! `ApplicationManager` -- the orchestration facade over the app-manager
//! ecosystem.
//!
//! This is the single entry point consumers (the `app-manager-cli` binary,
//! or any other Omnisystem component) use to install, run, and inspect
//! applications. It does not reimplement any lifecycle, download, or
//! verification logic itself -- it wires together the already-real
//! `app-manager-repository::Repository`, `app-manager-installer::Installer`,
//! and `app-manager-core::module_lifecycle::ModuleLifecycleManager` and adds
//! the coordination policy on top (e.g. "starting an app means loading it
//! first if it's merely installed", "you can't start something that's
//! already running").

use crate::error::{AppIntegrationError, Result};
use app_manager_core::module_lifecycle::ModuleLifecycleManager;
use app_manager_core::types::{AppId, ModuleState, Version};
use app_manager_installer::Installer;
use app_manager_repository::{Repository, RepositoryConfig};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Aggregate, computed-on-demand health snapshot of the applications known
/// to this `ApplicationManager`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthStatus {
    /// `true` unless at least one registered module is `Failed` or `Corrupted`.
    /// An empty registry (nothing installed yet) is considered operational.
    pub operational: bool,
    /// Number of modules currently in the `Loaded` or `Running` state.
    pub modules_loaded: usize,
    /// Whether `initialize()` has completed successfully on this manager.
    pub initialized: bool,
}

/// Orchestrates the app-manager-core / app-manager-repository /
/// app-manager-installer ecosystem behind a single facade.
pub struct ApplicationManager {
    installer: Installer,
    apps_dir: PathBuf,
    cache_dir: PathBuf,
    initialized: AtomicBool,
}

impl ApplicationManager {
    /// Build a manager with default configuration: the marketplace repository
    /// config default and an apps directory taken from `OMNISYSTEM_APPS_DIR`
    /// (falling back to `./.omnisystem/apps`).
    pub fn new() -> Self {
        Self::with_config(RepositoryConfig::default())
    }

    /// Build a manager with a caller-supplied repository configuration
    /// (marketplace URL, cache dir, GitHub token, signature verification),
    /// still using the default apps directory resolution.
    pub fn with_config(config: RepositoryConfig) -> Self {
        Self::with_apps_dir_and_config(default_apps_dir(), config)
    }

    /// Build a manager with both the installed-apps base directory and the
    /// repository configuration supplied explicitly. This is the
    /// constructor tests use to point the manager at a temp directory and a
    /// local mock marketplace.
    pub fn with_apps_dir_and_config(apps_dir: PathBuf, config: RepositoryConfig) -> Self {
        let cache_dir = config.cache_dir.clone();
        let repository = Repository::new(config);
        let lifecycle_manager = ModuleLifecycleManager::new();
        let installer = Installer::new(repository, lifecycle_manager);

        ApplicationManager {
            installer,
            apps_dir,
            cache_dir,
            initialized: AtomicBool::new(false),
        }
    }

    /// Perform real, idempotent, observable startup work: ensure the apps
    /// directory and the repository cache directory exist on disk. Only
    /// flips `initialized` once that filesystem setup has actually
    /// succeeded.
    pub async fn initialize(&mut self) -> Result<()> {
        tokio::fs::create_dir_all(&self.apps_dir).await?;
        tokio::fs::create_dir_all(&self.cache_dir).await?;

        self.initialized.store(true, Ordering::SeqCst);

        tracing::info!(
            "ApplicationManager initialized (apps_dir={:?}, cache_dir={:?})",
            self.apps_dir,
            self.cache_dir
        );

        Ok(())
    }

    /// Whether `initialize()` has completed successfully.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// Base directory installed applications live under.
    pub fn apps_dir(&self) -> &Path {
        &self.apps_dir
    }

    fn installation_path(&self, app_id: &AppId) -> PathBuf {
        self.apps_dir.join(app_id.as_str())
    }

    /// Install `app_id` at `version` into this manager's apps directory.
    /// Delegates the entire register -> download -> fetch-manifest -> verify
    /// -> install pipeline to `Installer::install`.
    pub async fn load_application(&self, app_id: &AppId, version: &Version) -> Result<()> {
        let installation_path = self.installation_path(app_id);
        self.installer
            .install(app_id, version, installation_path)
            .await?;
        Ok(())
    }

    /// Uninstall `app_id` (stopping it first if it is running, then
    /// unloading it). Delegates to `Installer::uninstall`.
    pub async fn unload_application(&self, app_id: &AppId) -> Result<()> {
        self.installer.uninstall(app_id).await?;
        Ok(())
    }

    /// Update `app_id` to `version` (stop-if-running, snapshot, download,
    /// verify, install). Delegates to `Installer::update`.
    pub async fn update_application(&self, app_id: &AppId, version: &Version) -> Result<()> {
        let installation_path = self.installation_path(app_id);
        self.installer
            .update(app_id, version, installation_path)
            .await?;
        Ok(())
    }

    /// Roll `app_id` back to its last snapshot. Delegates to
    /// `Installer::rollback`.
    pub async fn rollback_application(&self, app_id: &AppId) -> Result<()> {
        let installation_path = self.installation_path(app_id);
        self.installer.rollback(app_id, &installation_path).await?;
        Ok(())
    }

    /// State-aware start: if the app is merely `Installed`, load it first
    /// and then start it; if it's already `Loaded`, start it directly; if
    /// it's already `Running`, this is an error (not a silent no-op); any
    /// other state is an invalid-transition error surfaced from the real
    /// lifecycle state machine.
    pub async fn start_application(&self, app_id: &AppId) -> Result<()> {
        let lifecycle = self.installer.get_lifecycle_manager();
        let state = lifecycle.get_state(app_id)?;

        match state {
            ModuleState::Installed => {
                lifecycle.load(app_id).await?;
                lifecycle.start(app_id).await?;
            }
            ModuleState::Loaded => {
                lifecycle.start(app_id).await?;
            }
            ModuleState::Running => {
                return Err(AppIntegrationError::AlreadyRunning(app_id.to_string()));
            }
            other => {
                return Err(AppIntegrationError::Lifecycle(
                    app_manager_core::error::AppManagerError::InvalidStateTransition(format!(
                        "cannot start application {} from state {:?}",
                        app_id, other
                    )),
                ));
            }
        }

        Ok(())
    }

    /// Stop a running application. Delegates directly to the lifecycle
    /// manager's real `Running -> Stopped` transition.
    pub async fn stop_application(&self, app_id: &AppId) -> Result<()> {
        self.installer.get_lifecycle_manager().stop(app_id).await?;
        Ok(())
    }

    /// Look up the real, current lifecycle state of a single application.
    pub fn get_application_state(&self, app_id: &AppId) -> Result<ModuleState> {
        Ok(self.installer.get_lifecycle_manager().get_state(app_id)?)
    }

    /// List every application this manager currently has state for, along
    /// with its real lifecycle state.
    pub fn list_applications(&self) -> Vec<(AppId, ModuleState)> {
        self.installer.get_lifecycle_manager().list_all_states()
    }

    /// Compute a real, current health snapshot from the lifecycle manager's
    /// state table -- no hardcoded values.
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let states = self.installer.get_lifecycle_manager().list_all_states();

        let modules_loaded = states
            .iter()
            .filter(|(_, state)| matches!(state, ModuleState::Loaded | ModuleState::Running))
            .count();

        let operational = !states
            .iter()
            .any(|(_, state)| matches!(state, ModuleState::Failed | ModuleState::Corrupted));

        Ok(HealthStatus {
            operational,
            modules_loaded,
            initialized: self.is_initialized(),
        })
    }
}

impl Default for ApplicationManager {
    fn default() -> Self {
        Self::new()
    }
}

fn default_apps_dir() -> PathBuf {
    std::env::var_os("OMNISYSTEM_APPS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./.omnisystem/apps"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_manager_core::types::Manifest;
    use std::collections::{HashMap, HashSet};

    fn test_manifest(app_id: &AppId, version: &Version) -> Manifest {
        Manifest {
            app_id: app_id.clone(),
            name: "Test App".to_string(),
            version: version.clone(),
            description: "A manifest served by the in-test mock marketplace".to_string(),
            author: "test-suite".to_string(),
            license: "Apache-2.0".to_string(),
            dependencies: Vec::new(),
            modules: HashMap::new(),
            entry_points: HashMap::new(),
            permissions: HashSet::new(),
            environment: HashMap::new(),
        }
    }

    /// Spin up a real local HTTP server implementing the tiny slice of the
    /// marketplace API that `Repository::fetch_manifest` calls
    /// (`GET /api/manifests/:app_id/:version`), so that `Installer::install`
    /// can run its real network fetch against something real instead of a
    /// hand-rolled fake. Returns the base URL to plug into
    /// `RepositoryConfig::marketplace_url`.
    async fn spawn_mock_marketplace(manifest: Manifest) -> String {
        use axum::extract::Path as AxumPath;
        use axum::routing::get;
        use axum::{Json, Router};

        let route_manifest = manifest.clone();
        let app = Router::new().route(
            "/api/manifests/:app_id/:version",
            get(move |AxumPath((_app_id, _version)): AxumPath<(String, String)>| {
                let manifest = route_manifest.clone();
                async move { Json(manifest) }
            }),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock marketplace listener");
        let addr = listener.local_addr().expect("mock marketplace local addr");

        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("mock marketplace server");
        });

        format!("http://{}", addr)
    }

    fn repo_config(marketplace_url: String, cache_dir: PathBuf) -> RepositoryConfig {
        RepositoryConfig {
            marketplace_url,
            cache_dir,
            github_token: None,
            verify_signatures: true,
        }
    }

    #[tokio::test]
    async fn test_new_is_not_initialized() {
        let mgr = ApplicationManager::new();
        assert!(!mgr.is_initialized());
    }

    #[tokio::test]
    async fn test_initialize_creates_directories() {
        let temp = tempfile::tempdir().unwrap();
        let apps_dir = temp.path().join("apps");
        let cache_dir = temp.path().join("cache");
        assert!(!apps_dir.exists());
        assert!(!cache_dir.exists());

        let config = repo_config("http://127.0.0.1:1".to_string(), cache_dir.clone());
        let mut mgr = ApplicationManager::with_apps_dir_and_config(apps_dir.clone(), config);

        mgr.initialize().await.unwrap();

        assert!(apps_dir.is_dir());
        assert!(cache_dir.is_dir());
        assert!(mgr.is_initialized());
    }

    #[tokio::test]
    async fn test_initialize_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let apps_dir = temp.path().join("apps");
        let cache_dir = temp.path().join("cache");
        let config = repo_config("http://127.0.0.1:1".to_string(), cache_dir);
        let mut mgr = ApplicationManager::with_apps_dir_and_config(apps_dir, config);

        mgr.initialize().await.unwrap();
        mgr.initialize().await.unwrap();
        assert!(mgr.is_initialized());
    }

    #[tokio::test]
    async fn test_health_check_empty_registry_is_operational() {
        let temp = tempfile::tempdir().unwrap();
        let config = repo_config("http://127.0.0.1:1".to_string(), temp.path().join("cache"));
        let mgr = ApplicationManager::with_apps_dir_and_config(temp.path().join("apps"), config);

        let health = mgr.health_check().await.unwrap();
        assert!(health.operational);
        assert_eq!(health.modules_loaded, 0);
        assert!(!health.initialized);
    }

    #[tokio::test]
    async fn test_full_lifecycle_through_real_install_and_start() {
        let temp = tempfile::tempdir().unwrap();
        let app_id = AppId::new("demo-app").unwrap();
        let version = Version::new(1, 0, 0);
        let manifest = test_manifest(&app_id, &version);

        let marketplace_url = spawn_mock_marketplace(manifest).await;
        let config = repo_config(marketplace_url, temp.path().join("cache"));
        let mut mgr = ApplicationManager::with_apps_dir_and_config(temp.path().join("apps"), config);

        mgr.initialize().await.unwrap();

        // Real install: register_module -> download -> fetch_manifest (real
        // HTTP call to the mock marketplace above) -> verify -> install.
        mgr.load_application(&app_id, &version).await.unwrap();
        assert_eq!(
            mgr.get_application_state(&app_id).unwrap(),
            ModuleState::Installed
        );

        let health = mgr.health_check().await.unwrap();
        assert!(health.operational);
        assert!(health.initialized);
        // Installed but not yet loaded/running.
        assert_eq!(health.modules_loaded, 0);

        // Real state-aware start: Installed -> Loaded -> Running.
        mgr.start_application(&app_id).await.unwrap();
        assert_eq!(
            mgr.get_application_state(&app_id).unwrap(),
            ModuleState::Running
        );

        let health = mgr.health_check().await.unwrap();
        assert_eq!(health.modules_loaded, 1);

        // Starting an already-running application is a real error, not a
        // silent no-op.
        let err = mgr.start_application(&app_id).await.unwrap_err();
        assert!(matches!(err, AppIntegrationError::AlreadyRunning(_)));

        // Real stop: Running -> Stopped.
        mgr.stop_application(&app_id).await.unwrap();
        assert_eq!(
            mgr.get_application_state(&app_id).unwrap(),
            ModuleState::Stopped
        );

        let health = mgr.health_check().await.unwrap();
        assert_eq!(health.modules_loaded, 0);
        assert!(health.operational);
    }

    #[tokio::test]
    async fn test_start_application_unknown_app_errors() {
        let temp = tempfile::tempdir().unwrap();
        let config = repo_config("http://127.0.0.1:1".to_string(), temp.path().join("cache"));
        let mgr = ApplicationManager::with_apps_dir_and_config(temp.path().join("apps"), config);

        let app_id = AppId::new("never-installed").unwrap();
        let err = mgr.start_application(&app_id).await.unwrap_err();
        assert!(matches!(err, AppIntegrationError::Lifecycle(_)));
    }

    #[tokio::test]
    async fn test_unload_application_from_loaded_state() {
        let temp = tempfile::tempdir().unwrap();
        let app_id = AppId::new("loaded-app").unwrap();
        let version = Version::new(1, 0, 0);
        let manifest = test_manifest(&app_id, &version);

        let marketplace_url = spawn_mock_marketplace(manifest).await;
        let config = repo_config(marketplace_url, temp.path().join("cache"));
        let mgr = ApplicationManager::with_apps_dir_and_config(temp.path().join("apps"), config);

        mgr.load_application(&app_id, &version).await.unwrap();

        // Drive directly to Loaded (without Running) via the real lifecycle
        // manager so uninstall() takes the "not running, unload straight
        // from Loaded" path, which is a legal transition.
        mgr.installer
            .get_lifecycle_manager()
            .load(&app_id)
            .await
            .unwrap();
        assert_eq!(
            mgr.get_application_state(&app_id).unwrap(),
            ModuleState::Loaded
        );

        mgr.unload_application(&app_id).await.unwrap();
        assert_eq!(
            mgr.get_application_state(&app_id).unwrap(),
            ModuleState::Unloaded
        );

        let health = mgr.health_check().await.unwrap();
        assert_eq!(health.modules_loaded, 0);
    }

    #[tokio::test]
    async fn test_load_application_network_failure_maps_to_installer_error() {
        // No mock marketplace listening on this port: the real HTTP fetch
        // inside Installer::install will fail, and that failure must come
        // back through our error type, not be swallowed.
        let temp = tempfile::tempdir().unwrap();
        let config = repo_config(
            "http://127.0.0.1:1".to_string(),
            temp.path().join("cache"),
        );
        let mgr = ApplicationManager::with_apps_dir_and_config(temp.path().join("apps"), config);

        let app_id = AppId::new("unreachable-app").unwrap();
        let version = Version::new(1, 0, 0);

        let err = mgr.load_application(&app_id, &version).await.unwrap_err();
        assert!(matches!(err, AppIntegrationError::Installer(_)));
    }

    #[tokio::test]
    async fn test_list_applications_reflects_registrations() {
        let temp = tempfile::tempdir().unwrap();
        let app_id = AppId::new("listed-app").unwrap();
        let version = Version::new(1, 0, 0);
        let manifest = test_manifest(&app_id, &version);

        let marketplace_url = spawn_mock_marketplace(manifest).await;
        let config = repo_config(marketplace_url, temp.path().join("cache"));
        let mgr = ApplicationManager::with_apps_dir_and_config(temp.path().join("apps"), config);

        assert!(mgr.list_applications().is_empty());

        mgr.load_application(&app_id, &version).await.unwrap();

        let apps = mgr.list_applications();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].0, app_id);
        assert_eq!(apps[0].1, ModuleState::Installed);
    }
}
