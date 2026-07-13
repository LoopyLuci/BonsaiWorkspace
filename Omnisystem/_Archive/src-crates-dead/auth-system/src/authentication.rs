use crate::{AuthError, AuthResult, Token, User};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct AuthenticationManager {
    users: Arc<DashMap<String, User>>,
    tokens: Arc<DashMap<String, Token>>,
}

impl AuthenticationManager {
    pub fn new() -> Self {
        Self {
            users: Arc::new(DashMap::new()),
            tokens: Arc::new(DashMap::new()),
        }
    }

    pub async fn register(&self, user: &User) -> AuthResult<()> {
        if self.users.contains_key(&user.user_id) {
            return Err(AuthError::InvalidCredentials);
        }
        self.users.insert(user.user_id.clone(), user.clone());
        Ok(())
    }

    pub async fn authenticate(&self, user_id: &str, _password: &str) -> AuthResult<Token> {
        if !self.users.contains_key(user_id) {
            return Err(AuthError::UserNotFound);
        }

        let token = Token {
            token_id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            token_type: "Bearer".to_string(),
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
        };

        self.tokens.insert(token.token_id.clone(), token.clone());
        Ok(token)
    }

    pub async fn verify_token(&self, token_id: &str) -> AuthResult<Token> {
        if let Some(token) = self.tokens.get(token_id) {
            if token.expires_at > Utc::now() {
                Ok(token.clone())
            } else {
                Err(AuthError::TokenExpired)
            }
        } else {
            Err(AuthError::TokenInvalid)
        }
    }

    pub fn user_count(&self) -> usize {
        self.users.len()
    }
}

impl Default for AuthenticationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_user() {
        let manager = AuthenticationManager::new();
        let user = User {
            user_id: "u1".to_string(),
            username: "alice".to_string(),
            password_hash: "hash".to_string(),
            created_at: Utc::now(),
        };

        manager.register(&user).await.unwrap();
        assert_eq!(manager.user_count(), 1);
    }

    #[tokio::test]
    async fn test_authenticate() {
        let manager = AuthenticationManager::new();
        let user = User {
            user_id: "u1".to_string(),
            username: "alice".to_string(),
            password_hash: "hash".to_string(),
            created_at: Utc::now(),
        };

        manager.register(&user).await.unwrap();
        let token = manager.authenticate("u1", "password").await.unwrap();
        assert_eq!(token.user_id, "u1");
    }

    #[tokio::test]
    async fn test_verify_token() {
        let manager = AuthenticationManager::new();
        let user = User {
            user_id: "u1".to_string(),
            username: "alice".to_string(),
            password_hash: "hash".to_string(),
            created_at: Utc::now(),
        };

        manager.register(&user).await.unwrap();
        let token = manager.authenticate("u1", "password").await.unwrap();
        let verified = manager.verify_token(&token.token_id).await.unwrap();
        assert_eq!(verified.user_id, "u1");
    }
}
