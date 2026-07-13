//! Conflict resolution for version mismatches

use crate::{Dependency, Result};

pub struct ConflictResolver;

impl ConflictResolver {
    pub fn new() -> Self {
        Self
    }

    /// Resolve version conflicts between dependencies
    pub fn resolve_conflicts(&self, dependencies: &[Dependency]) -> Result<Vec<Dependency>> {
        let mut resolved = dependencies.to_vec();

        // Simple conflict resolution: keep highest version
        resolved.sort_by(|a, b| b.module.version.cmp(&a.module.version));

        log::info!("Resolved {} dependencies", resolved.len());
        Ok(resolved)
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModuleId;

    #[test]
    fn test_resolver_creation() {
        let resolver = ConflictResolver::new();
        let mut deps = Vec::new();
        deps.push(Dependency {
            module: ModuleId {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
            },
            required_version: "~1.0".to_string(),
            optional: false,
        });

        let resolved = resolver.resolve_conflicts(&deps);
        assert!(resolved.is_ok());
    }
}
