//! Middleware for request/response processing, authentication, and logging

use axum::{
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use log::{debug, info};
use std::time::Instant;
use uuid::Uuid;

/// Request ID middleware
///
/// Adds a unique request ID to each request for tracing
pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    request.extensions_mut().insert(request_id.clone());

    let response = next.run(request).await;

    debug!("Request ID assigned: {}", request_id);

    response
}

/// Logging middleware
///
/// Logs request details and response times
pub async fn logging_middleware(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    info!(
        "{} {} - {} ({}ms)",
        method,
        uri,
        status,
        duration.as_millis()
    );

    response
}

// ============================================================================
// Capability Token Authentication
// ============================================================================

const AUTHORIZATION_HEADER: &str = "authorization";
const BEARER_PREFIX: &str = "Bearer ";

/// Extract capability token from Authorization header
pub fn extract_capability_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|auth_header| {
            if auth_header.starts_with(BEARER_PREFIX) {
                Some(auth_header[BEARER_PREFIX.len()..].to_string())
            } else {
                None
            }
        })
}

/// Capability auth middleware
pub async fn capability_auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    // Extract token if present
    if let Some(token_str) = extract_capability_token(&headers) {
        debug!("Capability token extracted: {}", &token_str[..8.min(token_str.len())]);
        // In production: validate token signature and capabilities
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_format() {
        let id = Uuid::new_v4().to_string();
        assert!(!id.is_empty());
        assert_eq!(id.len(), 36); // UUID format is 36 chars with hyphens
    }

    #[test]
    fn test_extract_capability_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer test-token-123".parse().unwrap());
        let token = extract_capability_token(&headers);
        assert_eq!(token, Some("test-token-123".to_string()));
    }

    #[test]
    fn test_extract_capability_token_missing() {
        let headers = HeaderMap::new();
        let token = extract_capability_token(&headers);
        assert_eq!(token, None);
    }

    #[test]
    fn test_extract_capability_token_invalid_format() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Basic dGVzdA==".parse().unwrap());
        let token = extract_capability_token(&headers);
        assert_eq!(token, None);
    }
}
