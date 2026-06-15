pub mod transaction;

use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub fn install_path() -> Result<PathBuf> {
    crate::utils::ecosystem_root()
}

pub fn ensure_install_root() -> Result<PathBuf> {
    let root = install_path()?;
    fs::create_dir_all(&root)?;
    Ok(root)
}
