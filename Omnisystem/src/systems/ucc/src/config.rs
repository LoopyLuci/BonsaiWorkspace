//! Configuration management

use crate::core::CompileTarget;
use crate::error::Result;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// UnixCC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub project_root: PathBuf,
    pub cache_dir: PathBuf,
    pub target: CompileTarget,
    pub optimization_level: u8,
    pub num_threads: usize,
    pub enable_incremental: bool,
    pub enable_cache: bool,
    pub distributed_build: bool,
}

impl Config {
    /// Create a new config with defaults
    pub fn new(project_root: PathBuf) -> Self {
        let cache_dir = project_root.join(".unixcc-cache");

        Self {
            project_root: project_root.clone(),
            cache_dir,
            target: CompileTarget::native(),
            optimization_level: 2,
            num_threads: num_cpus::get(),
            enable_incremental: true,
            enable_cache: true,
            distributed_build: false,
        }
    }

    /// Load from file
    pub fn from_file(_path: &PathBuf) -> Result<Self> {
        // Phase 1: Stub
        Ok(Self::new(PathBuf::from(".")))
    }

    /// Save to file
    pub fn to_file(&self, _path: &PathBuf) -> Result<()> {
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}
