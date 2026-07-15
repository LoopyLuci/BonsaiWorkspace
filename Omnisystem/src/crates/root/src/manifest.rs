use anyhow::{anyhow, Context, Result};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub components: Vec<Component>,
    pub launcher_version: String,
    pub signature: Option<String>,
    pub public_key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub size_mb: u32,
    pub download_url: String,
    pub hash: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub launch_cmd: Option<String>,
    pub recommended: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    pub risk_level: String,
}

impl Manifest {
    pub fn signed_payload_bytes(&self) -> Result<Vec<u8>> {
        let mut clone = self.clone();
        clone.signature = None;
        serde_json::to_vec(&clone).context("failed to serialize manifest payload")
    }

    pub fn verify(&self, public_key_bytes: &[u8; 32]) -> Result<()> {
        let signature_hex = self
            .signature
            .as_ref()
            .ok_or_else(|| anyhow!("missing manifest signature"))?;
        let sig_vec = hex::decode(signature_hex).context("invalid hex in signature")?;
        let sig_arr: [u8; 64] = sig_vec
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid ed25519 signature length"))?;
        let signature = Signature::from_bytes(&sig_arr);

        let key = VerifyingKey::from_bytes(public_key_bytes)
            .context("invalid public key bytes for manifest verification")?;
        let payload = self.signed_payload_bytes()?;
        key.verify(&payload, &signature)
            .map_err(|e| anyhow!("manifest signature verification failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn sample_manifest() -> Manifest {
        Manifest {
            version: 1,
            components: vec![Component {
                id: "bonsai-workspace".to_string(),
                name: "Bonsai Workspace".to_string(),
                description: "IDE".to_string(),
                version: "2.0.0".to_string(),
                size_mb: 123,
                download_url: "https://example.com/workspace.zip".to_string(),
                hash: "deadbeef".to_string(),
                dependencies: vec![],
                launch_cmd: Some("workspace.exe".to_string()),
                recommended: true,
                tags: vec!["core".to_string()],
                risk_level: "low".to_string(),
            }],
            launcher_version: "1.0.0".to_string(),
            signature: None,
            public_key_id: "root-1".to_string(),
        }
    }

    #[test]
    fn verifies_signed_manifest() {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verify_key = signing_key.verifying_key();

        let mut manifest = sample_manifest();
        let payload = manifest.signed_payload_bytes().expect("payload should serialize");
        let sig = signing_key.sign(&payload);
        manifest.signature = Some(hex::encode(sig.to_bytes()));

        manifest
            .verify(&verify_key.to_bytes())
            .expect("manifest should verify");
    }

    #[test]
    fn fails_on_tampered_manifest() {
        let signing_key = SigningKey::from_bytes(&[9u8; 32]);
        let verify_key = signing_key.verifying_key();

        let mut manifest = sample_manifest();
        let payload = manifest.signed_payload_bytes().expect("payload should serialize");
        let sig = signing_key.sign(&payload);
        manifest.signature = Some(hex::encode(sig.to_bytes()));

        manifest.launcher_version = "9.9.9".to_string();
        let err = manifest.verify(&verify_key.to_bytes()).expect_err("tampered payload must fail");
        assert!(err.to_string().contains("verification failed"));
    }
}
