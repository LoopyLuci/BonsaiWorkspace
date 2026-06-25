//! Extended language adapters for additional programming languages

use crate::{ModuleId, ModuleMetadata, ModuleManagerError, Result};
use std::path::Path;

use super::LanguageAdapterTrait;

/// Ruby module adapter
pub struct RubyAdapter;

impl LanguageAdapterTrait for RubyAdapter {
    fn language(&self) -> &str {
        "ruby"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        let gemfile = path.join("Gemfile");

        if gemfile.exists() {
            self.read_gemfile(&gemfile)
        } else if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.extension().map_or(false, |ext| ext == "gemspec") {
                    return self.read_gemspec(&entry_path);
                }
            }
            Err(ModuleManagerError::InvalidModule(
                "No Gemfile or .gemspec found".to_string(),
            ))
        } else {
            Err(ModuleManagerError::InvalidModule("Invalid path".to_string()))
        }
    }

    fn extract(&self, _path: &Path) -> Result<()> {
        Ok(())
    }

    fn verify_checksum(&self, path: &Path, expected: &str) -> Result<()> {
        let actual = self.calculate_checksum(path)?;
        if actual != expected {
            return Err(ModuleManagerError::InvalidModule(
                format!("Checksum mismatch: {} != {}", actual, expected),
            ));
        }
        Ok(())
    }

    fn validate(&self, metadata: &ModuleMetadata) -> Result<()> {
        if metadata.language != "ruby" {
            return Err(ModuleManagerError::InvalidModule(
                "Not a Ruby module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl RubyAdapter {
    fn read_gemfile(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "ruby-module", "1.0.0", "ruby"),
            language: "ruby".to_string(),
            description: "Ruby module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("lib/main.rb".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
    }

    fn read_gemspec(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "ruby-module", "1.0.0", "ruby"),
            language: "ruby".to_string(),
            description: "Ruby module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("lib/main.rb".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
    }

    fn calculate_checksum(&self, path: &Path) -> Result<String> {
        use blake3::Hasher;
        use std::fs;

        let mut hasher = Hasher::new();

        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if let Ok(contents) = fs::read(entry.path()) {
                        hasher.update(&contents);
                    }
                }
            }
        }

        Ok(hasher.finalize().to_hex().to_string())
    }
}

/// PHP module adapter
pub struct PhpAdapter;

impl LanguageAdapterTrait for PhpAdapter {
    fn language(&self) -> &str {
        "php"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        let composer_json = path.join("composer.json");

        if !composer_json.exists() {
            return Err(ModuleManagerError::InvalidModule(
                "No composer.json found".to_string(),
            ));
        }

        self.read_composer(&composer_json)
    }

    fn extract(&self, _path: &Path) -> Result<()> {
        Ok(())
    }

    fn verify_checksum(&self, path: &Path, expected: &str) -> Result<()> {
        let actual = self.calculate_checksum(path)?;
        if actual != expected {
            return Err(ModuleManagerError::InvalidModule(
                format!("Checksum mismatch: {} != {}", actual, expected),
            ));
        }
        Ok(())
    }

    fn validate(&self, metadata: &ModuleMetadata) -> Result<()> {
        if metadata.language != "php" {
            return Err(ModuleManagerError::InvalidModule(
                "Not a PHP module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl PhpAdapter {
    fn read_composer(&self, path: &Path) -> Result<ModuleMetadata> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;

        let obj: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|e| ModuleManagerError::InvalidModule(e.to_string()))?;

        let name = obj["name"]
            .as_str()
            .ok_or_else(|| ModuleManagerError::InvalidModule("No name in composer.json".to_string()))?;

        let version = obj["version"]
            .as_str()
            .unwrap_or("0.0.0");

        let checksum = self.calculate_checksum(path.parent().unwrap())?;

        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", name, version, "php"),
            language: "php".to_string(),
            description: obj["description"].as_str().unwrap_or("").to_string(),
            author: obj["authors"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|a| a.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            license: obj["license"].as_str().unwrap_or("MIT").to_string(),
            dependencies: vec![],
            entry_point: Some("index.php".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
    }

    fn calculate_checksum(&self, path: &Path) -> Result<String> {
        use blake3::Hasher;
        use std::fs;

        let mut hasher = Hasher::new();

        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if let Ok(contents) = fs::read(entry.path()) {
                        hasher.update(&contents);
                    }
                }
            }
        }

        Ok(hasher.finalize().to_hex().to_string())
    }
}

/// Scala module adapter
pub struct ScalaAdapter;

impl LanguageAdapterTrait for ScalaAdapter {
    fn language(&self) -> &str {
        "scala"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        let build_sbt = path.join("build.sbt");

        if !build_sbt.exists() {
            return Err(ModuleManagerError::InvalidModule(
                "No build.sbt found".to_string(),
            ));
        }

        self.read_sbt(&build_sbt)
    }

