//! Dependency modeling and constraints

use serde::{Deserialize, Serialize};
use semver::Version;
use crate::error::AppManagerResult;

/// Kind of dependency relationship
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum DependencyKind {
    Runtime,
    BuildTime,
    Optional,
}

impl std::fmt::Display for DependencyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyKind::Runtime => write!(f, "runtime"),
            DependencyKind::BuildTime => write!(f, "build"),
            DependencyKind::Optional => write!(f, "optional"),
        }
    }
}

/// Version constraint (SemVer compatible)
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum VersionConstraint {
    Exact(Version),
    Caret(Version),           // ^1.2.3 - >=1.2.3, <2.0.0
    Tilde(Version),           // ~1.2.3 - >=1.2.3, <1.3.0
    GreaterEqual(Version),    // >=1.2.3
    LessEqual(Version),       // <=1.2.3
    Greater(Version),         // >1.2.3
    Less(Version),            // <1.2.3
    Range(Version, Version),  // 1.2.3 - 2.0.0
}

impl VersionConstraint {
    pub fn satisfies(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::Caret(v) => {
                version >= v && version.major == v.major
            }
            VersionConstraint::Tilde(v) => {
                version >= v && version.major == v.major && version.minor == v.minor
            }
            VersionConstraint::GreaterEqual(v) => version >= v,
            VersionConstraint::LessEqual(v) => version <= v,
            VersionConstraint::Greater(v) => version > v,
            VersionConstraint::Less(v) => version < v,
            VersionConstraint::Range(min, max) => version >= min && version <= max,
        }
    }

    pub fn parse(s: &str) -> AppManagerResult<Self> {
        let s = s.trim();

        if let Some(exact) = s.strip_prefix('=') {
            return Ok(VersionConstraint::Exact(Version::parse(exact.trim())?));
        }

        if let Some(caret) = s.strip_prefix('^') {
            return Ok(VersionConstraint::Caret(Version::parse(caret.trim())?));
        }

        if let Some(tilde) = s.strip_prefix('~') {
            return Ok(VersionConstraint::Tilde(Version::parse(tilde.trim())?));
        }

        if let Some(ge) = s.strip_prefix(">=") {
            return Ok(VersionConstraint::GreaterEqual(Version::parse(ge.trim())?));
        }

        if let Some(le) = s.strip_prefix("<=") {
            return Ok(VersionConstraint::LessEqual(Version::parse(le.trim())?));
        }

        if let Some(gt) = s.strip_prefix('>') {
            return Ok(VersionConstraint::Greater(Version::parse(gt.trim())?));
        }

        if let Some(lt) = s.strip_prefix('<') {
            return Ok(VersionConstraint::Less(Version::parse(lt.trim())?));
        }

        if s.contains('-') {
            let parts: Vec<&str> = s.split('-').collect();
            if parts.len() == 2 {
                let min = Version::parse(parts[0].trim())?;
                let max = Version::parse(parts[1].trim())?;
                return Ok(VersionConstraint::Range(min, max));
            }
        }

        // Default to GreaterEqual
        Ok(VersionConstraint::GreaterEqual(Version::parse(s)?))
    }

}

impl std::fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionConstraint::Exact(v) => write!(f, "={}", v),
            VersionConstraint::Caret(v) => write!(f, "^{}", v),
            VersionConstraint::Tilde(v) => write!(f, "~{}", v),
            VersionConstraint::GreaterEqual(v) => write!(f, ">={}", v),
            VersionConstraint::LessEqual(v) => write!(f, "<={}", v),
            VersionConstraint::Greater(v) => write!(f, ">{}", v),
            VersionConstraint::Less(v) => write!(f, "<{}", v),
            VersionConstraint::Range(min, max) => write!(f, "{} - {}", min, max),
        }
    }
}

/// App-level dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version_constraint: VersionConstraint,
    pub optional: bool,
    pub kind: DependencyKind,
}

impl Dependency {
    pub fn new(name: String, version_constraint: VersionConstraint) -> Self {
        Self {
            name,
            version_constraint,
            optional: false,
            kind: DependencyKind::Runtime,
        }
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    pub fn with_kind(mut self, kind: DependencyKind) -> Self {
        self.kind = kind;
        self
    }
}

/// Module-level dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependency {
    pub name: String,
    pub version_constraint: VersionConstraint,
    pub optional: bool,
}

impl ModuleDependency {
    pub fn new(name: String, version_constraint: VersionConstraint) -> Self {
        Self {
            name,
            version_constraint,
            optional: false,
        }
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constraint_caret() {
        let constraint = VersionConstraint::Caret(Version::parse("1.2.3").unwrap());
        assert!(constraint.satisfies(&Version::parse("1.3.0").unwrap()));
        assert!(constraint.satisfies(&Version::parse("1.9.9").unwrap()));
        assert!(!constraint.satisfies(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_version_constraint_tilde() {
        let constraint = VersionConstraint::Tilde(Version::parse("1.2.3").unwrap());
        assert!(constraint.satisfies(&Version::parse("1.2.4").unwrap()));
        assert!(!constraint.satisfies(&Version::parse("1.3.0").unwrap()));
    }

    #[test]
    fn test_version_constraint_parse() {
        assert!(matches!(
            VersionConstraint::parse("^1.2.3").unwrap(),
            VersionConstraint::Caret(_)
        ));

        assert!(matches!(
            VersionConstraint::parse("~1.2.3").unwrap(),
            VersionConstraint::Tilde(_)
        ));

        assert!(matches!(
            VersionConstraint::parse(">=1.2.3").unwrap(),
            VersionConstraint::GreaterEqual(_)
        ));
    }

    #[test]
    fn test_dependency_creation() {
        let dep = Dependency::new(
            "test-lib".to_string(),
            VersionConstraint::Caret(Version::parse("1.0.0").unwrap()),
        );

        assert_eq!(dep.name, "test-lib");
        assert!(!dep.optional);
        assert_eq!(dep.kind, DependencyKind::Runtime);
    }

    #[test]
    fn test_dependency_optional() {
        let dep = Dependency::new(
            "test-lib".to_string(),
            VersionConstraint::Caret(Version::parse("1.0.0").unwrap()),
        )
        .optional();

        assert!(dep.optional);
    }
}
