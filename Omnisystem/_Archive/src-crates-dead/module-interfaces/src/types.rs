use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ModuleId(pub String);

impl ModuleId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build: Option<String>,
}

impl ModuleVersion {
    pub fn parse(version_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() < 3 {
            return Err("Invalid version format".to_string());
        }

        let major = parts[0].parse::<u32>().map_err(|_| "Invalid major version")?;
        let minor = parts[1].parse::<u32>().map_err(|_| "Invalid minor version")?;
        let patch_part = parts[2];

        let (patch, pre_release) = if let Some(hyphen_pos) = patch_part.find('-') {
            let patch = patch_part[..hyphen_pos].parse::<u32>().map_err(|_| "Invalid patch version")?;
            let pre = patch_part[hyphen_pos + 1..].to_string();
            (patch, Some(pre))
        } else {
            let patch = patch_part.parse::<u32>().map_err(|_| "Invalid patch version")?;
            (patch, None)
        };

        Ok(ModuleVersion {
            major,
            minor,
            patch,
            pre_release,
            build: None,
        })
    }

    pub fn to_string_canonical(&self) -> String {
        let base = format!("{}.{}.{}", self.major, self.minor, self.patch);
        match (&self.pre_release, &self.build) {
            (Some(pre), Some(build)) => format!("{}-{}+{}", base, pre, build),
            (Some(pre), None) => format!("{}-{}", base, pre),
            (None, Some(build)) => format!("{}+{}", base, build),
            (None, None) => base,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionConstraint {
    pub constraint_type: VersionConstraintType,
    pub version: ModuleVersion,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VersionConstraintType {
    Exact,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Compatible,
    Approximate,
}

impl VersionConstraint {
    pub fn satisfies(&self, version: &ModuleVersion) -> bool {
        match self.constraint_type {
            VersionConstraintType::Exact => {
                version.major == self.version.major
                    && version.minor == self.version.minor
                    && version.patch == self.version.patch
            }
            VersionConstraintType::GreaterThan => {
                (version.major > self.version.major)
                    || (version.major == self.version.major && version.minor > self.version.minor)
                    || (version.major == self.version.major
                        && version.minor == self.version.minor
                        && version.patch > self.version.patch)
            }
            VersionConstraintType::GreaterThanOrEqual => {
                (version.major > self.version.major)
                    || (version.major == self.version.major && version.minor > self.version.minor)
                    || (version.major == self.version.major
                        && version.minor == self.version.minor
                        && version.patch >= self.version.patch)
            }
            VersionConstraintType::LessThan => {
                (version.major < self.version.major)
                    || (version.major == self.version.major && version.minor < self.version.minor)
                    || (version.major == self.version.major
                        && version.minor == self.version.minor
                        && version.patch < self.version.patch)
            }
            VersionConstraintType::LessThanOrEqual => {
                (version.major < self.version.major)
                    || (version.major == self.version.major && version.minor < self.version.minor)
                    || (version.major == self.version.major
                        && version.minor == self.version.minor
                        && version.patch <= self.version.patch)
            }
            VersionConstraintType::Compatible => {
                version.major == self.version.major && version.minor >= self.version.minor
            }
            VersionConstraintType::Approximate => {
                version.major == self.version.major && version.minor == self.version.minor
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleCapability {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<ModuleVersion>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: ModuleId,
    pub name: String,
    pub version: ModuleVersion,
    pub description: String,
    pub author: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub capabilities: Vec<ModuleCapability>,
    pub dependencies: Vec<ModuleDep>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleDep {
    pub module_id: ModuleId,
    pub constraint: VersionConstraint,
    pub optional: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleStats {
    pub id: ModuleId,
    pub load_count: u64,
    pub execution_count: u64,
    pub error_count: u64,
    pub success_rate: f64,
    pub avg_load_time_ms: f64,
    pub avg_execution_time_ms: f64,
    pub last_accessed: u64,
    pub uptime_seconds: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleEvent {
    pub id: String,
    pub module_id: ModuleId,
    pub event_type: ModuleEventType,
    pub timestamp: u64,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModuleEventType {
    Registered,
    Loaded,
    Executed,
    Configured,
    Updated,
    Unloaded,
    Error,
    HealthCheckPassed,
    HealthCheckFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_id_creation() {
        let id = ModuleId::new("test-module");
        assert_eq!(id.as_str(), "test-module");
    }

    #[test]
    fn test_module_version_parsing() {
        let version = ModuleVersion::parse("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_module_version_parsing_with_pre_release() {
        let version = ModuleVersion::parse("1.2.3-alpha").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.pre_release, Some("alpha".to_string()));
    }

    #[test]
    fn test_module_version_canonical_string() {
        let version = ModuleVersion {
            major: 1,
            minor: 2,
            patch: 3,
            pre_release: Some("alpha".to_string()),
            build: None,
        };
        assert_eq!(version.to_string_canonical(), "1.2.3-alpha");
    }

    #[test]
    fn test_version_constraint_greater_than_or_equal() {
        let constraint = VersionConstraint {
            constraint_type: VersionConstraintType::GreaterThanOrEqual,
            version: ModuleVersion {
                major: 1,
                minor: 0,
                patch: 0,
                pre_release: None,
                build: None,
            },
        };

        let version_satisfied = ModuleVersion {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: None,
            build: None,
        };

        let version_not_satisfied = ModuleVersion {
            major: 0,
            minor: 9,
            patch: 9,
            pre_release: None,
            build: None,
        };

        assert!(constraint.satisfies(&version_satisfied));
        assert!(!constraint.satisfies(&version_not_satisfied));
    }

    #[test]
    fn test_module_capability_creation() {
        let cap = ModuleCapability {
            name: "realtime-processing".to_string(),
            description: Some("Real-time event processing".to_string()),
            version: None,
        };
        assert_eq!(cap.name, "realtime-processing");
    }

    #[test]
    fn test_module_event_creation() {
        let event = ModuleEvent {
            id: "event-1".to_string(),
            module_id: ModuleId::new("test"),
            event_type: ModuleEventType::Loaded,
            timestamp: 0,
            metadata: HashMap::new(),
        };
        assert_eq!(event.module_id.as_str(), "test");
    }

    #[test]
    fn test_module_stats_creation() {
        let stats = ModuleStats {
            id: ModuleId::new("test"),
            load_count: 100,
            execution_count: 500,
            error_count: 2,
            success_rate: 99.6,
            avg_load_time_ms: 25.5,
            avg_execution_time_ms: 10.2,
            last_accessed: 0,
            uptime_seconds: 86400,
        };
        assert_eq!(stats.load_count, 100);
        assert!(stats.success_rate > 99.0);
    }
}
