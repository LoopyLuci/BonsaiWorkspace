use freellmapi_core::*;
use async_trait::async_trait;
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use sha2::{Sha256, Digest};
use hex::encode as hex_encode;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

pub struct AuthService {
    storage: Arc<dyn StorageRepository>,
    secret_key: String,
    cache: Arc<DashMap<String, CachedAuth>>,
}

#[derive(Debug, Clone)]
struct CachedAuth {
    tenant: Tenant,
    expires_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct JWTClaims {
    sub: String,
    scopes: Vec<String>,
    exp: u64,
    iat: u64,
    iss: String,
}

impl AuthService {
    pub fn new(storage: Arc<dyn StorageRepository>, secret_key: &str) -> Result<Self> {
        Ok(AuthService {
            storage,
            secret_key: secret_key.to_string(),
            cache: Arc::new(DashMap::new()),
        })
    }

    pub fn hash_api_key(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hex_encode(hasher.finalize())
    }

    pub async fn validate_api_key(&self, key: &str) -> Result<Tenant> {
        let key_hash = self.hash_api_key(key);

        // Check cache first (5-min TTL)
        if let Some(cached) = self.cache.get(&key_hash) {
            if cached.expires_at > unix_now() {
                return Ok(cached.tenant.clone());
            }
        }

        // Query from storage
        let api_key = self.storage.get_api_key_by_hash(&key_hash).await?;

        // Check expiration
        if let Some(expires_at) = api_key.expires_at {
            if expires_at <= unix_now() {
                return Err(anyhow!("API key expired"));
            }
        }

        let tenant = self.storage.get_tenant(&api_key.tenant_id).await?;

        // Cache result (5 min TTL)
        let expires_at = unix_now() + 300;
        self.cache.insert(
            key_hash,
            CachedAuth {
                tenant: tenant.clone(),
                expires_at,
            },
        );

        Ok(tenant)
    }

    pub async fn issue_jwt(&self, tenant_id: &str, scopes: Vec<&str>) -> Result<String> {
        let now = unix_now();
        let scopes: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();

        let claims = JWTClaims {
            sub: tenant_id.to_string(),
            scopes,
            exp: now + 3600, // 1 hour expiry
            iat: now,
            iss: "freellmapi".to_string(),
        };

        let json = serde_json::to_string(&claims)?;

        // Simple JWT encoding (header.payload.signature)
        let header_str = serde_json::to_string(&serde_json::json!({
            "alg": "HS256",
            "typ": "JWT"
        }))?;
        let header = base64::encode_simple(header_str.as_bytes());

        let payload = base64::encode_simple(json.as_bytes());

        // Create HMAC signature
        use sha2::Sha256;
        use hmac::{Hmac, Mac};

        type HmacSha256 = Hmac<Sha256>;
        let message = format!("{}.{}", header, payload);
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .map_err(|_| anyhow!("Invalid secret key"))?;
        mac.update(message.as_bytes());
        let signature = hex_encode(mac.finalize().into_bytes());

        Ok(format!("{}.{}.{}", header, payload, signature))
    }

    pub async fn validate_jwt(&self, token: &str) -> Result<(String, Vec<String>)> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(anyhow!("Invalid JWT format"));
        }

        let payload = base64::decode_simple(parts[1])
            .map_err(|_| anyhow!("Invalid JWT payload encoding"))?;

        let claims: JWTClaims = serde_json::from_slice(&payload)?;

        // Check expiration
        if claims.exp <= unix_now() {
            return Err(anyhow!("JWT token expired"));
        }

        // Verify signature
        use sha2::Sha256;
        use hmac::{Hmac, Mac};

        type HmacSha256 = Hmac<Sha256>;
        let message = format!("{}.{}", parts[0], parts[1]);
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .map_err(|_| anyhow!("Invalid secret key"))?;
        mac.update(message.as_bytes());

        let expected_signature = hex_encode(mac.finalize().into_bytes());
        if expected_signature != parts[2] {
            return Err(anyhow!("Invalid JWT signature"));
        }

