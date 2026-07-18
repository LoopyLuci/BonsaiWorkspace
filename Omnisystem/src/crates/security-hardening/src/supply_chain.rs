//! Supply-chain artifact verification. `verify_artifact` performs a real
//! check against the filesystem (the artifact must exist and be non-empty)
//! and computes its SHA-256 digest rather than unconditionally reporting
//! success. `verify_provenance` checks that the recorded provenance
//! signature is non-empty and internally consistent (a real, if minimal,
//! well-formedness check — full cryptographic signature verification would
//! require a trusted public key, which this crate does not manage).

use crate::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub artifact: String,
    pub builder: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub signature: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactVerification {
    pub path: String,
    pub exists: bool,
    pub size_bytes: u64,
    pub sha256: String,
    pub verified: bool,
}

pub struct SupplyChainVerifier;

impl SupplyChainVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Verify that `path` exists, is non-empty, and compute its SHA-256
    /// digest. Returns `true` only if the artifact is present and readable.
    pub async fn verify_artifact(&self, path: &str) -> Result<bool> {
        Ok(self.inspect_artifact(path).await?.verified)
    }

    /// Same check as `verify_artifact`, but returns the full inspection
    /// result (digest, size) rather than collapsing it to a bool.
    pub async fn inspect_artifact(&self, path: &str) -> Result<ArtifactVerification> {
        let bytes = match tokio::fs::read(path).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return Ok(ArtifactVerification {
                    path: path.to_string(),
                    exists: false,
                    size_bytes: 0,
                    sha256: String::new(),
                    verified: false,
                });
            }
        };

        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let digest = hasher.finalize();
        let sha256 = digest.iter().map(|b| format!("{:02x}", b)).collect::<String>();

        Ok(ArtifactVerification {
            path: path.to_string(),
            exists: true,
            size_bytes: bytes.len() as u64,
            sha256,
            verified: !bytes.is_empty(),
        })
    }

    /// Minimal well-formedness check on a provenance record: the signature
    /// and builder fields must be non-empty, and the artifact name must be
    /// non-empty. This does not cryptographically verify the signature.
    pub async fn verify_provenance(&self, provenance: &Provenance) -> Result<bool> {
        Ok(!provenance.signature.trim().is_empty()
            && !provenance.builder.trim().is_empty()
            && !provenance.artifact.trim().is_empty())
    }
}

impl Default for SupplyChainVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_verify_artifact_missing_file_fails() {
        let verifier = SupplyChainVerifier::new();
        let result = verifier.verify_artifact("/nonexistent/artifact.bin").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_verify_artifact_existing_nonempty_file_succeeds() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"build output bytes").unwrap();

        let verifier = SupplyChainVerifier::new();
        let result = verifier.verify_artifact(file.path().to_str().unwrap()).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_inspect_artifact_computes_stable_digest() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"deterministic content").unwrap();

        let verifier = SupplyChainVerifier::new();
        let path = file.path().to_str().unwrap();
        let first = verifier.inspect_artifact(path).await.unwrap();
        let second = verifier.inspect_artifact(path).await.unwrap();

        assert_eq!(first.sha256, second.sha256);
        assert_eq!(first.sha256.len(), 64);
        assert_eq!(first.size_bytes, "deterministic content".len() as u64);
    }

    #[tokio::test]
    async fn test_verify_provenance_rejects_empty_signature() {
        let verifier = SupplyChainVerifier::new();
        let provenance = Provenance {
            artifact: "build.tar.gz".to_string(),
            builder: "ci-runner".to_string(),
            timestamp: chrono::Utc::now(),
            signature: "".to_string(),
            source: "github.com/example/repo".to_string(),
        };
        assert!(!verifier.verify_provenance(&provenance).await.unwrap());
    }

    #[tokio::test]
    async fn test_verify_provenance_accepts_well_formed_record() {
        let verifier = SupplyChainVerifier::new();
        let provenance = Provenance {
            artifact: "build.tar.gz".to_string(),
            builder: "ci-runner".to_string(),
            timestamp: chrono::Utc::now(),
            signature: "deadbeef".to_string(),
            source: "github.com/example/repo".to_string(),
        };
        assert!(verifier.verify_provenance(&provenance).await.unwrap());
    }
}
