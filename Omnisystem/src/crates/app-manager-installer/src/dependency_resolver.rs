use crate::{Result, InstallerError};
use app_manager_core::{AppId, Version, ModuleResolver, DependencyGraph};

pub struct DependencyResolver;

impl DependencyResolver {
    pub fn resolve(
        app_id: &AppId,
        graph: &DependencyGraph,
        resolver: &ModuleResolver,
    ) -> Result<Vec<(AppId, Version)>> {
        let dependencies = graph
            .get_module(app_id)
            .map_err(|_| InstallerError::DependencyResolutionFailed("Module not found".to_string()))?
            .dependencies;

        let mut resolved = Vec::new();

        for dep in dependencies {
            if !dep.optional {
                let version = resolver
                    .find_compatible_version(&dep.app_id, &dep.version_constraint)
                    .map_err(|_| {
                        InstallerError::DependencyResolutionFailed(format!(
                            "Cannot resolve {} {:?}",
                            dep.app_id, dep.version_constraint
                        ))
                    })?;

                resolved.push((dep.app_id.clone(), version));
            }
        }

        Ok(resolved)
    }

    pub fn resolve_transitive(
        app_id: &AppId,
        graph: &DependencyGraph,
        resolver: &ModuleResolver,
    ) -> Result<Vec<(AppId, Version)>> {
        let mut resolved = Vec::new();
        let mut visited = std::collections::HashSet::new();

        Self::resolve_recursive(app_id, graph, resolver, &mut resolved, &mut visited)?;

        Ok(resolved)
    }

    fn resolve_recursive(
        app_id: &AppId,
        graph: &DependencyGraph,
        resolver: &ModuleResolver,
        resolved: &mut Vec<(AppId, Version)>,
        visited: &mut std::collections::HashSet<AppId>,
    ) -> Result<()> {
        if visited.contains(app_id) {
            return Ok(());
        }

        visited.insert(app_id.clone());

        let module = graph
            .get_module(app_id)
            .map_err(|_| InstallerError::DependencyResolutionFailed("Module not found".to_string()))?;

        for dep in &module.dependencies {
            if !dep.optional {
                let version = resolver
                    .find_compatible_version(&dep.app_id, &dep.version_constraint)
                    .map_err(|_| {
                        InstallerError::DependencyResolutionFailed(format!(
                            "Cannot resolve {} {:?}",
                            dep.app_id, dep.version_constraint
                        ))
                    })?;

                resolved.push((dep.app_id.clone(), version.clone()));

                Self::resolve_recursive(&dep.app_id, graph, resolver, resolved, visited)?;
            }
        }

        Ok(())
    }

    pub fn validate_dependencies(
        _app_id: &AppId,
        dependencies: &[(AppId, Version)],
        graph: &DependencyGraph,
    ) -> Result<()> {
        let conflicts = graph
            .list_all_modules()
            .map_err(|_| InstallerError::DependencyResolutionFailed("Failed to list modules".to_string()))?
            .iter()
            .filter_map(|m| {
                if !m.conflicts.is_empty() {
                    Some(m.conflicts.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<_>>();

        for conflict in conflicts {
            for (dep_id, _) in dependencies {
                if conflict.conflicting_app_id == *dep_id {
                    return Err(InstallerError::DependencyResolutionFailed(format!(
                        "Conflict detected: {}",
                        conflict.reason
                    )));
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
    fn test_dependency_resolver() {
        let app_id = AppId::new("test-app").unwrap();
        assert!(!app_id.as_str().is_empty());
    }
}
