//! Automatic project detection and analysis

use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectType {
    Rust,
    Python,
    Go,
    Java,
    Kotlin,
    TypeScript,
    JavaScript,
    CSharp,
    Swift,
    Ruby,
    Php,
    Cpp,
    Scala,
    Clojure,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub project_type: ProjectType,
    pub root_path: PathBuf,
    pub name: String,
    pub version: String,
    pub languages: Vec<ProjectType>,
    pub dependencies: Vec<String>,
    pub entry_points: Vec<PathBuf>,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

/// Automatic project detector
pub struct ProjectDetector;

impl ProjectDetector {
    /// Auto-detect project type and configuration
    pub fn detect(path: &PathBuf) -> Result<ProjectInfo> {
        let mut languages = Vec::new();
        let mut project_type = ProjectType::Unknown;

        // Check for Rust
        if path.join("Cargo.toml").exists() {
            languages.push(ProjectType::Rust);
            project_type = ProjectType::Rust;
        }

        // Check for Python
        if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
            languages.push(ProjectType::Python);
            if project_type == ProjectType::Unknown {
                project_type = ProjectType::Python;
            }
        }

        // Check for Go
        if path.join("go.mod").exists() {
            languages.push(ProjectType::Go);
            if project_type == ProjectType::Unknown {
                project_type = ProjectType::Go;
            }
        }

        // Check for TypeScript/JavaScript
        if path.join("package.json").exists() {
            if path.join("tsconfig.json").exists() {
                languages.push(ProjectType::TypeScript);
                if project_type == ProjectType::Unknown {
                    project_type = ProjectType::TypeScript;
                }
            } else {
                languages.push(ProjectType::JavaScript);
                if project_type == ProjectType::Unknown {
                    project_type = ProjectType::JavaScript;
                }
            }
        }

        // Check for Java/Kotlin
        if path.join("pom.xml").exists() {
            languages.push(ProjectType::Java);
            if project_type == ProjectType::Unknown {
                project_type = ProjectType::Java;
            }
        }

        if path.join("build.gradle.kts").exists() || path.join("build.gradle").exists() {
            languages.push(ProjectType::Kotlin);
            if project_type == ProjectType::Unknown {
                project_type = ProjectType::Kotlin;
            }
        }

        // Check for C#
        let has_csproj = std::fs::read_dir(path)
            .ok()
            .and_then(|entries| {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".csproj") {
                            return Some(true);
                        }
                    }
                }
                None
            })
            .is_some();

        if has_csproj {
            languages.push(ProjectType::CSharp);
            if project_type == ProjectType::Unknown {
                project_type = ProjectType::CSharp;
            }
        }

        // Check for Swift
        if path.join("Package.swift").exists() {
            languages.push(ProjectType::Swift);
            if project_type == ProjectType::Unknown {
                project_type = ProjectType::Swift;
            }
        }

        // Check for C/C++
        if path.join("CMakeLists.txt").exists() || path.join("Makefile").exists() {
            languages.push(ProjectType::Cpp);
            if project_type == ProjectType::Unknown {
                project_type = ProjectType::Cpp;
            }
        }

        // Multi-language detection
        if languages.len() > 1 {
            project_type = ProjectType::Mixed;
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown-project")
            .to_string();

        Ok(ProjectInfo {
            project_type,
            root_path: path.clone(),
            name,
            version: "0.0.0".to_string(),
            languages,
            dependencies: Vec::new(),
            entry_points: Vec::new(),
            detected_at: chrono::Utc::now(),
        })
    }

    /// Recursively detect all projects in directory tree
    pub fn detect_all(root_path: &PathBuf) -> Result<Vec<ProjectInfo>> {
        let mut projects = Vec::new();

        // Check current directory
        if let Ok(info) = Self::detect(root_path) {
            if info.project_type != ProjectType::Unknown {
                projects.push(info);
            }
        }

        // Recursively check subdirectories
        if let Ok(entries) = std::fs::read_dir(root_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(mut subprojects) = Self::detect_all(&path) {
                        projects.append(&mut subprojects);
                    }
                }
            }
        }

        Ok(projects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_detection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Create Cargo.toml to simulate Rust project
        std::fs::write(path.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let info = ProjectDetector::detect(&path).unwrap();
        assert_eq!(info.project_type, ProjectType::Rust);
    }

    #[test]
    fn test_multi_language_detection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Create multiple project files
        std::fs::write(path.join("Cargo.toml"), "[package]").unwrap();
        std::fs::write(path.join("package.json"), "{}").unwrap();

        let info = ProjectDetector::detect(&path).unwrap();
        assert_eq!(info.project_type, ProjectType::Mixed);
        assert!(info.languages.contains(&ProjectType::Rust));
        assert!(info.languages.contains(&ProjectType::JavaScript));
    }
}