    fn extract(&self, path: &Path) -> Result<()> {
        let jar = path.with_extension("jar");
        if jar.exists() {
            std::fs::remove_file(&jar).ok();
        }
        Ok(())
    }

    fn verify_checksum(&self, path: &Path, expected: &str) -> Result<()> {
        let actual = self.calculate_checksum(path)?;
        if actual != expected {
            return Err(ModuleManagerError::InvalidModule(
                format!("Checksum mismatch: {} != {}", actual, expected),
            ));
        }
        Ok(())
    }

    fn validate(&self, metadata: &ModuleMetadata) -> Result<()> {
        if metadata.language != "scala" {
            return Err(ModuleManagerError::InvalidModule(
                "Not a Scala module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl ScalaAdapter {
    fn read_sbt(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "scala-module", "1.0.0", "scala"),
            language: "scala".to_string(),
            description: "Scala module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("src/main/scala/Main.scala".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
    }

    fn calculate_checksum(&self, path: &Path) -> Result<String> {
        use blake3::Hasher;
        use std::fs;

        let mut hasher = Hasher::new();

        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if let Ok(contents) = fs::read(entry.path()) {
                        hasher.update(&contents);
                    }
                }
            }
        }

        Ok(hasher.finalize().to_hex().to_string())
    }
}

/// R module adapter
pub struct RAdapter;

impl LanguageAdapterTrait for RAdapter {
    fn language(&self) -> &str {
        "r"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        let description = path.join("DESCRIPTION");

        if !description.exists() {
            return Err(ModuleManagerError::InvalidModule(
                "No DESCRIPTION file found".to_string(),
            ));
        }

        self.read_description(&description)
    }

    fn extract(&self, _path: &Path) -> Result<()> {
        Ok(())
    }

    fn verify_checksum(&self, path: &Path, expected: &str) -> Result<()> {
        let actual = self.calculate_checksum(path)?;
        if actual != expected {
            return Err(ModuleManagerError::InvalidModule(
                format!("Checksum mismatch: {} != {}", actual, expected),
            ));
        }
        Ok(())
    }

    fn validate(&self, metadata: &ModuleMetadata) -> Result<()> {
        if metadata.language != "r" {
            return Err(ModuleManagerError::InvalidModule(
                "Not an R module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl RAdapter {
    fn read_description(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "r-module", "1.0.0", "r"),
            language: "r".to_string(),
            description: "R module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("R/main.R".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
    }

    fn calculate_checksum(&self, path: &Path) -> Result<String> {
        use blake3::Hasher;
        use std::fs;

        let mut hasher = Hasher::new();

        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if let Ok(contents) = fs::read(entry.path()) {
                        hasher.update(&contents);
                    }
                }
            }
        }

        Ok(hasher.finalize().to_hex().to_string())
    }
}

/// Clojure module adapter
pub struct ClojureAdapter;

impl LanguageAdapterTrait for ClojureAdapter {
    fn language(&self) -> &str {
        "clojure"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        let project_clj = path.join("project.clj");
        let deps_edn = path.join("deps.edn");

        if project_clj.exists() {
            self.read_project_clj(&project_clj)
        } else if deps_edn.exists() {
            self.read_deps_edn(&deps_edn)
        } else {
            Err(ModuleManagerError::InvalidModule(
                "No project.clj or deps.edn found".to_string(),
            ))
        }
    }

    fn extract(&self, path: &Path) -> Result<()> {
        let jar = path.with_extension("jar");
        if jar.exists() {
            std::fs::remove_file(&jar).ok();
        }
        Ok(())
    }

    fn verify_checksum(&self, path: &Path, expected: &str) -> Result<()> {
        let actual = self.calculate_checksum(path)?;
        if actual != expected {
            return Err(ModuleManagerError::InvalidModule(
                format!("Checksum mismatch: {} != {}", actual, expected),
            ));
        }
        Ok(())
    }

    fn validate(&self, metadata: &ModuleMetadata) -> Result<()> {
        if metadata.language != "clojure" {
            return Err(ModuleManagerError::InvalidModule(
                "Not a Clojure module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl ClojureAdapter {
    fn read_project_clj(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "clojure-module", "1.0.0", "clojure"),
            language: "clojure".to_string(),
            description: "Clojure module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("src/main.clj".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
    }

    fn read_deps_edn(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "clojure-module", "1.0.0", "clojure"),
            language: "clojure".to_string(),
            description: "Clojure module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("src/main.clj".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
    }

    fn calculate_checksum(&self, path: &Path) -> Result<String> {
        use blake3::Hasher;
        use std::fs;

        let mut hasher = Hasher::new();

        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if let Ok(contents) = fs::read(entry.path()) {
                        hasher.update(&contents);
                    }
                }
            }
        }

        Ok(hasher.finalize().to_hex().to_string())
    }
}
