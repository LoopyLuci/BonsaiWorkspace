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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oidc_validation_is_not_yet_configured() {
        // Honest placeholder: no enterprise OIDC provider is wired in, so
        // every JWT is rejected with a clear reason rather than silently
        // accepted or given a fabricated claim set.
        let result = validate_bearer_jwt("any.jwt.token");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not configured"));
    }
}
