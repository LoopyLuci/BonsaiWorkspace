//! Omnisystem Compiler Module v1.0.0
//!
//! Universal multi-language compiler with:
//! - Multi-language support (Rust, C/C++, Go, Zig, Titan, Python, TypeScript, JavaScript, Java, Kotlin, C#, Objective-C, Swift, D, Haskell)
//! - Distributed compilation (Phase 2B)
//! - Advanced caching (Phase 2C - Blake3 CAS)
//! - IDE integration (Phase 2D - VSCode, JetBrains)
//! - Production hardening (Phase 2E - comprehensive testing)

pub mod module;

pub use module::{CompilerModule, CompilerConfig, CompilerCore};
pub use omnisystem_core::{Error, Result};

/// Create a new compiler module with default configuration
pub fn create_module() -> Result<CompilerModule> {
    CompilerModule::new(CompilerConfig::default())
}

/// Create a new compiler module with custom configuration
pub fn create_module_with_config(config: CompilerConfig) -> Result<CompilerModule> {
    CompilerModule::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_module() {
        let module = create_module().unwrap();
        assert_eq!(module.name(), "omnisystem-compiler");
    }

    #[test]
    fn test_create_module_with_config() {
        let config = CompilerConfig {
            enabled_languages: vec!["rust".to_string()],
            ..Default::default()
        };
        let module = create_module_with_config(config).unwrap();
        assert_eq!(module.config().enabled_languages, vec!["rust"]);
    }
}
