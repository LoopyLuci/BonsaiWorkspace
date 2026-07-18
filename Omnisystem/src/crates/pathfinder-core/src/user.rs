use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct UserManager {
    users: Arc<DashMap<String, User>>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub enrollment_date: u64,
    pub completed_courses: u32,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            users: Arc::new(DashMap::new()),
        }
    }

    pub fn register_user(&self, user: User) -> Result<()> {
        self.users.insert(user.id.clone(), user);
        tracing::info!("User registered");
        Ok(())
    }

    pub fn get_user(&self, id: &str) -> Result<User> {
        self.users
            .get(id)
            .map(|u| u.value().clone())
            .ok_or_else(|| crate::PathfinderError::UserNotFound(id.to_string()))
    }

    pub fn user_count(&self) -> usize {
        self.users.len()
    }
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_manager() {
        let manager = UserManager::new();
        let user = User {
            id: "u1".to_string(),
            name: "John".to_string(),
            email: "john@example.com".to_string(),
            enrollment_date: 1000,
            completed_courses: 0,
        };
        assert!(manager.register_user(user).is_ok());
        assert_eq!(manager.user_count(), 1);
    }
}
