/// FFI Version Management - Backward/forward compatibility

use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Version { major, minor, patch }
    }

    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Check if this version is compatible with required version
    pub fn is_compatible_with(&self, required: Version) -> bool {
        // Major version must match (breaking changes)
        if self.major != required.major {
            return false;
        }

        // Minor version must be >= (backward compatible)
        if self.minor < required.minor {
            return false;
        }

        // Patch version can be different (bug fixes)
        true
    }

    /// Check if this version satisfies a version requirement
    pub fn satisfies_requirement(&self, requirement: &VersionRequirement) -> bool {
        match requirement {
            VersionRequirement::Exact(v) => self == v,
            VersionRequirement::AtLeast(v) => self >= v,
            VersionRequirement::AtMost(v) => self <= v,
            VersionRequirement::Range(min, max) => self >= min && self <= max,
            VersionRequirement::CompatibleWith(v) => self.is_compatible_with(*v),
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => self.patch.cmp(&other.patch),
                other => other,
            },
            other => other,
        }
    }
}

#[derive(Debug, Clone)]
pub enum VersionRequirement {
    /// Exact version match
    Exact(Version),
    /// Version >= specified
    AtLeast(Version),
    /// Version <= specified
    AtMost(Version),
    /// Version range (min, max)
    Range(Version, Version),
    /// Compatible version (same major, >= minor)
    CompatibleWith(Version),
}

impl VersionRequirement {
    pub fn satisfied_by(&self, version: &Version) -> bool {
        version.satisfies_requirement(self)
    }
}

/// API versioning information
pub struct APIVersion {
    pub name: String,
    pub version: Version,
    pub min_supported: Version,
    pub max_supported: Version,
}

impl APIVersion {
    pub fn new(
        name: &str,
        version: Version,
        min_supported: Version,
        max_supported: Version,
    ) -> Self {
        APIVersion {
            name: name.to_string(),
            version,
            min_supported,
            max_supported,
        }
    }

    pub fn can_support_client(&self, client_version: Version) -> bool {
        client_version >= self.min_supported && client_version <= self.max_supported
    }
}

/// Module versioning
pub struct ModuleVersion {
    pub name: String,
    pub version: Version,
    pub dependencies: Vec<(String, VersionRequirement)>,
}

impl ModuleVersion {
    pub fn new(name: &str, version: Version) -> Self {
        ModuleVersion {
            name: name.to_string(),
            version,
            dependencies: Vec::new(),
        }
    }

    pub fn add_dependency(&mut self, name: &str, requirement: VersionRequirement) {
        self.dependencies.push((name.to_string(), requirement));
    }

    pub fn check_dependencies(
        &self,
        available_modules: &[(String, Version)],
    ) -> Result<(), String> {
        for (dep_name, requirement) in &self.dependencies {
            let found = available_modules.iter().find(|(name, _)| name == dep_name);

            match found {
                Some((_, version)) => {
                    if !requirement.satisfied_by(version) {
                        return Err(format!(
                            "Dependency {} requires {:?}, but found {}",
                            dep_name, requirement, version.to_string()
                        ));
                    }
                }
                None => {
                    return Err(format!("Dependency {} not found", dep_name));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 0, 1);
        let v3 = Version::new(1, 1, 0);
        let v4 = Version::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
    }

    #[test]
    fn test_version_compatibility() {
        let v1_0_0 = Version::new(1, 0, 0);
        let v1_0_1 = Version::new(1, 0, 1);
        let v1_1_0 = Version::new(1, 1, 0);
        let v2_0_0 = Version::new(2, 0, 0);

        assert!(v1_0_1.is_compatible_with(v1_0_0));
        assert!(v1_1_0.is_compatible_with(v1_0_0));
        assert!(!v2_0_0.is_compatible_with(v1_0_0));
    }

    #[test]
    fn test_version_requirement() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 2, 0);
        let v3 = Version::new(2, 0, 0);

        let req = VersionRequirement::AtLeast(Version::new(1, 0, 0));
        assert!(req.satisfied_by(&v1));
        assert!(req.satisfied_by(&v2));
        assert!(req.satisfied_by(&v3));

        let req2 = VersionRequirement::AtMost(Version::new(1, 5, 0));
        assert!(req2.satisfied_by(&v1));
        assert!(req2.satisfied_by(&v2));
        assert!(!req2.satisfied_by(&v3));
    }

    #[test]
    fn test_api_version() {
        let api = APIVersion::new(
            "OmniAPI",
            Version::new(1, 0, 0),
            Version::new(1, 0, 0),
            Version::new(1, 2, 0),
        );

        assert!(api.can_support_client(Version::new(1, 0, 0)));
        assert!(api.can_support_client(Version::new(1, 1, 0)));
        assert!(api.can_support_client(Version::new(1, 2, 0)));
        assert!(!api.can_support_client(Version::new(2, 0, 0)));
    }

    #[test]
    fn test_module_dependencies() {
        let mut module = ModuleVersion::new("mymodule", Version::new(1, 0, 0));

        module.add_dependency("kernel", VersionRequirement::AtLeast(Version::new(1, 0, 0)));
        module.add_dependency("runtime", VersionRequirement::CompatibleWith(Version::new(2, 0, 0)));

        let available = vec![
            ("kernel".to_string(), Version::new(1, 2, 0)),
            ("runtime".to_string(), Version::new(2, 1, 0)),
        ];

        assert!(module.check_dependencies(&available).is_ok());
    }
}
