use axum::http::HeaderMap;
use anyhow::{Result, anyhow};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimpleToken {
    pub token: String,
}

pub fn extract_token(headers: &HeaderMap) -> Result<SimpleToken> {
    let token_str = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| anyhow!("Missing Authorization header"))?;
    Ok(SimpleToken {
        token: token_str.to_string(),
    })
}
