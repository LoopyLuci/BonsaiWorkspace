//! Isolated project environments

use crate::lockfile::Lockfile;
use anyhow::Result;
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone)]
pub struct Environment {
    pub name: String,
    pub path: PathBuf,
    pub bin_dir: PathBuf,
    pub lib_dir: PathBuf,
}

impl Environment {
    pub async fn run_command(&self, command: &[&str]) -> Result<()> {
        // For MVP: run command with PATH set to include bin_dir
        let status = std::process::Command::new(command[0])
            .args(&command[1..])
            .env("PATH", format!("{}{}{}",
                self.bin_dir.display(),
                std::path::MAIN_SEPARATOR,
                std::env::var("PATH").unwrap_or_default()
            ))
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Command failed with status {}", status));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct EnvironmentManager {
    root: PathBuf,
}

impl EnvironmentManager {
    pub async fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root).await?;
        Ok(Self { root })
    }

    pub async fn create(&mut self, name: &str, _lockfile: &Lockfile) -> Result<Environment> {
        let env_path = self.root.join(name);
        let bin_dir = env_path.join("bin");
        let lib_dir = env_path.join("lib");

        fs::create_dir_all(&bin_dir).await?;
        fs::create_dir_all(&lib_dir).await?;

        Ok(Environment {
            name: name.to_string(),
            path: env_path,
            bin_dir,
            lib_dir,
        })
    }

    pub async fn get(&self, name: &str) -> Result<Environment> {
        let env_path = self.root.join(name);
        if !env_path.exists() {
            return Err(anyhow::anyhow!("Environment {} not found", name));
        }

        Ok(Environment {
            name: name.to_string(),
            path: env_path.clone(),
            bin_dir: env_path.join("bin"),
            lib_dir: env_path.join("lib"),
        })
    }
}
