//! Software Bill of Materials generation. `generate` reads a real
//! `Cargo.toml` at the given path and lists its declared dependencies as
//! SBOM components (a lightweight, real dependency inventory rather than a
//! full `cargo metadata` resolution).

use crate::{Result, SecurityError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sbom {
    pub bom_version: u32,
    pub spec_version: String,
    pub version: u32,
    pub components: Vec<Component>,
    pub services: Vec<Service>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub bom_ref: String,
    pub component_type: String,
    pub name: String,
    pub version: String,
    pub purl: String,
    pub hashes: Vec<Hash>,
    pub licenses: Vec<String>,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hash {
    pub alg: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub bom_ref: String,
    pub name: String,
    pub version: String,
    pub endpoints: Vec<String>,
}

pub struct SbomGenerator;

impl SbomGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate an SBOM by reading and parsing the `Cargo.toml` at `path`
    /// (either the manifest itself, or a directory containing one).
    pub async fn generate(&self, path: &str) -> Result<Sbom> {
        let manifest_path = if path.ends_with(".toml") {
            path.to_string()
        } else {
            format!("{}/Cargo.toml", path.trim_end_matches('/'))
        };

        let content = tokio::fs::read_to_string(&manifest_path)
            .await
            .map_err(|e| SecurityError::SbomGenerationFailed(format!("reading {}: {}", manifest_path, e)))?;

        let manifest: toml::Value = content
            .parse::<toml::Value>()
            .map_err(|e| SecurityError::SbomGenerationFailed(format!("parsing {}: {}", manifest_path, e)))?;

        let package_name = manifest
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();

        let mut components = Vec::new();
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(deps) = manifest.get(section).and_then(|d| d.as_table()) {
                for (name, spec) in deps {
                    components.push(dependency_to_component(name, spec, section));
                }
            }
        }

        Ok(Sbom {
            bom_version: 1,
            spec_version: "1.4".to_string(),
            version: 1,
            components,
            services: vec![Service {
                bom_ref: format!("service:{}", package_name),
                name: package_name,
                version: manifest
                    .get("package")
                    .and_then(|p| p.get("version"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.0.0")
                    .to_string(),
                endpoints: vec![],
            }],
        })
    }

    pub fn export_cyclonedx(&self, sbom: &Sbom) -> Result<String> {
        serde_json::to_string_pretty(sbom)
            .map_err(|e| SecurityError::SbomGenerationFailed(e.to_string()))
    }
}

impl Default for SbomGenerator {
    fn default() -> Self {
        Self::new()
    }
}

fn dependency_to_component(name: &str, spec: &toml::Value, scope: &str) -> Component {
    let version = match spec {
        toml::Value::String(v) => v.clone(),
        toml::Value::Table(t) => t
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("*")
            .to_string(),
        _ => "*".to_string(),
    };

    Component {
        bom_ref: format!("pkg:cargo/{}@{}", name, version),
        component_type: "library".to_string(),
        name: name.to_string(),
        version: version.clone(),
        purl: format!("pkg:cargo/{}@{}", name, version),
        hashes: vec![],
        licenses: vec![],
        scope: scope.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    async fn write_temp_manifest(content: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    #[tokio::test]
    async fn test_generate_reads_dependencies() {
        let manifest = r#"
[package]
name = "example"
version = "0.3.1"

[dependencies]
serde = "1.0"
tokio = { version = "1.35", features = ["full"] }
"#;
        let file = write_temp_manifest(manifest).await;
        let generator = SbomGenerator::new();
        let sbom = generator.generate(file.path().to_str().unwrap()).await.unwrap();

        assert_eq!(sbom.components.len(), 2);
        assert!(sbom.components.iter().any(|c| c.name == "serde" && c.version == "1.0"));
        assert!(sbom.components.iter().any(|c| c.name == "tokio" && c.version == "1.35"));
        assert_eq!(sbom.services[0].name, "example");
        assert_eq!(sbom.services[0].version, "0.3.1");
    }

    #[tokio::test]
    async fn test_generate_missing_manifest_errors() {
        let generator = SbomGenerator::new();
        let result = generator.generate("/nonexistent/path/Cargo.toml").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_export_cyclonedx_round_trips() {
        let generator = SbomGenerator::new();
        let sbom = Sbom {
            bom_version: 1,
            spec_version: "1.4".to_string(),
            version: 1,
            components: vec![],
            services: vec![],
        };
        let json = generator.export_cyclonedx(&sbom).unwrap();
        assert!(json.contains("\"specVersion\"") || json.contains("spec_version"));
    }
}