        Ok((claims.sub, claims.scopes))
    }
}

#[async_trait]
impl OmnisystemService for AuthService {
    fn service_id(&self) -> &str {
        "freellmapi-auth"
    }

    fn service_name(&self) -> &str {
        "FreeLLMAPI Auth Service"
    }

    fn version(&self) -> &str {
        "2.0.0"
    }

    async fn initialize(&self) -> Result<()> {
        tracing::info!("Auth service initialized");
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

// Simple base64 encoding (inline since we want no extra deps)
mod base64 {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode_simple(data: &[u8]) -> String {
        let mut result = String::new();
        for chunk in data.chunks(3) {
            let b1 = chunk[0];
            let b2 = chunk.get(1).copied().unwrap_or(0);
            let b3 = chunk.get(2).copied().unwrap_or(0);

            let n = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);

            result.push(CHARS[((n >> 18) & 63) as usize] as char);
            result.push(CHARS[((n >> 12) & 63) as usize] as char);
            if chunk.len() > 1 {
                result.push(CHARS[((n >> 6) & 63) as usize] as char);
            } else {
                result.push('=');
            }
            if chunk.len() > 2 {
                result.push(CHARS[(n & 63) as usize] as char);
            } else {
                result.push('=');
            }
        }
        result
    }

    pub fn decode_simple(data: &str) -> Result<Vec<u8>, String> {
        let mut result = Vec::new();
        let data = data.trim_end_matches('=');

        for chunk in data.chars().collect::<Vec<_>>().chunks(4) {
            let mut n = 0u32;
            for (i, c) in chunk.iter().enumerate() {
                let val = CHARS.iter().position(|&b| b == *c as u8).ok_or("Invalid char")?;
                n |= (val as u32) << (18 - 6 * i);
            }

            result.push((n >> 16) as u8);
            if chunk.len() > 2 {
                result.push((n >> 8) as u8);
            }
            if chunk.len() > 3 {
                result.push(n as u8);
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_hashing() {
        let auth = AuthService::new(
            Arc::new(MockStorage),
            "test-secret",
        ).unwrap();

        let key1 = auth.hash_api_key("test-key-1");
        let key2 = auth.hash_api_key("test-key-2");

        assert_ne!(key1, key2);
        assert_eq!(key1.len(), 64); // SHA256 hex length
    }

    struct MockStorage;

    #[async_trait]
    impl StorageRepository for MockStorage {
        async fn create_tenant(&self, _: &Tenant) -> Result<()> { Ok(()) }
        async fn get_tenant(&self, _: &str) -> Result<Tenant> {
            Ok(Tenant {
                id: "test".to_string(),
                name: "Test".to_string(),
                email: "test@example.com".to_string(),
                tier: "pro".to_string(),
                monthly_budget_usd: 100.0,
                created_at: 0,
            })
        }
        async fn create_api_key(&self, _: &ApiKey) -> Result<()> { Ok(()) }
        async fn get_api_key_by_hash(&self, _: &str) -> Result<ApiKey> {
            Ok(ApiKey {
                id: "key-1".to_string(),
                tenant_id: "test".to_string(),
                key_hash: "hash".to_string(),
                scopes: vec![],
                created_at: 0,
                expires_at: None,
            })
        }
        async fn log_request(&self, _: &RequestLog) -> Result<()> { Ok(()) }
        async fn get_request_logs(
            &self,
            _: &str,
            _: u64,
            _: u64,
            _: u32,
        ) -> Result<Vec<RequestLog>> { Ok(vec![]) }
        async fn get_tenant_costs(&self, _: &str, _: u32) -> Result<f64> { Ok(0.0) }
        async fn create_webhook(&self, _: &Webhook) -> Result<()> { Ok(()) }
        async fn get_webhooks(&self, _: &str) -> Result<Vec<Webhook>> { Ok(vec![]) }
    }
}
