use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcClaims {
    pub sub: String,
    pub aud: String,
    pub iss: String,
    pub exp: i64,
}

pub fn validate_bearer_jwt(_jwt: &str) -> anyhow::Result<OidcClaims> {
    anyhow::bail!("OIDC validation is not configured; enable via enterprise auth provider")
}
