use axum::http::HeaderMap;
use capability_registry::CapabilityToken;
use anyhow::{Result, anyhow};

pub fn extract_token(headers: &HeaderMap) -> Result<CapabilityToken> {
    let token_str = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| anyhow!("Missing Authorization header"))?;
    let token = serde_json::from_str::<CapabilityToken>(token_str)?;
    token.verify()?;
    Ok(token)
}
