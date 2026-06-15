//! Semantic version resolution

use anyhow::Result;

pub struct VersionResolver;

impl VersionResolver {
    pub fn new() -> Self {
        Self
    }

    /// Check if a version satisfies a requirement
    pub fn satisfies(&self, version: &str, requirement: &str) -> Result<bool> {
        // Simple version matching logic
        // In production, would use semver crate

        let satisfies = match requirement.chars().next() {
            Some('=') => version == requirement.trim_start_matches('='),
            Some('>') => version > requirement.trim_start_matches('>'),
            Some('<') => version < requirement.trim_start_matches('<'),
            Some('~') => {
                // Caret: ~1.2.3 means >=1.2.3 <1.3.0
                version.starts_with(&requirement.trim_start_matches('~').split('.').take(2).collect::<Vec<_>>().join("."))
            }
            Some('^') => {
                // Tilde: ^1.2.3 means >=1.2.3 <2.0.0
                version.starts_with(&requirement.trim_start_matches('^').split('.').next().unwrap_or(""))
            }
            _ => version == requirement,
        };

        Ok(satisfies)
    }
}

impl Default for VersionResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() -> Result<()> {
        let resolver = VersionResolver::new();
        assert!(resolver.satisfies("1.0.0", "1.0.0")?);
        Ok(())
    }
}
