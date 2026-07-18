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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_root_is_under_home() {
        let home = home_dir().expect("home dir should resolve in test environment");
        let eco = ecosystem_root().unwrap();
        assert_eq!(eco, home.join("Bonsai-Ecosystem"));
    }

    #[test]
    fn state_and_rollback_dirs_nest_correctly() {
        let home = home_dir().unwrap();
        let state = state_dir().unwrap();
        let rollback = rollback_dir().unwrap();
        assert_eq!(state, home.join(".bonsai").join("root"));
        assert_eq!(rollback, state.join("rollback"));
    }
}
