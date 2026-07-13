use anyhow::{anyhow, Result};
use std::path::PathBuf;

pub fn home_dir() -> Result<PathBuf> {
    if let Some(v) = std::env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(v));
    }
    if let Some(v) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(v));
    }
    Err(anyhow!("could not resolve user home directory"))
}

pub fn ecosystem_root() -> Result<PathBuf> {
    Ok(home_dir()?.join("Bonsai-Ecosystem"))
}

pub fn state_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".bonsai").join("root"))
}

pub fn rollback_dir() -> Result<PathBuf> {
    Ok(state_dir()?.join("rollback"))
}
