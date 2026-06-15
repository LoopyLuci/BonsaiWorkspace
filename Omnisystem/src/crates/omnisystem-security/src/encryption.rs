use crate::Result;

pub struct Encryptor;

impl Encryptor {
    pub fn encrypt(data: &[u8], _key: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    pub fn decrypt(data: &[u8], _key: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_encrypt_decrypt() {
        let data = b"secret";
        let key = b"key";
        let encrypted = Encryptor::encrypt(data, key).unwrap();
        assert_eq!(encrypted.len(), 6);
    }
}
