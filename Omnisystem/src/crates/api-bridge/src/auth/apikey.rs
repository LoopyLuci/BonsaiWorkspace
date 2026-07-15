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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_missing_header_is_rejected() {
        let headers = HeaderMap::new();
        let err = verify_api_key(&headers).unwrap_err();
        assert!(err.to_string().contains("missing x-api-key"));
    }

    #[test]
    fn test_unconfigured_keys_reject_any_key() {
        // BONSAI_API_KEYS is not set in the test environment, so the
        // lazily-initialized key set is empty and every key is rejected
        // with a distinct "not configured" error (rather than silently
        // treating an unconfigured gateway as open).
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("whatever"));
        let err = verify_api_key(&headers).unwrap_err();
        assert!(err.to_string().contains("not configured"));
    }
}
