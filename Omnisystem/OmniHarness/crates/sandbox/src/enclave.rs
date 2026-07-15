//! Enclave: the top-level handle that ties together the manifest, lockfile,
//! resolver, content-addressed store, and environment manager into the
//! `enclave init/add/lock/install/shell/run` workflow used by the CLI.

use crate::cas::ContentAddressedStore;
use crate::environment::{Environment, EnvironmentManager};
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::resolver::DependencyResolver;
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct EnclaveConfig {
    pub root_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub lockfile_path: PathBuf,
    pub cas_dir: PathBuf,
    pub env_dir: PathBuf,
}

impl EnclaveConfig {
    pub fn new(root_dir: PathBuf) -> Result<Self> {
        let cas_dir = root_dir.join(".enclave").join("cas");
        let env_dir = root_dir.join(".enclave").join("envs");
        std::fs::create_dir_all(&cas_dir)?;
        std::fs::create_dir_all(&env_dir)?;

        Ok(Self {
            manifest_path: root_dir.join("enclave.toml"),
            lockfile_path: root_dir.join("enclave.lock"),
            cas_dir,
            env_dir,
            root_dir,
        })
    }
}

pub struct Enclave {
    pub config: EnclaveConfig,
    cas: ContentAddressedStore,
    resolver: DependencyResolver,
    environments: EnvironmentManager,
}

impl Enclave {
    pub async fn new(config: EnclaveConfig) -> Result<Self> {
        let cas = ContentAddressedStore::new(config.cas_dir.clone()).await?;
        let environments = EnvironmentManager::new(config.env_dir.clone()).await?;

        Ok(Self {
            config,
            cas,
            resolver: DependencyResolver::new(),
            environments,
        })
    }

    pub fn cas(&self) -> &ContentAddressedStore {
        &self.cas
    }

    /// Load the project manifest, or an empty one if it doesn't exist yet.
    pub async fn load_manifest(&self) -> Result<Manifest> {
        if self.config.manifest_path.exists() {
            Manifest::load(&self.config.manifest_path).await
        } else {
            Ok(Manifest::default())
        }
    }

    /// Load the lockfile, or an empty one if it doesn't exist yet.
    pub async fn load_lockfile(&self) -> Result<Lockfile> {
        if self.config.lockfile_path.exists() {
            Lockfile::load(&self.config.lockfile_path).await
        } else {
            Ok(Lockfile::new())
        }
    }

    /// Resolve the manifest's dependencies deterministically and persist the lockfile.
    pub async fn lock(&mut self) -> Result<Lockfile> {
        let manifest = self.load_manifest().await?;
        let lockfile = self.resolver.resolve(&manifest).await?;
        lockfile.save(&self.config.lockfile_path).await?;
        Ok(lockfile)
    }

    /// Create (or recreate) a named isolated environment from the current lockfile.
    pub async fn create_environment(&mut self, name: &str) -> Result<Environment> {
        let lockfile = self.load_lockfile().await?;
        self.environments.create(name, &lockfile).await
    }

    /// Run a command inside a named environment, creating it first if needed.
    pub async fn run(&mut self, env_name: &str, args: &[&str]) -> Result<()> {
        let env = match self.environments.get(env_name).await {
            Ok(env) => env,
            Err(_) => self.create_environment(env_name).await?,
        };
        env.run_command(args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enclave_config_creates_dirs() {
        let tmpdir = tempfile::tempdir().unwrap();
        let config = EnclaveConfig::new(tmpdir.path().to_path_buf()).unwrap();
        assert!(config.cas_dir.exists());
        assert!(config.env_dir.exists());
    }

    #[tokio::test]
    async fn test_enclave_lock_empty_manifest() {
        let tmpdir = tempfile::tempdir().unwrap();
        let config = EnclaveConfig::new(tmpdir.path().to_path_buf()).unwrap();
        let mut enclave = Enclave::new(config).await.unwrap();

        let lockfile = enclave.lock().await.unwrap();
        assert!(lockfile.packages.is_empty());
        assert!(tmpdir.path().join("enclave.lock").exists());
    }

    #[tokio::test]
    async fn test_enclave_create_environment() {
        let tmpdir = tempfile::tempdir().unwrap();
        let config = EnclaveConfig::new(tmpdir.path().to_path_buf()).unwrap();
        let mut enclave = Enclave::new(config).await.unwrap();

        let env = enclave.create_environment("default").await.unwrap();
        assert_eq!(env.name, "default");
        assert!(env.bin_dir.exists());
    }
}
