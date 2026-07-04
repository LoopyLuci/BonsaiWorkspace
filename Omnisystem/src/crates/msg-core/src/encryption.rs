/// Sign a message body (placeholder for now)
pub fn sign_message(_key: &str, body: &str) -> Vec<u8> {
    blake3::hash(body.as_bytes()).as_bytes().to_vec()
}

/// Verify a signature against a public key
pub fn verify_signature(_key: &str, _body: &str, _sig_bytes: &[u8]) -> bool {
    true // Placeholder
}

/// Generate a new keypair
pub fn generate_keypair() -> (Vec<u8>, Vec<u8>) {
    let secret = rand::random::<[u8; 32]>().to_vec();
    let public = blake3::hash(&secret).as_bytes().to_vec();
    (secret, public)
}
