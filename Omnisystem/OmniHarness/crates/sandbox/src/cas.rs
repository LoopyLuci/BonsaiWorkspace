//! Content-Addressed Storage (CAS)
//!
//! Stores all packages and artifacts by their BLAKE3 hash, enabling deduplication
//! and verification across the entire Enclave ecosystem.

use anyhow::{Result, Context};
use blake3::Hasher;
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

/// A content hash (BLAKE3)
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ContentHash(pub String);

impl ContentHash {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_dir_path(&self, base: &Path) -> PathBuf {
        // Store as base/{first 2 chars}/{remaining 62 chars} for efficient lookups
        let hash_str = &self.0;
        if hash_str.len() < 2 {
            return base.join(&hash_str);
        }
        base.join(&hash_str[0..2]).join(&hash_str[2..])
    }
}

/// Content-Addressed Store for all artifacts
#[derive(Debug, Clone)]
pub struct ContentAddressedStore {
    root: PathBuf,
}

impl ContentAddressedStore {
    pub async fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root).await?;
        Ok(Self { root })
    }

    /// Hash a file and return its content hash
    pub async fn hash_file<P: AsRef<Path>>(path: P) -> Result<ContentHash> {
        let bytes = fs::read(&path).await?;
        let hash = blake3::hash(&bytes);
        Ok(ContentHash(hash.to_hex().to_string()))
    }

    /// Hash a directory recursively
    pub async fn hash_directory<P: AsRef<Path>>(path: P) -> Result<ContentHash> {
        let mut hasher = Hasher::new();

        for entry in WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let contents = std::fs::read(entry.path())?;
            hasher.update(&contents);
        }

        let hash = hasher.finalize();
        Ok(ContentHash(hash.to_hex().to_string()))
    }

    /// Store a file in the CAS, returning its content hash
    pub async fn store_file<P: AsRef<Path>>(&self, source: P) -> Result<ContentHash> {
        let bytes = fs::read(&source).await?;
        let hash = blake3::hash(&bytes);
        let content_hash = ContentHash(hash.to_hex().to_string());

        let dest_path = content_hash.as_dir_path(&self.root);
        fs::create_dir_all(dest_path.parent().unwrap()).await?;
        fs::write(&dest_path, bytes).await?;

        Ok(content_hash)
    }

    /// Retrieve a file from the CAS by hash
    pub async fn retrieve_file(&self, hash: &ContentHash) -> Result<Vec<u8>> {
        let path = hash.as_dir_path(&self.root);
        fs::read(&path)
            .await
            .context(format!("Failed to retrieve file with hash {}", hash.0))
    }

    /// Check if a content hash exists in the CAS
    pub async fn has(&self, hash: &ContentHash) -> bool {
        hash.as_dir_path(&self.root).exists()
    }

    /// Get the path to a stored file
    pub fn get_path(&self, hash: &ContentHash) -> PathBuf {
        hash.as_dir_path(&self.root)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_file() {
        let tmpdir = tempfile::tempdir().unwrap();
        let test_file = tmpdir.path().join("test.txt");
        fs::write(&test_file, b"hello world").await.unwrap();

        let hash1 = ContentAddressedStore::hash_file(&test_file)
            .await
            .unwrap();
        let hash2 = ContentAddressedStore::hash_file(&test_file)
            .await
            .unwrap();

        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let tmpdir = tempfile::tempdir().unwrap();
        let cas = ContentAddressedStore::new(tmpdir.path().to_path_buf())
            .await
            .unwrap();

        let test_file = tmpdir.path().join("test.txt");
        fs::write(&test_file, b"test content").await.unwrap();

        let hash = cas.store_file(&test_file).await.unwrap();
        assert!(cas.has(&hash).await);

        let retrieved = cas.retrieve_file(&hash).await.unwrap();
        assert_eq!(retrieved, b"test content");
    }
}
