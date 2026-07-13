use anyhow::{anyhow, Result};
use axum::http::HeaderMap;
use once_cell::sync::Lazy;
use std::collections::HashSet;

static API_KEYS: Lazy<HashSet<String>> = Lazy::new(|| {
    std::env::var("BONSAI_API_KEYS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
});

pub fn verify_api_key(headers: &HeaderMap) -> Result<()> {
    let key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| anyhow!("missing x-api-key"))?;

    if API_KEYS.is_empty() {
        return Err(anyhow!("API key auth is not configured"));
    }

    if API_KEYS.contains(key) {
        Ok(())
    } else {
        Err(anyhow!("invalid API key"))
    }
}
