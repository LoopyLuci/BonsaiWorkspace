use crate::{ModuleVersion, SubModuleError, Result};

pub struct VersionResolver;

impl VersionResolver {
    pub fn is_compatible(
        required: &ModuleVersion,
        available: &ModuleVersion,
    ) -> Result<()> {
        if available.major != required.major {
            return Err(SubModuleError::VersionMismatch {
                required: required.to_string(),
                actual: available.to_string(),
            });
        }

        if available.minor < required.minor {
            return Err(SubModuleError::VersionMismatch {
                required: required.to_string(),
                actual: available.to_string(),
            });
        }

        Ok(())
    }

    pub fn resolve_latest(versions: &[ModuleVersion]) -> Option<ModuleVersion> {
        versions
            .iter()
            .max_by(|a, b| {
                (a.major, a.minor, a.patch)
                    .cmp(&(b.major, b.minor, b.patch))
            })
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compatible_versions() {
        let req = ModuleVersion::new(1, 2, 0);
        let avail = ModuleVersion::new(1, 3, 0);
        assert!(VersionResolver::is_compatible(&req, &avail).is_ok());
    }

    #[test]
    fn test_incompatible_major() {
        let req = ModuleVersion::new(1, 0, 0);
        let avail = ModuleVersion::new(2, 0, 0);
        assert!(VersionResolver::is_compatible(&req, &avail).is_err());
    }

    #[test]
    fn test_resolve_latest() {
        let versions = vec![
            ModuleVersion::new(1, 0, 0),
            ModuleVersion::new(1, 2, 0),
            ModuleVersion::new(1, 1, 5),
        ];
        let latest = VersionResolver::resolve_latest(&versions);
        assert_eq!(latest, Some(ModuleVersion::new(1, 2, 0)));
    }
}
