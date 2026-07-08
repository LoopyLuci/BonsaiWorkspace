use crate::{Result, RepositoryError};
use app_manager_core::Manifest;
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};

pub struct PackageValidator;

impl PackageValidator {
    pub fn validate_checksum(data: &[u8], expected: &str) -> Result<bool> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let hex = hex::encode(result);

        Ok(hex == expected)
    }

    pub fn validate_manifest(manifest: &Manifest) -> Result<()> {
        if manifest.app_id.as_str().is_empty() {
            return Err(RepositoryError::InvalidManifest);
        }

        if manifest.name.is_empty() {
            return Err(RepositoryError::InvalidManifest);
        }

        if manifest.author.is_empty() {
            return Err(RepositoryError::InvalidManifest);
        }

        Ok(())
    }

    pub fn validate_package_structure(data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Err(RepositoryError::CorruptedPackage);
        }

        Ok(())
    }

    pub fn validate_signature(data: &[u8], signature: &str) -> Result<bool> {
        // Expected secret key for HMAC (in production, this would be configured)
        let secret = b"omnisystem-app-manager-secret";

        // Compute HMAC-SHA256
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret)
            .map_err(|_| RepositoryError::InvalidSignature)?;
        mac.update(data);

        // Convert computed MAC to hex
        let computed_hex = hex::encode(mac.finalize().into_bytes());

        // Compare with provided signature (constant-time comparison to prevent timing attacks)
        if computed_hex.len() != signature.len() {
            return Ok(false);
        }

        let mut matches = true;
        for (computed, provided) in computed_hex.bytes().zip(signature.bytes()) {
            if computed != provided {
                matches = false;
            }
        }

        Ok(matches)
    }

    pub fn calculate_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex::encode(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_manager_core::{AppId, Version};

    #[test]
    fn test_calculate_hash() {
        let data = b"test data";
        let hash = PackageValidator::calculate_hash(data);
        let expected = "916f0027a575074ce72a331777c3478d6513f786a591bd892da1a577bf2335f9";
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_validate_checksum() {
        let data = b"test data";
        let expected = "916f0027a575074ce72a331777c3478d6513f786a591bd892da1a577bf2335f9";

        let result = PackageValidator::validate_checksum(data, expected).unwrap();
        assert!(result);
    }

    #[test]
    fn test_invalid_checksum() {
        let data = b"test data";
        let expected = "invalid";

        let result = PackageValidator::validate_checksum(data, expected).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_validate_manifest() {
        let mut manifest = Manifest {
            app_id: AppId::new("test").unwrap(),
            name: "Test App".to_string(),
            version: Version::new(1, 0, 0),
            description: "Test".to_string(),
            author: "Test Author".to_string(),
            license: "MIT".to_string(),
            dependencies: Vec::new(),
            modules: Default::default(),
            entry_points: Default::default(),
            permissions: Default::default(),
            environment: Default::default(),
        };

        assert!(PackageValidator::validate_manifest(&manifest).is_ok());

        manifest.author = String::new();
        assert!(PackageValidator::validate_manifest(&manifest).is_err());
    }

    #[test]
    fn test_validate_package_structure() {
        assert!(PackageValidator::validate_package_structure(b"data").is_ok());
        assert!(PackageValidator::validate_package_structure(b"").is_err());
    }
}
