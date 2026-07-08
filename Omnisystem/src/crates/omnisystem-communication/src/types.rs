use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub source: String,
    pub target: String,
    pub payload: Vec<u8>,
}

impl Message {
    pub fn new(id: String, source: String, target: String, payload: Vec<u8>) -> Self {
        Self { id, source, target, payload }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_message_creation() {
        let msg = Message::new(
            "msg1".to_string(),
            "s1".to_string(),
            "t1".to_string(),
            vec![1, 2],
        );
        assert_eq!(msg.id, "msg1");
    }
}
