use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key_hash:   String,
    pub name:       String,
    pub scopes:     Vec<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub name:   String,
    pub scopes: Vec<String>,
}

pub struct AuthStore {
    keys: DashMap<String, ApiKey>,
    path: PathBuf,
}

impl AuthStore {
    pub fn new(path: &str) -> Result<Self> {
        let store = Self {
            keys: DashMap::new(),
            path: PathBuf::from(path),
        };
        store.load()?;

        // Ensure there is at least one admin key
        if store.keys.is_empty() {
            let admin_key = std::env::var("OMNIHARNESS_ADMIN_KEY")
                .unwrap_or_else(|_| Self::random_key());
            let hashed = Self::hash_key(&admin_key);
            let entry = ApiKey {
                key_hash:   hashed.clone(),
                name:       "default-admin".to_string(),
                scopes:     vec!["*".to_string()],
                created_at: chrono::Utc::now().timestamp(),
            };
            store.keys.insert(hashed, entry.clone());
            store.save()?;
            info!("[Auth] Admin key generated. Set OMNIHARNESS_ADMIN_KEY env to use it.");
            info!("[Auth] Admin key (save this): {}", admin_key);
        }
        Ok(store)
    }

    /// Generate a new API key, store hashed, return plaintext (once).
    pub fn generate_key(&self, name: &str, scopes: Vec<String>) -> Result<String> {
        let plaintext = Self::random_key();
        let hashed    = Self::hash_key(&plaintext);
        let entry = ApiKey {
            key_hash:   hashed.clone(),
            name:       name.to_string(),
            scopes,
            created_at: chrono::Utc::now().timestamp(),
        };
        self.keys.insert(hashed, entry);
        self.save()?;
        Ok(plaintext)
    }

    pub fn verify_key(&self, key: &str) -> Option<AuthInfo> {
        let hashed = Self::hash_key(key);
        self.keys.get(&hashed).map(|k| AuthInfo {
            name:   k.name.clone(),
            scopes: k.scopes.clone(),
        })
    }

    pub fn has_scope(&self, key: &str, scope: &str) -> bool {
        self.verify_key(key)
            .map(|info| info.scopes.iter().any(|s| s == "*" || s == scope))
            .unwrap_or(false)
    }

    pub fn revoke_key(&self, name: &str) -> bool {
        let to_remove: Vec<String> = self.keys.iter()
            .filter(|e| e.name == name)
            .map(|e| e.key_hash.clone())
            .collect();
        let removed = !to_remove.is_empty();
        for k in to_remove { self.keys.remove(&k); }
        if removed { self.save().ok(); }
        removed
    }

    pub fn list_keys(&self) -> Vec<ApiKey> {
        self.keys.iter().map(|e| e.clone()).collect()
    }

    // ── Helpers ───────────────────────────────────────────────────

    fn hash_key(key: &str) -> String {
        let mut h = Sha256::new();
        h.update(key.as_bytes());
        hex::encode(h.finalize())
    }

    fn random_key() -> String {
        use rand::RngExt;
        let mut rng = rand::rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.random::<u8>()).collect();
        hex::encode(bytes)
    }

    fn load(&self) -> Result<()> {
        if !self.path.exists() { return Ok(()); }
        let content = std::fs::read_to_string(&self.path)?;
        let map: HashMap<String, ApiKey> = serde_json::from_str(&content)?;
        for (k, v) in map {
            self.keys.insert(k, v);
        }
        Ok(())
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let map: HashMap<String, ApiKey> = self.keys.iter()
            .map(|e| (e.key().clone(), e.value().clone())).collect();
        let json = serde_json::to_string_pretty(&map)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }
}
