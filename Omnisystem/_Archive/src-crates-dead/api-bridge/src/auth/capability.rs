use anyhow::{anyhow, Result};
use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub subject: Option<String>,
    pub capabilities: Vec<String>,
    pub exp: Option<i64>,
    pub sig: Option<String>,
}

impl CapabilityToken {
    pub fn verify(&self) -> Result<()> {
        if let Some(exp) = self.exp {
            let now = chrono::Utc::now().timestamp();
            if now > exp {
                return Err(anyhow!("capability token expired"));
            }
        }
        Ok(())
    }

    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == "ApiCap:*" || c == cap)
    }
}

pub fn extract_bearer_token(headers: &HeaderMap) -> Result<CapabilityToken> {
    let raw = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| anyhow!("missing Authorization bearer token"))?;

    let token: CapabilityToken = serde_json::from_str(raw)
        .map_err(|_| anyhow!("invalid capability token format; expected JSON token"))?;
    token.verify()?;
    Ok(token)
}
