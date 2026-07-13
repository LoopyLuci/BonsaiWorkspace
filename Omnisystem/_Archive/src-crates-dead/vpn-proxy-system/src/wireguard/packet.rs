//! WireGuard Packet Types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    MessageHandshakeInitiation = 1,
    MessageHandshakeResponse = 2,
    MessageCookieReply = 3,
    MessageData = 4,
}

impl MessageType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(MessageType::MessageHandshakeInitiation),
            2 => Some(MessageType::MessageHandshakeResponse),
            3 => Some(MessageType::MessageCookieReply),
            4 => Some(MessageType::MessageData),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Message {
    pub msg_type: MessageType,
    pub sender_index: u32,
    pub payload: Vec<u8>,
}

impl Message {
    pub fn new(msg_type: MessageType, sender_index: u32, payload: Vec<u8>) -> Self {
        Self {
            msg_type,
            sender_index,
            payload,
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(5 + self.payload.len());
        bytes.push(self.msg_type as u8);
        bytes.extend_from_slice(&self.sender_index.to_le_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 5 {
            return Err("Message too short".to_string());
        }

        let msg_type = MessageType::from_u8(data[0])
            .ok_or_else(|| "Invalid message type".to_string())?;

        let sender_index = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
        let payload = data[5..].to_vec();

        Ok(Self {
            msg_type,
            sender_index,
            payload,
        })
    }

    pub fn get_size(&self) -> usize {
        5 + self.payload.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_conversion() {
        assert_eq!(MessageType::from_u8(1), Some(MessageType::MessageHandshakeInitiation));
        assert_eq!(MessageType::from_u8(4), Some(MessageType::MessageData));
        assert_eq!(MessageType::from_u8(255), None);
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::new(
            MessageType::MessageData,
            12345,
            b"test payload".to_vec(),
        );

        let bytes = msg.as_bytes();
        let decoded = Message::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.msg_type, MessageType::MessageData);
        assert_eq!(decoded.sender_index, 12345);
        assert_eq!(&decoded.payload[..], b"test payload");
    }

    #[test]
    fn test_message_size() {
        let msg = Message::new(MessageType::MessageData, 1, vec![0u8; 100]);
        assert_eq!(msg.get_size(), 105); // 1 + 4 + 100
    }
}
