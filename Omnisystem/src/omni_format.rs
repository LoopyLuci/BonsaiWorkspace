// OMNI FORMAT IMPLEMENTATION
// Universal data format with encryption, versioning, and query support
// Version: 2.0

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::io::{Read, Write};

/// OMNI Value - universal data representation
#[derive(Debug, Clone)]
pub enum OmniValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<OmniValue>),
    Object(HashMap<String, OmniValue>),
}

impl OmniValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            OmniValue::Null => "null",
            OmniValue::Bool(_) => "boolean",
            OmniValue::Integer(_) => "integer",
            OmniValue::Float(_) => "float",
            OmniValue::String(_) => "string",
            OmniValue::Bytes(_) => "bytes",
            OmniValue::Array(_) => "array",
            OmniValue::Object(_) => "object",
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            OmniValue::String(s) => Some(s.clone()),
            OmniValue::Integer(i) => Some(i.to_string()),
            OmniValue::Float(f) => Some(f.to_string()),
            OmniValue::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            OmniValue::Integer(i) => Some(*i),
            OmniValue::Float(f) => Some(*f as i64),
            OmniValue::Bool(b) => Some(if *b { 1 } else { 0 }),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            OmniValue::Float(f) => Some(*f),
            OmniValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            OmniValue::Bool(b) => Some(*b),
            OmniValue::Integer(i) => Some(*i != 0),
            OmniValue::Float(f) => Some(*f != 0.0),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<OmniValue>> {
        match self {
            OmniValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, OmniValue>> {
        match self {
            OmniValue::Object(o) => Some(o),
            _ => None,
        }
    }
}

/// OMNI Header (256 bytes)
#[derive(Debug, Clone)]
pub struct OmniHeader {
    pub magic: u32,              // 0x4F4D4E49 ("OMNI")
    pub version: u16,            // Format version
    pub revision: u16,           // Revision number
    pub endianness: u8,          // 0x01 = little endian
    pub compression: CompressionType,
    pub encryption: EncryptionType,
    pub checksum_algo: ChecksumAlgorithm,
    pub total_size: u32,
    pub content_size: u32,
    pub metadata_offset: u32,
    pub schema_offset: u32,
    pub content_offset: u32,
    pub attachments_offset: u32,
    pub history_offset: u32,
    pub timestamp: u64,
    pub modified_timestamp: u64,
}

impl OmniHeader {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        OmniHeader {
            magic: 0x4F4D4E49,
            version: 0x0200,
            revision: 1,
            endianness: 0x01,
            compression: CompressionType::None,
            encryption: EncryptionType::None,
            checksum_algo: ChecksumAlgorithm::SHA256,
            total_size: 256,
            content_size: 0,
            metadata_offset: 256,
            schema_offset: 512,
            content_offset: 1024,
            attachments_offset: 0,
            history_offset: 0,
            timestamp: now,
            modified_timestamp: now,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; 256];
        bytes[0..4].copy_from_slice(&self.magic.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.version.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.revision.to_le_bytes());
        bytes[8] = self.endianness;
        bytes[9] = self.compression as u8;
        bytes[10] = self.encryption as u8;
        bytes[11] = self.checksum_algo as u8;
        bytes[12..16].copy_from_slice(&self.total_size.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.content_size.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.metadata_offset.to_le_bytes());
        bytes[24..28].copy_from_slice(&self.schema_offset.to_le_bytes());
        bytes[28..32].copy_from_slice(&self.content_offset.to_le_bytes());
        bytes[32..36].copy_from_slice(&self.attachments_offset.to_le_bytes());
        bytes[36..40].copy_from_slice(&self.history_offset.to_le_bytes());
        bytes[40..48].copy_from_slice(&self.timestamp.to_le_bytes());
        bytes[48..56].copy_from_slice(&self.modified_timestamp.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, OmniError> {
        if bytes.len() < 256 {
            return Err(OmniError::InvalidFormat("Header too short".to_string()));
        }

        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != 0x4F4D4E49 {
            return Err(OmniError::InvalidFormat("Invalid magic number".to_string()));
        }

        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        let revision = u16::from_le_bytes([bytes[6], bytes[7]]);
        let endianness = bytes[8];
        let compression = CompressionType::from_u8(bytes[9])?;
        let encryption = EncryptionType::from_u8(bytes[10])?;
        let checksum_algo = ChecksumAlgorithm::from_u8(bytes[11])?;

        Ok(OmniHeader {
            magic,
            version,
            revision,
            endianness,
            compression,
            encryption,
            checksum_algo,
            total_size: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            content_size: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            metadata_offset: u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
            schema_offset: u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            content_offset: u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
            attachments_offset: u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]),
            history_offset: u32::from_le_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]),
            timestamp: u64::from_le_bytes([
                bytes[40], bytes[41], bytes[42], bytes[43],
                bytes[44], bytes[45], bytes[46], bytes[47],
            ]),
            modified_timestamp: u64::from_le_bytes([
                bytes[48], bytes[49], bytes[50], bytes[51],
                bytes[52], bytes[53], bytes[54], bytes[55],
            ]),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CompressionType {
    None = 0,
    Zstandard = 1,
    Brotli = 2,
    LZMA = 3,
    Zlib = 4,
}

impl CompressionType {
    pub fn from_u8(n: u8) -> Result<Self, OmniError> {
        match n {
            0 => Ok(CompressionType::None),
            1 => Ok(CompressionType::Zstandard),
            2 => Ok(CompressionType::Brotli),
            3 => Ok(CompressionType::LZMA),
            4 => Ok(CompressionType::Zlib),
            _ => Err(OmniError::InvalidFormat("Unknown compression type".to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EncryptionType {
    None = 0,
    AES256GCM = 1,
    ChaCha20 = 2,
}

impl EncryptionType {
    pub fn from_u8(n: u8) -> Result<Self, OmniError> {
        match n {
            0 => Ok(EncryptionType::None),
            1 => Ok(EncryptionType::AES256GCM),
            2 => Ok(EncryptionType::ChaCha20),
            _ => Err(OmniError::InvalidFormat("Unknown encryption type".to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ChecksumAlgorithm {
    SHA256 = 1,
    SHA3 = 2,
    BLAKE3 = 3,
}

impl ChecksumAlgorithm {
    pub fn from_u8(n: u8) -> Result<Self, OmniError> {
        match n {
            1 => Ok(ChecksumAlgorithm::SHA256),
            2 => Ok(ChecksumAlgorithm::SHA3),
            3 => Ok(ChecksumAlgorithm::BLAKE3),
            _ => Err(OmniError::InvalidFormat("Unknown checksum algorithm".to_string())),
        }
    }
}

/// OMNI Document
pub struct OmniDocument {
    pub header: OmniHeader,
    pub metadata: HashMap<String, String>,
    pub schema: HashMap<String, OmniValue>,
    pub content: OmniValue,
    pub attachments: Vec<Attachment>,
}

pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
    pub checksum: String,
}

impl OmniDocument {
    pub fn new() -> Self {
        OmniDocument {
            header: OmniHeader::new(),
            metadata: HashMap::new(),
            schema: HashMap::new(),
            content: OmniValue::Null,
            attachments: Vec::new(),
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, OmniError> {
        let mut result = Vec::new();

        // Write header
        result.extend_from_slice(&self.header.to_bytes());

        // Serialize content to JSON bytes
        let content_json = serialize_value(&self.content);
        result.extend_from_slice(content_json.as_bytes());

        Ok(result)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, OmniError> {
        if bytes.len() < 256 {
            return Err(OmniError::InvalidFormat("File too short".to_string()));
        }

        let header = OmniHeader::from_bytes(&bytes[0..256])?;
        let content_bytes = &bytes[256..];
        let content_str = String::from_utf8(content_bytes.to_vec())
            .map_err(|_| OmniError::InvalidFormat("Invalid UTF-8 in content".to_string()))?;

        let content = deserialize_json(&content_str)?;

        Ok(OmniDocument {
            header,
            metadata: HashMap::new(),
            schema: HashMap::new(),
            content,
            attachments: Vec::new(),
        })
    }
}

/// Serialize OmniValue to JSON string
fn serialize_value(value: &OmniValue) -> String {
    match value {
        OmniValue::Null => "null".to_string(),
        OmniValue::Bool(b) => b.to_string(),
        OmniValue::Integer(i) => i.to_string(),
        OmniValue::Float(f) => f.to_string(),
        OmniValue::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
        OmniValue::Bytes(b) => {
            let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
            format!("\"[bytes:{}]\"", hex)
        }
        OmniValue::Array(arr) => {
            let items: Vec<String> = arr.iter().map(serialize_value).collect();
            format!("[{}]", items.join(","))
        }
        OmniValue::Object(obj) => {
            let items: Vec<String> = obj
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", k, serialize_value(v)))
                .collect();
            format!("{{{}}}", items.join(","))
        }
    }
}

/// Deserialize JSON string to OmniValue (simplified)
fn deserialize_json(json: &str) -> Result<OmniValue, OmniError> {
    let trimmed = json.trim();

    if trimmed == "null" {
        Ok(OmniValue::Null)
    } else if trimmed == "true" {
        Ok(OmniValue::Bool(true))
    } else if trimmed == "false" {
        Ok(OmniValue::Bool(false))
    } else if let Ok(i) = trimmed.parse::<i64>() {
        Ok(OmniValue::Integer(i))
    } else if let Ok(f) = trimmed.parse::<f64>() {
        Ok(OmniValue::Float(f))
    } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
        let s = trimmed[1..trimmed.len() - 1].replace("\\\"", "\"");
        Ok(OmniValue::String(s))
    } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
        // Simple array parsing
        let inner = &trimmed[1..trimmed.len() - 1];
        let items: Vec<OmniValue> = inner
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| deserialize_json(s))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(OmniValue::Array(items))
    } else if trimmed.starts_with('{') && trimmed.ends_with('}') {
        // Simple object parsing
        let mut obj = HashMap::new();
        let inner = &trimmed[1..trimmed.len() - 1];
        for pair in inner.split(',') {
            if let Some(colon_pos) = pair.find(':') {
                let key = pair[..colon_pos].trim().trim_matches('"');
                let val_str = pair[colon_pos + 1..].trim();
                let val = deserialize_json(val_str)?;
                obj.insert(key.to_string(), val);
            }
        }
        Ok(OmniValue::Object(obj))
    } else {
        Err(OmniError::InvalidFormat(format!("Cannot parse: {}", trimmed)))
    }
}

/// OMNI Errors
#[derive(Debug, Clone)]
pub enum OmniError {
    InvalidFormat(String),
    SerializationError(String),
    DeserializationError(String),
    EncryptionError(String),
    CompressionError(String),
    ValidationError(String),
}

impl std::fmt::Display for OmniError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            OmniError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            OmniError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            OmniError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            OmniError::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            OmniError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            OmniError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_omni_header_creation() {
        let header = OmniHeader::new();
        assert_eq!(header.magic, 0x4F4D4E49);
    }

    #[test]
    fn test_omni_header_serialize_deserialize() {
        let header = OmniHeader::new();
        let bytes = header.to_bytes();
        let deserialized = OmniHeader::from_bytes(&bytes).unwrap();
        assert_eq!(deserialized.magic, header.magic);
        assert_eq!(deserialized.version, header.version);
    }

    #[test]
    fn test_omni_value_type_name() {
        assert_eq!(OmniValue::Null.type_name(), "null");
        assert_eq!(OmniValue::Bool(true).type_name(), "boolean");
        assert_eq!(OmniValue::Integer(42).type_name(), "integer");
        assert_eq!(OmniValue::String("hello".to_string()).type_name(), "string");
    }

    #[test]
    fn test_omni_document_serialize_deserialize() {
        let mut doc = OmniDocument::new();
        doc.content = OmniValue::String("Hello, OMNI!".to_string());

        let bytes = doc.serialize().unwrap();
        let deserialized = OmniDocument::deserialize(&bytes).unwrap();

        assert_eq!(
            deserialized.content.as_string(),
            Some("Hello, OMNI!".to_string())
        );
    }

    #[test]
    fn test_serialize_value() {
        let val = OmniValue::Integer(42);
        assert_eq!(serialize_value(&val), "42");

        let val = OmniValue::Bool(true);
        assert_eq!(serialize_value(&val), "true");

        let val = OmniValue::String("test".to_string());
        assert_eq!(serialize_value(&val), "\"test\"");
    }
}
