//! Language-specific module adapters

use crate::{ModuleId, ModuleMetadata, ModuleManagerError, Result};
use std::path::Path;

/// Language adapter trait for polymorphic module loading
pub trait LanguageAdapterTrait: Send + Sync {
    /// Get language name
    fn language(&self) -> &str;

    /// Load module metadata from filesystem
    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata>;

    /// Extract module archive
    fn extract(&self, path: &Path) -> Result<()>;

    /// Verify module checksum (robust validation)
    fn verify_checksum(&self, path: &Path, expected: &str) -> Result<()>;

    /// Validate module integrity
    fn validate(&self, metadata: &ModuleMetadata) -> Result<()>;

    /// Cleanup after unload
    fn cleanup(&self, path: &Path) -> Result<()>;
}

/// Rust module adapter
pub struct RustAdapter;

impl LanguageAdapterTrait for RustAdapter {
    fn language(&self) -> &str {
        "rust"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        let cargo_toml = path.join("Cargo.toml");
        let contents = std::fs::read_to_string(&cargo_toml)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;

        let table: toml::Table = toml::from_str(&contents)
            .map_err(|e| ModuleManagerError::InvalidModule(e.to_string()))?;

        let package = table
            .get("package")
            .ok_or_else(|| ModuleManagerError::InvalidModule("No [package] section".to_string()))?
            .as_table()
            .ok_or_else(|| ModuleManagerError::InvalidModule("package is not a table".to_string()))?;

        let name = package
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModuleManagerError::InvalidModule("Missing package name".to_string()))?;

        let version = package
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0");

        let description = package
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("No description");

        let author = package
            .get("authors")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        let checksum = self.calculate_checksum(path)?;

