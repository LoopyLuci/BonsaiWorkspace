use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
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
    next_id: Arc<AtomicU64>,
}

impl ChannelManager {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(DashMap::new()),
            next_id: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn create_channel(&self, session_id: String, channel_type: ChannelType, bandwidth: u32) -> String {
        // A monotonic counter, not `self.channels.len()`: length-derived ids
        // collide once a channel is closed and a new one is created,
        // silently overwriting an unrelated still-open channel's entry.
        let id_num = self.next_id.fetch_add(1, Ordering::Relaxed);
        let channel_id = format!("ch_{}", id_num);
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

    #[test]
    fn test_channel_ids_never_collide_after_close() {
        // Regression test: length-derived ids would reuse "ch_1" here once
        // the first channel is closed, silently clobbering the second
        // channel's entry.
        let cm = ChannelManager::new();
        let first = cm.create_channel("s1".to_string(), ChannelType::Control, 100);
        let second = cm.create_channel("s1".to_string(), ChannelType::Streaming, 200);
        assert!(cm.close_channel(&first));

        let third = cm.create_channel("s1".to_string(), ChannelType::Interactive, 300);
        assert_ne!(third, second, "new channel id must not collide with a still-open channel");

        // The second channel must still be exactly as it was created.
        let still_open = cm.get_channel(&second).unwrap();
        assert_eq!(still_open.channel_type, ChannelType::Streaming);
        assert_eq!(still_open.bandwidth, 200);
    }
}
