//! Deterministic dependency resolver (PubGrub-like algorithm)

use crate::{lockfile::{Lockfile, LockedPackage, LockedRuntime}, manifest::Manifest};
use anyhow::Result;

pub struct DependencyResolver {
    // Simple resolver for MVP
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {}
    }

    /// Deterministically resolve all dependencies to a lockfile
    pub async fn resolve(&mut self, manifest: &Manifest) -> Result<Lockfile> {
        let mut lockfile = Lockfile::new();

        // For MVP: directly add dependencies without complex resolution
        // In production: use PubGrub SAT solver

        for (name, spec) in &manifest.dependencies {
            let pkg = LockedPackage {
                name: name.clone(),
                version: spec.version.clone(),
                hash: String::new(), // Will be filled when downloading
                language: spec.language.clone(),
                dependencies: Vec::new(),
            };
            lockfile.add_package(pkg);
        }

        // Add runtimes if needed
        if !manifest.project.language.is_empty() {
            let runtime = LockedRuntime {
                language: manifest.project.language.clone(),
                version: "latest".to_string(),
                hash: String::new(),
            };
            lockfile.add_runtime(manifest.project.language.clone(), runtime);
        }

        Ok(lockfile)
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}
