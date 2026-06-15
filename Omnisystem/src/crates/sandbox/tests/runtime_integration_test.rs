//! Integration tests for Bonsai Enclave runtime downloader

use enclave::{
    EnclaveConfig, RuntimeManifest, RuntimeEntry, ContentAddressedStore,
};
use tempfile::TempDir;

#[tokio::test]
async fn test_runtime_manifest_parsing() {
    let manifest_toml = r#"
[[runtimes]]
name = "python"
version = "3.11.9"
platform = "x86_64-unknown-linux-gnu"
url = "https://example.com/python-3.11.9.tar.xz"
hash = "blake3:0a1b2c3d4e5f"
compressed = true
"#;

    let manifest = RuntimeManifest::from_toml(manifest_toml).unwrap();
    assert_eq!(manifest.runtimes.len(), 1);
    assert_eq!(manifest.runtimes[0].name, "python");
    assert_eq!(manifest.runtimes[0].version, "3.11.9");
    assert_eq!(manifest.runtimes[0].full_id(), "python@3.11.9");
}

#[tokio::test]
async fn test_enclave_config_creation() {
    let tmpdir = TempDir::new().unwrap();
    let config = EnclaveConfig::new(tmpdir.path().to_path_buf()).unwrap();

    assert!(config.cas_dir.exists());
    assert!(config.env_dir.exists());
    assert_eq!(config.root_dir, tmpdir.path());
}

#[tokio::test]
async fn test_cas_hash_verification() {
    let data = b"test runtime data";
    let hash_value = blake3::hash(data).to_hex().to_string();

    let entry = RuntimeEntry {
        name: "test".to_string(),
        version: "1.0".to_string(),
        platform: "x86_64".to_string(),
        url: "https://example.com/test".to_string(),
        hash: format!("blake3:{}", hash_value),
        signature: String::new(),
        compressed: false,
        build_script: None,
    };

    assert!(entry.verify_hash(data).unwrap());
    assert!(!entry.verify_hash(b"wrong data").unwrap());
}

#[tokio::test]
async fn test_runtime_full_id() {
    let entry = RuntimeEntry {
        name: "python".to_string(),
        version: "3.11.9".to_string(),
        platform: "x86_64-unknown-linux-gnu".to_string(),
        url: "https://example.com".to_string(),
        hash: "blake3:abc123".to_string(),
        signature: String::new(),
        compressed: false,
        build_script: None,
    };

    assert_eq!(entry.full_id(), "python@3.11.9");
}

#[tokio::test]
async fn test_find_runtime_in_manifest() {
    let manifest_toml = r#"
[[runtimes]]
name = "python"
version = "3.11.9"
platform = "x86_64-unknown-linux-gnu"
url = "https://example.com/python-3.11.9.tar.xz"
hash = "blake3:abc123"
compressed = true

[[runtimes]]
name = "python"
version = "3.12.0"
platform = "x86_64-unknown-linux-gnu"
url = "https://example.com/python-3.12.0.tar.xz"
hash = "blake3:def456"
compressed = true

[[runtimes]]
name = "node"
version = "20.12.2"
platform = "x86_64-unknown-linux-gnu"
url = "https://example.com/node-20.12.2.tar.xz"
hash = "blake3:ghi789"
compressed = true
"#;

    let manifest = RuntimeManifest::from_toml(manifest_toml).unwrap();

    let py311 = manifest.find("python", "3.11.9").unwrap();
    assert_eq!(py311.full_id(), "python@3.11.9");

    let py312 = manifest.find("python", "3.12.0").unwrap();
    assert_eq!(py312.version, "3.12.0");

    let node = manifest.find("node", "20.12.2").unwrap();
    assert_eq!(node.name, "node");

    let missing = manifest.find("go", "1.22");
    assert!(missing.is_none());
}

#[tokio::test]
async fn test_all_runtimes_for_language() {
    let manifest_toml = r#"
[[runtimes]]
name = "python"
version = "3.11.9"
platform = "x86_64-unknown-linux-gnu"
url = "https://example.com/python-3.11.9.tar.xz"
hash = "blake3:abc123"
compressed = true

[[runtimes]]
name = "python"
version = "3.12.0"
platform = "x86_64-unknown-linux-gnu"
url = "https://example.com/python-3.12.0.tar.xz"
hash = "blake3:def456"
compressed = true

[[runtimes]]
name = "node"
version = "20.12.2"
platform = "x86_64-unknown-linux-gnu"
url = "https://example.com/node-20.12.2.tar.xz"
hash = "blake3:ghi789"
compressed = true
"#;

    let manifest = RuntimeManifest::from_toml(manifest_toml).unwrap();
    let python_runtimes = manifest.all_for_language("python");
    assert_eq!(python_runtimes.len(), 2);
    assert_eq!(python_runtimes[0].version, "3.11.9");
    assert_eq!(python_runtimes[1].version, "3.12.0");

    let node_runtimes = manifest.all_for_language("node");
    assert_eq!(node_runtimes.len(), 1);
}

#[tokio::test]
async fn test_content_addressed_storage() {
    let tmpdir = TempDir::new().unwrap();
    let cas = ContentAddressedStore::new(tmpdir.path().to_path_buf())
        .await
        .unwrap();

    let data = b"test data";
    let hash = enclave::cas::ContentHash(
        blake3::hash(data).to_hex().to_string(),
    );

    assert!(!cas.has(&hash).await);
}
