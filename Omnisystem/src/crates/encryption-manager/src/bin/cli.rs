//! CLI demo: create a key, encrypt a message, then decrypt it back.

use chrono::Utc;
use encryption_manager::{EncryptionEngine, EncryptionKey, KeyManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let keys = KeyManager::new();
    let engine = EncryptionEngine::new();

    keys.create_key(&EncryptionKey {
        key_id: "key-1".to_string(),
        algorithm: "AES-256-GCM".to_string(),
        created_at: Utc::now(),
        expires_at: None,
        is_active: true,
    })
    .await?;

    let encrypted = engine.encrypt("top secret message", "key-1").await?;
    println!("Encrypted with key {}: {}", encrypted.key_id, encrypted.ciphertext);

    let decrypted = engine.decrypt(&encrypted).await?;
    println!("Decrypted: {}", decrypted);

    Ok(())
}
