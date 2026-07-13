use parking_lot::RwLock;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use crate::KernelError;

pub type ChannelId = u64;
pub type MessageId = u64;

#[derive(Clone, Debug)]
pub struct Message {
    pub id: MessageId,
    pub sender_pid: u64,
    pub receiver_pid: u64,
    pub data: Vec<u8>,
}

pub struct Channel {
    pub id: ChannelId,
    pub sender_pid: u64,
    pub receiver_pid: u64,
    pub messages: RwLock<VecDeque<Message>>,
    pub max_messages: usize,
}

impl Channel {
    pub fn new(id: ChannelId, sender_pid: u64, receiver_pid: u64) -> Self {
        Channel {
            id,
            sender_pid,
            receiver_pid,
            messages: RwLock::new(VecDeque::new()),
            max_messages: 100,
        }
    }

    pub fn send(&self, message: Message) -> Result<(), KernelError> {
        let mut messages = self.messages.write();

        if messages.len() >= self.max_messages {
            return Err(KernelError::IPCError("Channel full".to_string()));
        }

        messages.push_back(message);
        Ok(())
    }

    pub fn recv(&self) -> Option<Message> {
        self.messages.write().pop_front()
    }

    pub fn message_count(&self) -> usize {
        self.messages.read().len()
    }
}

pub struct IPCManager {
    channels: RwLock<BTreeMap<ChannelId, Arc<Channel>>>,
    next_channel_id: RwLock<ChannelId>,
    next_message_id: RwLock<MessageId>,
}

impl IPCManager {
    pub fn new() -> Self {
        IPCManager {
            channels: RwLock::new(BTreeMap::new()),
            next_channel_id: RwLock::new(1),
            next_message_id: RwLock::new(1),
        }
    }

    pub fn create_channel(
        &self,
        sender_pid: u64,
        receiver_pid: u64,
    ) -> Result<Arc<Channel>, KernelError> {
        let channel_id = {
            let mut next_id = self.next_channel_id.write();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let channel = Arc::new(Channel::new(channel_id, sender_pid, receiver_pid));
        self.channels.write().insert(channel_id, Arc::clone(&channel));

        Ok(channel)
    }

    pub fn get_channel(&self, channel_id: ChannelId) -> Option<Arc<Channel>> {
        self.channels.read().get(&channel_id).cloned()
    }

    pub fn send_message(
        &self,
        channel_id: ChannelId,
        sender_pid: u64,
        data: Vec<u8>,
    ) -> Result<(), KernelError> {
        let message_id = {
            let mut next_id = self.next_message_id.write();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let channel = self
            .get_channel(channel_id)
            .ok_or(KernelError::IPCError("Channel not found".to_string()))?;

        channel.send(Message {
            id: message_id,
            sender_pid,
            receiver_pid: channel.receiver_pid,
            data,
        })
    }

    pub fn recv_message(&self, channel_id: ChannelId) -> Result<Option<Message>, KernelError> {
        let channel = self
            .get_channel(channel_id)
            .ok_or(KernelError::IPCError("Channel not found".to_string()))?;

        Ok(channel.recv())
    }

    pub fn delete_channel(&self, channel_id: ChannelId) -> Result<(), KernelError> {
        self.channels
            .write()
            .remove(&channel_id)
            .ok_or(KernelError::IPCError("Channel not found".to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let ipc = IPCManager::new();
        let channel = ipc.create_channel(1, 2);
        assert!(channel.is_ok());
    }

    #[test]
    fn test_send_receive() {
        let ipc = IPCManager::new();
        let channel = ipc.create_channel(1, 2).unwrap();

        let msg = Message {
            id: 1,
            sender_pid: 1,
            receiver_pid: 2,
            data: vec![1, 2, 3],
        };

        let result = channel.send(msg.clone());
        assert!(result.is_ok());

        let received = channel.recv();
        assert!(received.is_some());
    }
}
