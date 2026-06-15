//! Permission and capability types

use serde::{Deserialize, Serialize};

/// Permission category for capability-based security
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PermissionCategory {
    FileSystem,
    Network,
    Process,
    Hardware,
    Memory,
    GPU,
    Audio,
    Video,
    Camera,
    Microphone,
    Geolocation,
}

impl std::fmt::Display for PermissionCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionCategory::FileSystem => write!(f, "filesystem"),
            PermissionCategory::Network => write!(f, "network"),
            PermissionCategory::Process => write!(f, "process"),
            PermissionCategory::Hardware => write!(f, "hardware"),
            PermissionCategory::Memory => write!(f, "memory"),
            PermissionCategory::GPU => write!(f, "gpu"),
            PermissionCategory::Audio => write!(f, "audio"),
            PermissionCategory::Video => write!(f, "video"),
            PermissionCategory::Camera => write!(f, "camera"),
            PermissionCategory::Microphone => write!(f, "microphone"),
            PermissionCategory::Geolocation => write!(f, "geolocation"),
        }
    }
}

/// Risk level classification for permissions
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "low"),
            RiskLevel::Medium => write!(f, "medium"),
            RiskLevel::High => write!(f, "high"),
            RiskLevel::Critical => write!(f, "critical"),
        }
    }
}

/// Permission requirement for an app or module
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: PermissionCategory,
    pub risk_level: RiskLevel,
}

impl Permission {
    pub fn new(
        id: String,
        name: String,
        category: PermissionCategory,
        risk_level: RiskLevel,
    ) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            category,
            risk_level,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_creation() {
        let perm = Permission::new(
            "fs.read".to_string(),
            "Read Files".to_string(),
            PermissionCategory::FileSystem,
            RiskLevel::Medium,
        );

        assert_eq!(perm.id, "fs.read");
        assert_eq!(perm.category, PermissionCategory::FileSystem);
        assert_eq!(perm.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_permission_with_description() {
        let perm = Permission::new(
            "net.http".to_string(),
            "HTTP Requests".to_string(),
            PermissionCategory::Network,
            RiskLevel::High,
        )
        .with_description("Allow HTTP requests to external servers".to_string());

        assert_eq!(perm.description, "Allow HTTP requests to external servers");
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_permission_category_display() {
        assert_eq!(PermissionCategory::FileSystem.to_string(), "filesystem");
        assert_eq!(PermissionCategory::Network.to_string(), "network");
    }
}
