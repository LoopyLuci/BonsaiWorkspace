use crate::{Result, SecurityError};
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};

type HmacSha256 = Hmac<Sha256>;

pub struct SignatureVerifier;

impl SignatureVerifier {
    pub fn verify_hmac(data: &[u8], key: &[u8], signature: &str) -> Result<bool> {
        let mut mac = HmacSha256::new_from_slice(key)
            .map_err(|_| SecurityError::CryptoError("Invalid key".to_string()))?;

        mac.update(data);

        let expected = hex::encode(mac.finalize().into_bytes());
        Ok(expected == signature)
    }

    pub fn generate_hmac(data: &[u8], key: &[u8]) -> Result<String> {
        let mut mac = HmacSha256::new_from_slice(key)
            .map_err(|_| SecurityError::CryptoError("Invalid key".to_string()))?;

        mac.update(data);

        Ok(hex::encode(mac.finalize().into_bytes()))
    }

    pub fn verify_sha256(data: &[u8], expected: &str) -> Result<bool> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let hex = hex::encode(result);

        Ok(hex == expected)
    }

    pub fn generate_sha256(data: &[u8]) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();

        Ok(hex::encode(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_hmac() {
        let data = b"test data";
        let key = b"secret key";

        let hmac = SignatureVerifier::generate_hmac(data, key).unwrap();
        assert!(!hmac.is_empty());
    }

    #[test]
    fn test_verify_hmac() {
        let data = b"test data";
        let key = b"secret key";

        let hmac = SignatureVerifier::generate_hmac(data, key).unwrap();
        let verified = SignatureVerifier::verify_hmac(data, key, &hmac).unwrap();

        assert!(verified);
    }

    #[test]
    fn test_generate_sha256() {
        let data = b"test data";
        let hash = SignatureVerifier::generate_sha256(data).unwrap();

        assert_eq!(hash, "916f0027a575074ce72a331777c3478d6513f786a591bd892da1a577bf2335f9");
    }

    #[test]
    fn test_verify_sha256() {
        let data = b"test data";
        let expected = "916f0027a575074ce72a331777c3478d6513f786a591bd892da1a577bf2335f9";

        let verified = SignatureVerifier::verify_sha256(data, expected).unwrap();
        assert!(verified);
    }
}
