//! Data Manager - Separation of concerns for data storage
//!
//! Data is stored in separate locations:
//! - System: Shared configuration, system-wide data
//! - User: User-specific settings, preferences
//! - Device: Device-specific configuration, hardware profiles
//! - Temporary: Build artifacts, caches, session state

use std::path::{Path, PathBuf};
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Data storage manager with proper separation of concerns
pub struct DataManager {
    system_data: PathBuf,
    user_data: PathBuf,
    device_data: PathBuf,
    temp_data: PathBuf,
}

impl DataManager {
    /// Create data manager with default paths
    pub fn new() -> Result<Self> {
        let system_data = Self::default_system_path();
        let user_data = Self::default_user_path();
        let device_data = Self::default_device_path();
        let temp_data = Self::default_temp_path();

        // Create directories if they don't exist
        std::fs::create_dir_all(&system_data)?;
        std::fs::create_dir_all(&user_data)?;
        std::fs::create_dir_all(&device_data)?;
        std::fs::create_dir_all(&temp_data)?;

        Ok(Self {
            system_data,
            user_data,
            device_data,
            temp_data,
        })
    }

    /// Create with custom paths
    pub fn with_paths(
        system: PathBuf,
        user: PathBuf,
        device: PathBuf,
        temp: PathBuf,
    ) -> Result<Self> {
        std::fs::create_dir_all(&system)?;
        std::fs::create_dir_all(&user)?;
        std::fs::create_dir_all(&device)?;
        std::fs::create_dir_all(&temp)?;

        Ok(Self {
            system_data: system,
            user_data: user,
            device_data: device,
            temp_data: temp,
        })
    }

    // Path getters
    pub fn system_path(&self) -> &Path {
        &self.system_data
    }

    pub fn user_path(&self) -> &Path {
        &self.user_data
    }

    pub fn device_path(&self) -> &Path {
        &self.device_data
    }

    pub fn temp_path(&self) -> &Path {
        &self.temp_data
    }

    // Module data paths
    pub fn module_system_data(&self, module_name: &str) -> PathBuf {
        self.system_data.join("modules").join(module_name)
    }

    pub fn module_user_data(&self, module_name: &str) -> PathBuf {
        self.user_data.join("modules").join(module_name)
    }

    pub fn module_device_data(&self, module_name: &str) -> PathBuf {
        self.device_data.join("modules").join(module_name)
    }

    pub fn module_temp_data(&self, module_name: &str) -> PathBuf {
        self.temp_data.join("modules").join(module_name)
    }

    // Utility methods
    pub fn ensure_module_dir(&self, module_name: &str, location: DataLocation) -> Result<PathBuf> {
        let path = match location {
            DataLocation::System => self.module_system_data(module_name),
            DataLocation::User => self.module_user_data(module_name),
            DataLocation::Device => self.module_device_data(module_name),
            DataLocation::Temp => self.module_temp_data(module_name),
        };

        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// Store JSON data
    pub fn store_json<T: Serialize>(
        &self,
        module: &str,
        key: &str,
        data: &T,
        location: DataLocation,
    ) -> Result<()> {
        let dir = self.ensure_module_dir(module, location)?;
        let path = dir.join(format!("{}.json", key));
        let json = serde_json::to_string_pretty(data)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load JSON data
    pub fn load_json<T: for<'de> Deserialize<'de>>(
        &self,
        module: &str,
        key: &str,
        location: DataLocation,
    ) -> Result<T> {
        let dir = match location {
            DataLocation::System => self.module_system_data(module),
            DataLocation::User => self.module_user_data(module),
            DataLocation::Device => self.module_device_data(module),
            DataLocation::Temp => self.module_temp_data(module),
        };

        let path = dir.join(format!("{}.json", key));
        let contents = std::fs::read_to_string(path)?;
        let data = serde_json::from_str(&contents)?;
        Ok(data)
    }

    // Default path implementations
    fn default_system_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            PathBuf::from(
                std::env::var("ProgramData")
                    .unwrap_or_else(|_| "C:\\ProgramData".to_string()),
            )
            .join("Omnisystem")
        }

        #[cfg(target_os = "macos")]
        {
            PathBuf::from("/Library/Application Support/Omnisystem")
        }

        #[cfg(target_os = "linux")]
        {
            PathBuf::from("/var/lib/omnisystem")
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            PathBuf::from("/usr/local/share/omnisystem")
        }
    }

