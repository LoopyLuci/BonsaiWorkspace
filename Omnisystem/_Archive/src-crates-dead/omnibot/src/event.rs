// Universe event structures

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    CommandExecuted {
        event_id: String,
        user_id: String,
        command: String,
        timestamp: u64,
        success: bool,
    },
    IntentClassified {
        event_id: String,
        user_id: String,
        intent: String,
        confidence: f32,
        timestamp: u64,
    },
    UserRegistered {
        event_id: String,
        user_id: String,
        timestamp: u64,
    },
    PermissionDenied {
        event_id: String,
        user_id: String,
        action: String,
        timestamp: u64,
    },
}

impl Event {
    pub fn command_executed(user_id: String, command: String, success: bool) -> Self {
        Self::CommandExecuted {
            event_id: Uuid::new_v4().to_string(),
            user_id,
            command,
            timestamp: chrono::Utc::now().timestamp() as u64,
            success,
        }
    }

    pub fn intent_classified(user_id: String, intent: String, confidence: f32) -> Self {
        Self::IntentClassified {
            event_id: Uuid::new_v4().to_string(),
            user_id,
            intent,
            confidence,
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }

    pub fn user_registered(user_id: String) -> Self {
        Self::UserRegistered {
            event_id: Uuid::new_v4().to_string(),
            user_id,
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }

    pub fn permission_denied(user_id: String, action: String) -> Self {
        Self::PermissionDenied {
            event_id: Uuid::new_v4().to_string(),
            user_id,
            action,
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::command_executed("user1".into(), "help".into(), true);
        match event {
            Event::CommandExecuted { success, .. } => assert!(success),
            _ => panic!(),
        }
    }
}
