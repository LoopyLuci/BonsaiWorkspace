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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn token(caps: &[&str], exp: Option<i64>) -> CapabilityToken {
        CapabilityToken {
            subject: Some("user-1".to_string()),
            capabilities: caps.iter().map(|s| s.to_string()).collect(),
            exp,
            sig: None,
        }
    }

    #[test]
    fn test_has_capability_matches_exact_and_wildcard() {
        let t = token(&["ApiCap:inference"], None);
        assert!(t.has_capability("ApiCap:inference"));
        assert!(!t.has_capability("ApiCap:discovery"));

        let wildcard = token(&["ApiCap:*"], None);
        assert!(wildcard.has_capability("ApiCap:anything"));
    }

    #[test]
    fn test_verify_rejects_expired_token() {
        let expired = token(&["ApiCap:*"], Some(0)); // epoch, long expired
        assert!(expired.verify().is_err());

        let far_future = token(&["ApiCap:*"], Some(4_000_000_000));
        assert!(far_future.verify().is_ok());

        let no_expiry = token(&["ApiCap:*"], None);
        assert!(no_expiry.verify().is_ok());
    }

    #[test]
    fn test_extract_bearer_token_requires_bearer_prefix() {
        let mut headers = HeaderMap::new();
        let json = serde_json::to_string(&token(&["ApiCap:*"], None)).unwrap();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&json).unwrap(), // no "Bearer " prefix
        );
        assert!(extract_bearer_token(&headers).is_err());
    }

    #[test]
    fn test_extract_bearer_token_success() {
        let mut headers = HeaderMap::new();
        let json = serde_json::to_string(&token(&["ApiCap:inference"], None)).unwrap();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {json}")).unwrap(),
        );

        let extracted = extract_bearer_token(&headers).unwrap();
        assert!(extracted.has_capability("ApiCap:inference"));
    }

    #[test]
    fn test_extract_bearer_token_rejects_expired() {
        let mut headers = HeaderMap::new();
        let json = serde_json::to_string(&token(&["ApiCap:*"], Some(0))).unwrap();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {json}")).unwrap(),
        );

        assert!(extract_bearer_token(&headers).is_err());
    }
}
