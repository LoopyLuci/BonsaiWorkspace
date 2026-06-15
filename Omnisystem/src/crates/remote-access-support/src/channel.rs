use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Channel {
    pub channel_id: String,
    pub session_id: String,
    pub channel_type: ChannelType,
    pub bandwidth: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    Control,
    FileTransfer,
    Streaming,
    Interactive,
}

pub struct ChannelManager {
    channels: Arc<DashMap<String, Channel>>,
}

impl ChannelManager {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(DashMap::new()),
        }
    }

    pub fn create_channel(&self, session_id: String, channel_type: ChannelType, bandwidth: u32) -> String {
        let channel_id = format!("ch_{}", self.channels.len());
        let channel = Channel {
            channel_id: channel_id.clone(),
            session_id,
            channel_type,
            bandwidth,
        };
        self.channels.insert(channel_id.clone(), channel);
        channel_id
    }

    pub fn get_channel(&self, channel_id: &str) -> Option<Channel> {
        self.channels.get(channel_id).map(|c| c.clone())
    }

    pub fn close_channel(&self, channel_id: &str) -> bool {
        self.channels.remove(channel_id).is_some()
    }

    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let cm = ChannelManager::new();
        let channel_id = cm.create_channel("session1".to_string(), ChannelType::Control, 1000);
        assert!(!channel_id.is_empty());
    }

    #[test]
    fn test_channel_close() {
        let cm = ChannelManager::new();
        let channel_id = cm.create_channel("session1".to_string(), ChannelType::FileTransfer, 5000);
        assert!(cm.close_channel(&channel_id));
        assert_eq!(cm.channel_count(), 0);
    }
}
