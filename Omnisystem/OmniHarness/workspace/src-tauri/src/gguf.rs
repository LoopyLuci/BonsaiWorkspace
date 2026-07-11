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

/// Reads the GGUF magic/version/counts header plus the metadata KV section
/// from an already-open reader, leaving the cursor positioned exactly at the
/// start of the `tensor_info` array that follows (per spec). Shared by
/// `parse_gguf_metadata` (stops here) and `parse_gguf_tensor_info` (keeps
/// reading tensor_info from the same cursor).
fn read_gguf_header_and_metadata<R: Read>(
    r: &mut R,
) -> std::io::Result<(HashMap<String, GgufValue>, u64)> {
    let magic = read_u32(r)?;
    if magic != GGUF_MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not a GGUF file (bad magic)",
        ));
    }
    let _version = read_u32(r)?;
    let tensor_count = read_u64(r)?;
    let metadata_kv_count = read_u64(r)?;

    if tensor_count > 1_000_000 || metadata_kv_count > 1_000_000 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "GGUF header counts implausibly large — refusing to parse",
        ));
    }

    let mut metadata = HashMap::new();
    for _ in 0..metadata_kv_count {
        let key = read_gguf_string(r)?;
        let value_type = read_u32(r)?;
        let value = read_gguf_value(r, value_type)?;
        metadata.insert(key, value);
    }

    Ok((metadata, tensor_count))
}

/// Parse the real GGUF binary header and return every metadata key present,
/// plus the tensor count. Never fabricates a value that wasn't read from the
/// file. Shared by `model_discovery.rs` (profile metadata) and
/// `gguf_tokenizer.rs` (vocab/merges/special tokens).
pub fn parse_gguf_metadata(path: &Path) -> std::io::Result<(HashMap<String, GgufValue>, u64)> {
    let mut f = std::fs::File::open(path)?;
    read_gguf_header_and_metadata(&mut f)
}

/// Per-tensor entry from the GGUF `tensor_info` array (spec: name,
/// n_dimensions, dimensions[n_dimensions], type: ggml_type as u32, offset).
/// `ggml_type` is a *different* enum from the whole-file `general.file_type`
/// metadata key (`llama_ftype`, decoded by `model_registry.rs::Quant`) — see
/// `gpu_placement.rs` for the ggml_type → label mapping, empirically
/// confirmed against real on-disk GGUF files.
#[derive(Debug, Clone, PartialEq)]
pub struct TensorInfo {
    pub name: String,
    pub dims: Vec<u64>,
    pub ggml_type: u32,
    pub offset: u64,
}

impl TensorInfo {
    /// Element count (product of dimensions) — used for weighted (by
    /// parameter count, not raw tensor count) type-distribution stats in
    /// `gpu_placement.rs::classify_tensor_types`.
    pub fn element_count(&self) -> u64 {
        self.dims.iter().product()
    }
}

