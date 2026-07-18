//! Resolves which concrete version of a module satisfies a version constraint.
//!
//! Companion to [`crate::dependency_graph::DependencyGraph`]: the graph tracks *what*
//! depends on *what*, while `ModuleResolver` tracks *which versions are actually
//! available* for each module so a concrete version can be picked.

use crate::error::{AppManagerError, AppManagerResult as Result};
use crate::types::{AppId, Version, VersionConstraint};
use dashmap::DashMap;
use std::sync::Arc;

pub struct ModuleResolver {
    available_versions: Arc<DashMap<AppId, Vec<Version>>>,
}

impl ModuleResolver {
    pub fn new() -> Self {
        ModuleResolver {
            available_versions: Arc::new(DashMap::new()),
        }
    }

    pub fn register_version(&self, app_id: AppId, version: Version) {
        let mut entry = self.available_versions.entry(app_id).or_default();
        if !entry.contains(&version) {
            entry.push(version);
        }
    }

    pub fn register_versions(&self, app_id: AppId, versions: impl IntoIterator<Item = Version>) {
        for version in versions {
            self.register_version(app_id.clone(), version);
        }
    }

    pub fn available_versions(&self, app_id: &AppId) -> Vec<Version> {
        self.available_versions
            .get(app_id)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Finds the highest available version of `app_id` that satisfies `constraint`.
    pub fn find_compatible_version(
        &self,
        app_id: &AppId,
        constraint: &VersionConstraint,
    ) -> Result<Version> {
        let versions = self
            .available_versions
            .get(app_id)
            .ok_or_else(|| AppManagerError::AppNotFound(app_id.to_string()))?;

        versions
            .iter()
            .filter(|v| constraint.satisfies(v))
            .max()
            .cloned()
            .ok_or_else(|| {
                AppManagerError::InvalidVersion(format!(
                    "no version of {} satisfies constraint {:?}",
                    app_id, constraint
                ))
            })
    }

    pub fn remove_module(&self, app_id: &AppId) {
        self.available_versions.remove(app_id);
    }

    pub fn module_count(&self) -> usize {
        self.available_versions.len()
    }
}

impl Default for ModuleResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_list_versions() {
        let resolver = ModuleResolver::new();
        let app_id = AppId::new("foo").unwrap();

        resolver.register_version(app_id.clone(), Version::new(1, 0, 0));
        resolver.register_version(app_id.clone(), Version::new(1, 2, 0));
        resolver.register_version(app_id.clone(), Version::new(1, 0, 0));

        assert_eq!(resolver.available_versions(&app_id).len(), 2);
    }

    #[test]
    fn test_find_compatible_version_picks_highest_match() {
        let resolver = ModuleResolver::new();
        let app_id = AppId::new("foo").unwrap();

        resolver.register_versions(
            app_id.clone(),
            [Version::new(1, 0, 0), Version::new(1, 2, 0), Version::new(2, 0, 0)],
        );

        let resolved = resolver
            .find_compatible_version(&app_id, &VersionConstraint::Compatible(Version::new(1, 0, 0)))
            .unwrap();

        assert_eq!(resolved, Version::new(1, 2, 0));
    }

    #[test]
    fn test_find_compatible_version_at_least() {
        let resolver = ModuleResolver::new();
        let app_id = AppId::new("foo").unwrap();

        resolver.register_versions(
            app_id.clone(),
            [Version::new(1, 0, 0), Version::new(1, 5, 0), Version::new(2, 0, 0)],
        );

        let resolved = resolver
            .find_compatible_version(&app_id, &VersionConstraint::AtLeast(Version::new(1, 5, 0)))
            .unwrap();

        assert_eq!(resolved, Version::new(2, 0, 0));
    }

    #[test]
    fn test_find_compatible_version_no_match() {
        let resolver = ModuleResolver::new();
        let app_id = AppId::new("foo").unwrap();

        resolver.register_version(app_id.clone(), Version::new(1, 0, 0));

        let result = resolver
            .find_compatible_version(&app_id, &VersionConstraint::AtLeast(Version::new(2, 0, 0)));

        assert!(result.is_err());
    }

    #[test]
    fn test_find_compatible_version_unknown_module() {
        let resolver = ModuleResolver::new();
        let app_id = AppId::new("unknown").unwrap();

        let result = resolver
            .find_compatible_version(&app_id, &VersionConstraint::AtLeast(Version::new(1, 0, 0)));

        assert!(result.is_err());
    }

    #[test]
    fn test_remove_module() {
        let resolver = ModuleResolver::new();
        let app_id = AppId::new("foo").unwrap();

        resolver.register_version(app_id.clone(), Version::new(1, 0, 0));
        assert_eq!(resolver.module_count(), 1);

        resolver.remove_module(&app_id);
        assert_eq!(resolver.module_count(), 0);
    }
}
