use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AppId(pub String);

impl AppId {
    pub fn new(id: impl Into<String>) -> Result<Self, crate::AppManagerError> {
        let id = id.into();
        if id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') && !id.is_empty() {
            Ok(AppId(id))
        } else {
            Err(crate::AppManagerError::InvalidAppId(id))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AppId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Version { major, minor, patch }
    }

    pub fn parse(s: &str) -> Result<Self, crate::AppManagerError> {
        let parts: Vec<&str> = s.trim_start_matches('v').split('.').collect();
        if parts.len() != 3 {
            return Err(crate::AppManagerError::InvalidVersion(s.to_string()));
        }
        Ok(Version {
            major: parts[0].parse().map_err(|_| crate::AppManagerError::InvalidVersion(s.to_string()))?,
            minor: parts[1].parse().map_err(|_| crate::AppManagerError::InvalidVersion(s.to_string()))?,
            patch: parts[2].parse().map_err(|_| crate::AppManagerError::InvalidVersion(s.to_string()))?,
        })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VersionConstraint {
    Exact(Version),
    AtLeast(Version),
    AtMost(Version),
    Range(Version, Version),
    Compatible(Version),
    Approximate(Version),
}

impl VersionConstraint {
    pub fn satisfies(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::AtLeast(v) => version >= v,
            VersionConstraint::AtMost(v) => version <= v,
            VersionConstraint::Range(min, max) => version >= min && version <= max,
            VersionConstraint::Compatible(v) => {
                version.major == v.major && version >= v
            }
            VersionConstraint::Approximate(v) => {
                version.major == v.major && version.minor == v.minor && version >= v
            }
        }
    }

    pub fn parse(s: &str) -> Result<Self, crate::AppManagerError> {
        if let Some(exact) = s.strip_prefix('=') {
            Ok(VersionConstraint::Exact(Version::parse(exact)?))
        } else if let Some(compatible) = s.strip_prefix('^') {
            Ok(VersionConstraint::Compatible(Version::parse(compatible)?))
        } else if let Some(approx) = s.strip_prefix('~') {
            Ok(VersionConstraint::Approximate(Version::parse(approx)?))
        } else if let Some(at_least) = s.strip_prefix(">=") {
            Ok(VersionConstraint::AtLeast(Version::parse(at_least)?))
        } else if let Some(at_most) = s.strip_prefix("<=") {
            Ok(VersionConstraint::AtMost(Version::parse(at_most)?))
        } else {
            Ok(VersionConstraint::AtLeast(Version::parse(s)?))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub app_id: AppId,
    pub version_constraint: VersionConstraint,
    pub optional: bool,
    pub dev_only: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleState {
    Discovered,
    Downloading,
    Downloaded,
    Verifying,
    Verified,
    Installing,
    Installed,
    Loading,
    Loaded,
    Running,
    Stopped,
    Unloading,
    Unloaded,
    Failed,
    Corrupted,
}

impl ModuleState {
    pub fn is_ready(&self) -> bool {
        matches!(self, ModuleState::Loaded | ModuleState::Running)
    }

    pub fn is_transitioning(&self) -> bool {
        matches!(
            self,
            ModuleState::Downloading | ModuleState::Verifying | ModuleState::Installing | ModuleState::Loading | ModuleState::Unloading
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, ModuleState::Failed | ModuleState::Corrupted)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallRequest {
    pub app_id: AppId,
    pub version: Version,
    pub source: InstallSource,
    pub auto_load_deps: bool,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallSource {
    GitHub(String),
    Marketplace(String),
    LocalFile(String),
    PrivateRegistry(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub app_id: AppId,
    pub name: String,
    pub version: Version,
    pub description: String,
    pub author: String,
    pub license: String,
    pub dependencies: Vec<Dependency>,
    pub modules: HashMap<String, String>,
    pub entry_points: HashMap<String, String>,
    pub permissions: HashSet<String>,
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: AppId,
    pub version: Version,
    pub state: ModuleState,
    pub loaded_at: Option<DateTime<Utc>>,
    pub dependencies: Vec<Dependency>,
    pub manifest: Manifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub app_id: AppId,
    pub conflicting_app_id: AppId,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub app_id: AppId,
    pub version: Version,
    pub required_by: Vec<AppId>,
    pub requires: Vec<Dependency>,
}

#[derive(Debug, Clone)]
pub struct ModuleNode {
    pub id: AppId,
    pub version: Version,
    pub dependencies: Vec<Dependency>,
    pub dependents: Vec<AppId>,
    pub conflicts: Vec<ConflictInfo>,
    pub state: ModuleState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TrustLevel {
    Unknown,
    Unverified,
    Verified,
    Certified,
    Official,
}