    fn default_user_path() -> PathBuf {
        directories::ProjectDirs::from("dev", "omnisystem", "omnisystem")
            .map(|d| PathBuf::from(d.config_dir()))
            .unwrap_or_else(|| PathBuf::from("~/.omnisystem"))
    }

    fn default_device_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            std::env::var("LOCALAPPDATA")
                .map(PathBuf::from)
                .map(|p| p.join("Omnisystem"))
                .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Public\\Omnisystem"))
        }

        #[cfg(target_os = "macos")]
        {
            PathBuf::from("/Library/Preferences/Omnisystem")
        }

        #[cfg(target_os = "linux")]
        {
            PathBuf::from("/etc/omnisystem")
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            PathBuf::from("/etc/omnisystem")
        }
    }

    fn default_temp_path() -> PathBuf {
        std::env::temp_dir().join("omnisystem")
    }

    /// Get total used disk space
    pub fn disk_usage(&self) -> Result<DiskUsage> {
        let system = Self::dir_size(&self.system_data)?;
        let user = Self::dir_size(&self.user_data)?;
        let device = Self::dir_size(&self.device_data)?;
        let temp = Self::dir_size(&self.temp_data)?;

        Ok(DiskUsage {
            system_bytes: system,
            user_bytes: user,
            device_bytes: device,
            temp_bytes: temp,
            total_bytes: system + user + device + temp,
        })
    }

    fn dir_size(path: &Path) -> std::io::Result<u64> {
        let mut size = 0u64;
        if path.exists() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                if metadata.is_file() {
                    size += metadata.len();
                } else if metadata.is_dir() {
                    size += Self::dir_size(&entry.path())?;
                }
            }
        }
        Ok(size)
    }
}

impl Default for DataManager {
    fn default() -> Self {
        Self::new().expect("Failed to create data manager")
    }
}

/// Data storage location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataLocation {
    System,
    User,
    Device,
    Temp,
}

impl std::fmt::Display for DataLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataLocation::System => write!(f, "System"),
            DataLocation::User => write!(f, "User"),
            DataLocation::Device => write!(f, "Device"),
            DataLocation::Temp => write!(f, "Temp"),
        }
    }
}

/// Disk usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub system_bytes: u64,
    pub user_bytes: u64,
    pub device_bytes: u64,
    pub temp_bytes: u64,
    pub total_bytes: u64,
}

impl DiskUsage {
    pub fn total_mb(&self) -> f64 {
        self.total_bytes as f64 / (1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_manager_paths() {
        let temp_dir = std::env::temp_dir().join("omnisystem-test");
        let dm = DataManager::with_paths(
            temp_dir.clone(),
            temp_dir.clone(),
            temp_dir.clone(),
            temp_dir.clone(),
        )
        .unwrap();

        assert_eq!(dm.system_path(), &temp_dir);
        assert_eq!(dm.user_path(), &temp_dir);
        assert_eq!(dm.device_path(), &temp_dir);
        assert_eq!(dm.temp_path(), &temp_dir);
    }

    #[test]
    fn test_module_paths() {
        let temp_dir = std::env::temp_dir().join("omnisystem-test");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let dm = DataManager::with_paths(
            temp_dir.clone(),
            temp_dir.clone(),
            temp_dir.clone(),
            temp_dir.clone(),
        )
        .unwrap();

        let module_path = dm.module_user_data("test-module");
        assert!(module_path.to_string_lossy().contains("test-module"));
    }

    #[test]
    fn test_json_storage() {
        use std::collections::HashMap;

        let temp_dir = std::env::temp_dir().join("omnisystem-json-test");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let dm = DataManager::with_paths(
            temp_dir.clone(),
            temp_dir.clone(),
            temp_dir.clone(),
            temp_dir.clone(),
        )
        .unwrap();

        let mut data = HashMap::new();
        data.insert("key", "value");

        dm.store_json("test", "config", &data, DataLocation::User)
            .unwrap();

        let loaded: HashMap<String, String> =
            dm.load_json("test", "config", DataLocation::User).unwrap();

        assert_eq!(loaded.get("key").map(|s| s.as_str()), Some("value"));
    }
}