/// Parses GGUF metadata *and* the per-tensor `tensor_info` array that
/// follows it — the piece `parse_gguf_metadata` deliberately stops short of,
/// needed to know each tensor's actual quantization type (not just the
/// whole-file `general.file_type` summary) for GPU/CPU hybrid placement
/// decisions.
pub fn parse_gguf_tensor_info(
    path: &Path,
) -> std::io::Result<(HashMap<String, GgufValue>, Vec<TensorInfo>)> {
    let mut f = std::fs::File::open(path)?;
    let (metadata, tensor_count) = read_gguf_header_and_metadata(&mut f)?;

    // GGUF tensors are rank <= 4 in every real-world model architecture seen
    // to date; a generous cap catches corrupt/malicious files without
    // rejecting anything legitimate.
    const MAX_DIMS: u32 = 8;

    let mut tensors = Vec::with_capacity(tensor_count.min(100_000) as usize);
    for _ in 0..tensor_count {
        let name = read_gguf_string(&mut f)?;
        let n_dims = read_u32(&mut f)?;
        if n_dims > MAX_DIMS {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("tensor {name:?} has implausible dimension count {n_dims}"),
            ));
        }
        let mut dims = Vec::with_capacity(n_dims as usize);
        for _ in 0..n_dims {
            dims.push(read_u64(&mut f)?);
        }
        let ggml_type = read_u32(&mut f)?;
        let offset = read_u64(&mut f)?;
        tensors.push(TensorInfo {
            name,
            dims,
            ggml_type,
            offset,
        });
    }

    Ok((metadata, tensors))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_gguf_string(buf: &mut Vec<u8>, s: &str) {
        buf.extend_from_slice(&(s.len() as u64).to_le_bytes());
        buf.extend_from_slice(s.as_bytes());
    }

    /// Hand-builds a minimal, spec-valid GGUF byte buffer (magic, version,
    /// zero metadata KVs, two tensors) so the tensor_info parser has
    /// CI-safe coverage that doesn't depend on real model files existing on
    /// the test-running machine.
    fn synthetic_gguf_bytes() -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&GGUF_MAGIC.to_le_bytes());
        buf.extend_from_slice(&3u32.to_le_bytes()); // version
        buf.extend_from_slice(&2u64.to_le_bytes()); // tensor_count
        buf.extend_from_slice(&0u64.to_le_bytes()); // metadata_kv_count

        // Tensor 0: "token_embd.weight", 2D [32000, 4096], type F32 (0)
        write_gguf_string(&mut buf, "token_embd.weight");
        buf.extend_from_slice(&2u32.to_le_bytes()); // n_dimensions
        buf.extend_from_slice(&32000u64.to_le_bytes());
        buf.extend_from_slice(&4096u64.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // ggml_type = F32
        buf.extend_from_slice(&0u64.to_le_bytes()); // offset

        // Tensor 1: "blk.0.ffn_gate.weight", 2D [4096, 11008], type IQ1_S (19)
        write_gguf_string(&mut buf, "blk.0.ffn_gate.weight");
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&4096u64.to_le_bytes());
        buf.extend_from_slice(&11008u64.to_le_bytes());
        buf.extend_from_slice(&19u32.to_le_bytes()); // ggml_type = IQ1_S
        buf.extend_from_slice(&524_288_000u64.to_le_bytes()); // offset

        buf
    }

    #[test]
    fn parse_gguf_tensor_info_reads_synthetic_buffer() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("omnisystem-gguf-test-{}.gguf", std::process::id()));
        std::fs::File::create(&path)
            .unwrap()
            .write_all(&synthetic_gguf_bytes())
            .unwrap();

        let (metadata, tensors) = parse_gguf_tensor_info(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert!(metadata.is_empty());
        assert_eq!(tensors.len(), 2);
        assert_eq!(tensors[0].name, "token_embd.weight");
        assert_eq!(tensors[0].dims, vec![32000, 4096]);
        assert_eq!(tensors[0].ggml_type, 0);
        assert_eq!(tensors[0].element_count(), 32000 * 4096);
        assert_eq!(tensors[1].name, "blk.0.ffn_gate.weight");
        assert_eq!(tensors[1].ggml_type, 19);
    }

    #[test]
    fn parse_gguf_tensor_info_rejects_bad_magic() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "omnisystem-gguf-badmagic-test-{}.gguf",
            std::process::id()
        ));
        std::fs::File::create(&path)
            .unwrap()
            .write_all(b"NOPE0000")
            .unwrap();

        let result = parse_gguf_tensor_info(&path);
        std::fs::remove_file(&path).ok();
        assert!(result.is_err());
    }

    /// Cross-checks the real tensor-level type distribution of an actual
    /// on-disk IQ1_S-quantized model — confirms the mixed-precision premise
    /// the whole GPU/CPU hybrid placement system depends on (some tensors
    /// are IQ1_S, but not all — norms/embedding/output stay higher
    /// precision). Machine-specific fixture, gated behind `#[ignore]`.
    #[test]
    #[ignore]
    fn real_iq1_s_file_is_not_uniformly_quantized() {
        let path = Path::new(r"D:\Models\general\Bonsai-1.7B-IQ1_S\Bonsai-1.7B-IQ1_S.gguf");
        if !path.exists() {
            return;
        }
        let (_metadata, tensors) = parse_gguf_tensor_info(path).unwrap();
        assert!(!tensors.is_empty());

        let total_weight: u64 = tensors.iter().map(|t| t.element_count()).sum();
        let iq1_s_weight: u64 = tensors
            .iter()
            .filter(|t| t.ggml_type == 19) // IQ1_S
            .map(|t| t.element_count())
            .sum();
        let iq1_s_fraction = iq1_s_weight as f64 / total_weight as f64;

        // Confirmed empirically at ~69.5%; assert a wide-but-meaningful
        // range so a materially different requantization doesn't silently
        // pass, while tolerating minor quantizer-version drift.
        assert!(
            iq1_s_fraction > 0.4 && iq1_s_fraction < 0.9,
            "expected a mixed-precision file with IQ1_S as the dominant but not sole \
             type, got {:.1}% IQ1_S by weight",
            iq1_s_fraction * 100.0
        );

        // Some tensor must NOT be IQ1_S — the whole point of tensor-level
        // hybrid placement is that this minority exists.
        assert!(tensors.iter().any(|t| t.ggml_type != 19));
    }
}