        Ok(ModuleMetadata {
            id: ModuleId::new("omnisystem", name, version),
            language: "rust".to_string(),
            description: description.to_string(),
            author: author.to_string(),
            license: "Apache-2.0 OR MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("src/lib.rs".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
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
        if metadata.id.name.is_empty() {
            return Err(ModuleManagerError::InvalidModule(
                "Module name cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl RustAdapter {
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

/// Python module adapter
pub struct PythonAdapter;

impl LanguageAdapterTrait for PythonAdapter {
    fn language(&self) -> &str {
        "python"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        let setup_py = path.join("setup.py");
        let setup_cfg = path.join("setup.cfg");
        let pyproject_toml = path.join("pyproject.toml");

        let (name, version) = if pyproject_toml.exists() {
            self.read_pyproject(&pyproject_toml)?
        } else if setup_cfg.exists() {
            self.read_setup_cfg(&setup_cfg)?
        } else if setup_py.exists() {
            ("python-module".to_string(), "0.0.0".to_string())
        } else {
            return Err(ModuleManagerError::InvalidModule(
                "No Python project found".to_string(),
            ));
        };

        let checksum = self.calculate_checksum(path)?;

        Ok(ModuleMetadata {
            id: ModuleId::new("omnisystem", &name, &version),
            language: "python".to_string(),
            description: "Python module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("__main__.py".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
    }

    fn extract(&self, path: &Path) -> Result<()> {
        let tar_gz = path.with_extension("tar.gz");
        if tar_gz.exists() {
            let tar_file = std::fs::File::open(&tar_gz)
                .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
            let gz = flate2::read::GzDecoder::new(tar_file);
            tar::Archive::new(gz)
                .unpack(path)
                .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
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
        if metadata.language != "python" {
            return Err(ModuleManagerError::InvalidModule(
                "Not a Python module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl PythonAdapter {
    fn read_pyproject(&self, path: &Path) -> Result<(String, String)> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let table: toml::Table = toml::from_str(&contents)
            .map_err(|e| ModuleManagerError::InvalidModule(e.to_string()))?;

        let project = table.get("project").and_then(|p| p.as_table());
        let name = project
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("python-module");
        let version = project
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0");

        Ok((name.to_string(), version.to_string()))
    }

    fn read_setup_cfg(&self, path: &Path) -> Result<(String, String)> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        Ok(("python-module".to_string(), "0.0.0".to_string()))
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

/// Go module adapter
pub struct GoAdapter;

impl LanguageAdapterTrait for GoAdapter {
    fn language(&self) -> &str {
        "go"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        let go_mod = path.join("go.mod");
        let contents = std::fs::read_to_string(&go_mod)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;

        let lines: Vec<&str> = contents.lines().collect();
        let module_line = lines
            .first()
            .ok_or_else(|| ModuleManagerError::InvalidModule("Empty go.mod".to_string()))?;
        let name = module_line.split_whitespace().nth(1).unwrap_or("unknown");

        let checksum = self.calculate_checksum(path)?;

        Ok(ModuleMetadata {
            id: ModuleId::new("omnisystem", name, "0.0.0"),
            language: "go".to_string(),
            description: "Go module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("main.go".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
    }

    fn extract(&self, path: &Path) -> Result<()> {
        let tar_gz = path.with_extension("tar.gz");
        if tar_gz.exists() {
            let tar_file = std::fs::File::open(&tar_gz)
                .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
            let gz = flate2::read::GzDecoder::new(tar_file);
            tar::Archive::new(gz)
                .unpack(path)
                .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
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
        if metadata.language != "go" {
            return Err(ModuleManagerError::InvalidModule(
                "Not a Go module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl GoAdapter {
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

/// TypeScript module adapter
pub struct TypeScriptAdapter;

impl LanguageAdapterTrait for TypeScriptAdapter {
    fn language(&self) -> &str {
        "typescript"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        let package_json = path.join("package.json");
        let contents = std::fs::read_to_string(&package_json)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;

        let obj: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|e| ModuleManagerError::InvalidModule(e.to_string()))?;

        let name = obj["name"]
            .as_str()
            .ok_or_else(|| ModuleManagerError::InvalidModule("No name in package.json".to_string()))?;

        let version = obj["version"]
            .as_str()
            .unwrap_or("0.0.0");

        let checksum = self.calculate_checksum(path)?;

        Ok(ModuleMetadata {
            id: ModuleId::new("omnisystem", name, version),
            language: "typescript".to_string(),
            description: obj["description"].as_str().unwrap_or("").to_string(),
            author: obj["author"].as_str().unwrap_or("Unknown").to_string(),
            license: obj["license"].as_str().unwrap_or("MIT").to_string(),
            dependencies: vec![],
            entry_point: Some("index.ts".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
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
        if metadata.language != "typescript" {
            return Err(ModuleManagerError::InvalidModule(
                "Not a TypeScript module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl TypeScriptAdapter {
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

/// Java module adapter
pub struct JavaAdapter;

impl LanguageAdapterTrait for JavaAdapter {
    fn language(&self) -> &str {
        "java"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        // Try pom.xml first (Maven)
        let pom_xml = path.join("pom.xml");
        let gradle_build = path.join("build.gradle");

        if pom_xml.exists() {
            self.read_maven_pom(&pom_xml)
        } else if gradle_build.exists() {
            self.read_gradle_build(&gradle_build)
        } else {
            Err(ModuleManagerError::InvalidModule(
                "No pom.xml or build.gradle found".to_string(),
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
        if metadata.language != "java" {
            return Err(ModuleManagerError::InvalidModule(
                "Not a Java module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl JavaAdapter {
    fn read_maven_pom(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        // Stub: real implementation would parse XML
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "java-module", "1.0.0", "java"),
            language: "java".to_string(),
            description: "Java module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("src/main/java/Main.java".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
    }

    fn read_gradle_build(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        // Stub: real implementation would parse Gradle DSL
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "java-module", "1.0.0", "java"),
            language: "java".to_string(),
            description: "Java module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("src/main/java/Main.java".to_string()),
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

/// Kotlin module adapter
pub struct KotlinAdapter;

impl LanguageAdapterTrait for KotlinAdapter {
    fn language(&self) -> &str {
        "kotlin"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        let gradle_kts = path.join("build.gradle.kts");
        let gradle = path.join("build.gradle");

        if gradle_kts.exists() {
            self.read_gradle_kts(&gradle_kts)
        } else if gradle.exists() {
            self.read_gradle(&gradle)
        } else {
            Err(ModuleManagerError::InvalidModule(
                "No build.gradle.kts or build.gradle found".to_string(),
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
        if metadata.language != "kotlin" {
            return Err(ModuleManagerError::InvalidModule(
                "Not a Kotlin module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl KotlinAdapter {
    fn read_gradle_kts(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "kotlin-module", "1.0.0", "kotlin"),
            language: "kotlin".to_string(),
            description: "Kotlin module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("src/main/kotlin/Main.kt".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
    }

    fn read_gradle(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "kotlin-module", "1.0.0", "kotlin"),
            language: "kotlin".to_string(),
            description: "Kotlin module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("src/main/kotlin/Main.kt".to_string()),
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

/// C/C++ module adapter
pub struct CppAdapter;

impl LanguageAdapterTrait for CppAdapter {
    fn language(&self) -> &str {
        "cpp"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        let cmake = path.join("CMakeLists.txt");
        let makefile = path.join("Makefile");

        if cmake.exists() {
            self.read_cmake(&cmake)
        } else if makefile.exists() {
            self.read_makefile(&makefile)
        } else {
            Err(ModuleManagerError::InvalidModule(
                "No CMakeLists.txt or Makefile found".to_string(),
            ))
        }
    }

    fn extract(&self, path: &Path) -> Result<()> {
        let tar_gz = path.with_extension("tar.gz");
        if tar_gz.exists() {
            let tar_file = std::fs::File::open(&tar_gz)
                .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
            let gz = flate2::read::GzDecoder::new(tar_file);
            tar::Archive::new(gz)
                .unpack(path)
                .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
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
        if metadata.language != "cpp" {
            return Err(ModuleManagerError::InvalidModule(
                "Not a C/C++ module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl CppAdapter {
    fn read_cmake(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "cpp-module", "1.0.0", "cpp"),
            language: "cpp".to_string(),
            description: "C/C++ module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("src/main.cpp".to_string()),
            exports: vec![],
            capabilities: vec![],
            checksum,
        })
    }

    fn read_makefile(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "cpp-module", "1.0.0", "cpp"),
            language: "cpp".to_string(),
            description: "C/C++ module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("main.cpp".to_string()),
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

/// C# module adapter
pub struct CsharpAdapter;

impl LanguageAdapterTrait for CsharpAdapter {
    fn language(&self) -> &str {
        "csharp"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        // Try to find any .csproj file
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "csproj") {
                    return self.read_csproj(&path);
                }
            }
        }

        Err(ModuleManagerError::InvalidModule(
            "No .csproj or .sln found".to_string(),
        ))
    }

    fn extract(&self, path: &Path) -> Result<()> {
        let dll = path.with_extension("dll");
        if dll.exists() {
            std::fs::remove_file(&dll).ok();
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
        if metadata.language != "csharp" {
            return Err(ModuleManagerError::InvalidModule(
                "Not a C# module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl CsharpAdapter {
    fn read_csproj(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "csharp-module", "1.0.0", "csharp"),
            language: "csharp".to_string(),
            description: "C# module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("Program.cs".to_string()),
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

/// Swift module adapter
pub struct SwiftAdapter;

impl LanguageAdapterTrait for SwiftAdapter {
    fn language(&self) -> &str {
        "swift"
    }

    fn load_metadata(&self, path: &Path) -> Result<ModuleMetadata> {
        let package_swift = path.join("Package.swift");

        if !package_swift.exists() {
            return Err(ModuleManagerError::InvalidModule(
                "No Package.swift found".to_string(),
            ));
        }

        self.read_package_swift(&package_swift)
    }

    fn extract(&self, path: &Path) -> Result<()> {
        let tar_gz = path.with_extension("tar.gz");
        if tar_gz.exists() {
            let tar_file = std::fs::File::open(&tar_gz)
                .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
            let gz = flate2::read::GzDecoder::new(tar_file);
            tar::Archive::new(gz)
                .unpack(path)
                .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
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
        if metadata.language != "swift" {
            return Err(ModuleManagerError::InvalidModule(
                "Not a Swift module".to_string(),
            ));
        }
        Ok(())
    }

    fn cleanup(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}

impl SwiftAdapter {
    fn read_package_swift(&self, path: &Path) -> Result<ModuleMetadata> {
        let _contents = std::fs::read_to_string(path)
            .map_err(|e| ModuleManagerError::LoadingFailed(e.to_string()))?;
        let checksum = self.calculate_checksum(path.parent().unwrap())?;
        Ok(ModuleMetadata {
            id: ModuleId::with_language("omnisystem", "swift-module", "1.0.0", "swift"),
            language: "swift".to_string(),
            description: "Swift module".to_string(),
            author: "Unknown".to_string(),
            license: "MIT".to_string(),
            dependencies: vec![],
            entry_point: Some("Sources/main.swift".to_string()),
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

/// Unified adapter factory
pub enum LanguageAdapter {
    Rust(RustAdapter),
    Python(PythonAdapter),
    Go(GoAdapter),
    TypeScript(TypeScriptAdapter),
    Java(JavaAdapter),
    Kotlin(KotlinAdapter),
    Cpp(CppAdapter),
    Csharp(CsharpAdapter),
    Swift(SwiftAdapter),
}

impl LanguageAdapter {
    pub fn for_language(lang: &str) -> Option<Box<dyn LanguageAdapterTrait>> {
        use crate::adapters_extended::{ClojureAdapter, PhpAdapter, RAdapter, RubyAdapter, ScalaAdapter};

        match lang.to_lowercase().as_str() {
            // Compiled languages
            "rust" => Some(Box::new(RustAdapter)),
            "go" | "golang" => Some(Box::new(GoAdapter)),
            "cpp" | "c++" | "cxx" | "c" => Some(Box::new(CppAdapter)),
            "csharp" | "c#" | "dotnet" => Some(Box::new(CsharpAdapter)),
            "swift" => Some(Box::new(SwiftAdapter)),
            "java" => Some(Box::new(JavaAdapter)),
            "kotlin" | "kt" => Some(Box::new(KotlinAdapter)),
            "scala" => Some(Box::new(ScalaAdapter)),

            // Interpreted languages
            "python" | "py" => Some(Box::new(PythonAdapter)),
            "ruby" | "rb" => Some(Box::new(RubyAdapter)),
            "php" => Some(Box::new(PhpAdapter)),
            "typescript" | "ts" => Some(Box::new(TypeScriptAdapter)),
            "javascript" | "js" | "node" => Some(Box::new(TypeScriptAdapter)), // Use TS adapter for JS
            "clojure" | "clj" => Some(Box::new(ClojureAdapter)),
            "r" | "r-project" => Some(Box::new(RAdapter)),

            _ => None,
        }
    }

    pub fn supported_languages() -> &'static [&'static str] {
        &[
            // Compiled languages
            "rust",
            "go", "golang",
            "cpp", "c++", "cxx", "c",
            "csharp", "c#", "dotnet",
            "swift",
            "java",
            "kotlin", "kt",
            "scala",
            // Interpreted languages
            "python", "py",
            "ruby", "rb",
            "php",
            "typescript", "ts",
            "javascript", "js", "node",
            "clojure", "clj",
            "r", "r-project",
        ]
    }

    pub fn supported_languages_display() -> String {
        "Supported languages: Rust, Python, Go, TypeScript, JavaScript, Java, Kotlin, C/C++, C#, Swift, Ruby, PHP, Scala, Clojure, R, and more".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_factory() {
        assert!(LanguageAdapter::for_language("rust").is_some());
        assert!(LanguageAdapter::for_language("python").is_some());
        assert!(LanguageAdapter::for_language("go").is_some());
        assert!(LanguageAdapter::for_language("typescript").is_some());
        assert!(LanguageAdapter::for_language("unknown").is_none());
    }
}
