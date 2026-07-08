//! Shared low-level GGUF binary parsing primitives.
//!
//! Spec: https://github.com/ggerganov/ggml/blob/master/docs/gguf.md
//! Layout: magic:u32 "GGUF" (0x46554747 little-endian), version:u32,
//! tensor_count:u64, metadata_kv_count:u64, then metadata_kv_count entries of
//! (key: gguf_string, value_type: u32, value).
//!
//! Extracted from `model_discovery.rs` (which only ever needed scalar
//! metadata values for `ModelProfile`) so `gguf_tokenizer.rs` can reuse the
//! exact same reader for the vocab/merges arrays — real GGUF tokenizer data
//! lives in metadata array values (`tokenizer.ggml.tokens`/`.merges`/
//! `.token_type`), which the original parser deliberately discarded the
//! contents of (it only needed to skip past them to keep reading later
//! keys). `GgufValue::Array` now stores its real elements instead.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

pub const GGUF_MAGIC: u32 = 0x4655_4747; // "GGUF" little-endian read as u32

#[derive(Debug, Clone)]
pub enum GgufValue {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    F32(f32),
    Bool(bool),
    String(String),
    U64(u64),
    I64(i64),
    F64(f64),
    Array(Vec<GgufValue>),
}

pub fn read_u32<R: Read>(r: &mut R) -> std::io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}
pub fn read_u64<R: Read>(r: &mut R) -> std::io::Result<u64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}
pub fn read_i64<R: Read>(r: &mut R) -> std::io::Result<i64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(i64::from_le_bytes(b))
}

pub fn read_gguf_string<R: Read>(r: &mut R) -> std::io::Result<String> {
    let len = read_u64(r)? as usize;
    // Guard against corrupt/malicious length prefixes on untrusted files.
    if len > 64 * 1024 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "GGUF string length implausibly large",
        ));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

pub fn read_gguf_value<R: Read>(r: &mut R, value_type: u32) -> std::io::Result<GgufValue> {
    Ok(match value_type {
        0 => {
            let mut b = [0u8; 1];
            r.read_exact(&mut b)?;
            GgufValue::U8(b[0])
        }
        1 => {
            let mut b = [0u8; 1];
            r.read_exact(&mut b)?;
            GgufValue::I8(b[0] as i8)
        }
        2 => {
            let mut b = [0u8; 2];
            r.read_exact(&mut b)?;
            GgufValue::U16(u16::from_le_bytes(b))
        }
        3 => {
            let mut b = [0u8; 2];
            r.read_exact(&mut b)?;
            GgufValue::I16(i16::from_le_bytes(b))
        }
        4 => GgufValue::U32(read_u32(r)?),
        5 => GgufValue::I32(read_u32(r)? as i32),
        6 => {
            let v = read_u32(r)?;
            GgufValue::F32(f32::from_bits(v))
        }
        7 => {
            let mut b = [0u8; 1];
            r.read_exact(&mut b)?;
            GgufValue::Bool(b[0] != 0)
        }
        8 => GgufValue::String(read_gguf_string(r)?),
        9 => {
            // Array: element_type:u32, count:u64, then `count` elements.
            let elem_type = read_u32(r)?;
            let count = read_u64(r)?;
            if count > 10_000_000 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "GGUF array length implausibly large",
                ));
            }
            let mut elems = Vec::with_capacity(count.min(1_000_000) as usize);
            for _ in 0..count {
                elems.push(read_gguf_value(r, elem_type)?);
            }
            GgufValue::Array(elems)
        }
        10 => GgufValue::U64(read_u64(r)?),
        11 => GgufValue::I64(read_i64(r)?),
        12 => {
            let v = read_u64(r)?;
            GgufValue::F64(f64::from_bits(v))
        }
        other => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown GGUF value type {other}"),
            ))
        }
    })
}

impl GgufValue {
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            GgufValue::U8(v) => Some(*v as u64),
            GgufValue::U16(v) => Some(*v as u64),
            GgufValue::U32(v) => Some(*v as u64),
            GgufValue::U64(v) => Some(*v),
            GgufValue::I8(v) => Some(*v as u64),
            GgufValue::I16(v) => Some(*v as u64),
            GgufValue::I32(v) => Some(*v as u64),
            GgufValue::I64(v) => Some(*v as u64),
            _ => None,
        }
    }
    pub fn as_string(&self) -> Option<String> {
        match self {
            GgufValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            GgufValue::F32(v) => Some(*v),
            GgufValue::F64(v) => Some(*v as f32),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            GgufValue::Bool(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[GgufValue]> {
        match self {
            GgufValue::Array(v) => Some(v),
            _ => None,
        }
    }
}

/// Parse the real GGUF binary header and return every metadata key present,
/// plus the tensor count. Never fabricates a value that wasn't read from the
/// file. Shared by `model_discovery.rs` (profile metadata) and
/// `gguf_tokenizer.rs` (vocab/merges/special tokens).
pub fn parse_gguf_metadata(path: &Path) -> std::io::Result<(HashMap<String, GgufValue>, u64)> {
    let mut f = std::fs::File::open(path)?;
    let magic = read_u32(&mut f)?;
    if magic != GGUF_MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not a GGUF file (bad magic)",
        ));
    }
    let _version = read_u32(&mut f)?;
    let tensor_count = read_u64(&mut f)?;
    let metadata_kv_count = read_u64(&mut f)?;

    if tensor_count > 1_000_000 || metadata_kv_count > 1_000_000 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "GGUF header counts implausibly large — refusing to parse",
        ));
    }

    let mut metadata = HashMap::new();
    for _ in 0..metadata_kv_count {
        let key = read_gguf_string(&mut f)?;
        let value_type = read_u32(&mut f)?;
        let value = read_gguf_value(&mut f, value_type)?;
        metadata.insert(key, value);
    }

    Ok((metadata, tensor_count))
}
