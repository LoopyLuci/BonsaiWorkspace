//! Model registry: scans the models directory and parses GGUF file headers to
//! expose rich metadata without loading any weights into memory.

use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};

// ── GGUF magic ────────────────────────────────────────────────────────────────

const GGUF_MAGIC: u32 = 0x4655_4747; // "GGUF" in little-endian

// ── Quantization ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, PartialEq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub enum Quant {
    F32,
    F16,
    BF16,
    Q4_0,
    Q4_1,
    Q5_0,
    Q5_1,
    Q8_0,
    Q8_1,
    Q2_K,
    Q3_K,
    Q4_K,
    Q5_K,
    Q6_K,
    Q8_K,
    IQ1_S,
    IQ1_M,
    IQ2_XXS,
    IQ2_XS,
    IQ2_S,
    IQ2_M,
    IQ3_XXS,
    IQ3_XS,
    IQ3_S,
    IQ3_M,
    IQ4_NL,
    IQ4_XS,
    TQ1_0,
    TQ2_0,
    Unknown(u32),
}

impl Quant {
    /// Decodes the GGUF metadata key `general.file_type`, which stores
    /// `enum llama_ftype` (see `llama.h`) — a *different* enum from the
    /// per-tensor `ggml_type` (see `gguf.rs`/`gpu_placement.rs`). The two
    /// only coincide for values 0-3 and by coincidence at 10; past that they
    /// diverge completely. Confirmed empirically against real on-disk GGUF
    /// files: two independently-produced "IQ1_S"-named files both report
    /// `general.file_type=24`, which decodes correctly to `IQ1_S` here (the
    /// previous table wrongly read this as `ggml_type`'s IQ1_M).
    fn from_file_type(n: u32) -> Self {
        match n {
            0 => Self::F32,
            1 => Self::F16,
            2 => Self::Q4_0,
            3 => Self::Q4_1,
            7 => Self::Q8_0,
            8 => Self::Q5_0,
            9 => Self::Q5_1,
            10 => Self::Q2_K,
            11 | 12 | 13 => Self::Q3_K,   // Q3_K_S / Q3_K_M / Q3_K_L
            14 | 15 => Self::Q4_K,        // Q4_K_S / Q4_K_M
            16 | 17 => Self::Q5_K,        // Q5_K_S / Q5_K_M
            18 => Self::Q6_K,
            19 => Self::IQ2_XXS,
            20 => Self::IQ2_XS,
            21 => Self::Q2_K,             // Q2_K_S
            22 => Self::IQ3_XS,
            23 => Self::IQ3_XXS,
            24 => Self::IQ1_S,
            25 => Self::IQ4_NL,
            26 => Self::IQ3_S,
            27 => Self::IQ3_M,
            28 => Self::IQ2_S,
            29 => Self::IQ2_M,
            30 => Self::IQ4_XS,
            31 => Self::IQ1_M,
            32 => Self::BF16,
            36 => Self::TQ1_0,
            37 => Self::TQ2_0,
            n => Self::Unknown(n),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::F32 => "F32",
            Self::F16 => "F16",
            Self::BF16 => "BF16",
            Self::Q4_0 => "Q4_0",
            Self::Q4_1 => "Q4_1",
            Self::Q5_0 => "Q5_0",
            Self::Q5_1 => "Q5_1",
            Self::Q8_0 => "Q8_0",
            Self::Q8_1 => "Q8_1",
            Self::Q2_K => "Q2_K",
            Self::Q3_K => "Q3_K",
            Self::Q4_K => "Q4_K",
            Self::Q5_K => "Q5_K",
            Self::Q6_K => "Q6_K",
            Self::Q8_K => "Q8_K",
            Self::IQ1_S => "IQ1_S",
            Self::IQ1_M => "IQ1_M",
            Self::IQ2_XXS => "IQ2_XXS",
            Self::IQ2_XS => "IQ2_XS",
            Self::IQ2_S => "IQ2_S",
            Self::IQ2_M => "IQ2_M",
            Self::IQ3_XXS => "IQ3_XXS",
            Self::IQ3_XS => "IQ3_XS",
            Self::IQ3_S => "IQ3_S",
            Self::IQ3_M => "IQ3_M",
            Self::IQ4_NL => "IQ4_NL",
            Self::IQ4_XS => "IQ4_XS",
            Self::TQ1_0 => "TQ1_0",
            Self::TQ2_0 => "TQ2_0",
            Self::Unknown(_) => "?",
        }
    }

    /// Approximate average bits per weight for RAM estimation.
    pub fn bits_per_weight(&self) -> f64 {
        match self {
            Self::F32 => 32.0,
            Self::F16 | Self::BF16 => 16.0,
            Self::Q8_0 | Self::Q8_1 | Self::Q8_K => 8.5,
            Self::Q6_K => 6.6,
            Self::Q5_0 | Self::Q5_1 | Self::Q5_K => 5.6,
            Self::Q4_0 | Self::Q4_1 | Self::Q4_K | Self::IQ4_NL | Self::IQ4_XS => 4.6,
            Self::Q3_K | Self::IQ3_M => 3.7,
            Self::IQ3_XXS | Self::IQ3_S => 3.6,
            Self::IQ3_XS => 3.3,
            Self::Q2_K | Self::IQ2_XXS | Self::IQ2_XS | Self::IQ2_S => 2.6,
            Self::IQ2_M => 2.7,
            Self::TQ2_0 => 2.06,
            Self::IQ1_S | Self::IQ1_M => 1.6,
            Self::TQ1_0 => 1.69,
            Self::Unknown(_) => 8.0,
        }
    }
}

// ── ModelInfo ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    /// Stable identifier derived from the file path hash.
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub file_size_bytes: u64,
    pub architecture: String,
    /// 0 if not encoded in the GGUF header.
    pub parameter_count: u64,
    pub context_length: u32,
    /// Heuristic capability hint for swarm routing and tool-using agents.
    pub supports_tools: bool,
    pub quant: Quant,
    pub quant_label: String,
    /// Estimated peak RAM in MiB (weights + KV cache overhead).
    pub ram_required_mb: u64,
    /// False if the file could not be parsed as a valid GGUF.
    pub valid: bool,
}

impl ModelInfo {
    pub fn ram_label(&self) -> String {
        if self.ram_required_mb >= 1024 {
            format!("{:.1} GB", self.ram_required_mb as f64 / 1024.0)
        } else {
            format!("{} MB", self.ram_required_mb)
        }
    }
}

fn infer_supports_tools(name: &str, architecture: &str, path: &Path) -> bool {
    let haystack = format!(
        "{} {} {}",
        name.to_lowercase(),
        architecture.to_lowercase(),
        path.to_string_lossy().to_lowercase()
    );

    let explicit = [
        "functionary",
        "function-calling",
        "function_call",
        "tool",
        "tools",
        "fc",
    ];
    if explicit.iter().any(|needle| haystack.contains(needle)) {
        return true;
    }

    // Most local chat/coder models in this workspace can follow structured tool-call
    // prompting even if the GGUF metadata does not declare it explicitly.
    let families = [
        "llama",
        "qwen",
        "mistral",
        "mixtral",
        "deepseek",
        "granite",
        "gemma",
        "command-r",
        "phi",
        "coder",
    ];
    families.iter().any(|needle| haystack.contains(needle))
}

// ── ModelRegistry ─────────────────────────────────────────────────────────────

pub struct ModelRegistry {
    pub models: Vec<ModelInfo>,
    /// All directories that were (or should be) scanned — kept for refresh.
    pub scan_dirs: Vec<PathBuf>,
}

impl ModelRegistry {
    /// Scan a single directory recursively for `.gguf` files.
    pub fn scan(dir: &Path) -> Self {
        Self::scan_dirs_recursive(&[dir])
    }

    /// Scan multiple directories (each recursively) and merge results.
    /// Deduplicates by stable file-path hash so symlinks don't double-count.
    pub fn scan_dirs_recursive(dirs: &[&Path]) -> Self {
        let mut models: Vec<ModelInfo> = Vec::new();
        let mut seen = std::collections::HashSet::<String>::new();
        for dir in dirs {
            for info in walk_gguf_in(dir) {
                if seen.insert(info.id.clone()) {
                    models.push(info);
                }
            }
        }
        models.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            models,
            scan_dirs: dirs.iter().map(|p| p.to_path_buf()).collect(),
        }
    }

    /// Re-scan the same directories in place.
    pub fn refresh(&mut self) {
        let dirs: Vec<&Path> = self.scan_dirs.iter().map(PathBuf::as_path).collect();
        let fresh = Self::scan_dirs_recursive(&dirs);
        self.models = fresh.models;
    }

    pub fn by_id(&self, id: &str) -> Option<&ModelInfo> {
        self.models.iter().find(|m| m.id == id)
    }
}

/// Walk `dir` recursively up to 4 levels and return all `.gguf` files found.
fn walk_gguf_in(dir: &Path) -> Vec<ModelInfo> {
    use walkdir::WalkDir;
    if !dir.exists() {
        return vec![];
    }
    WalkDir::new(dir)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            // Some downloader/extraction tools leave a directory behind with
            // the same name as the model file it contains (e.g. `foo.gguf/`
            // wrapping the real `foo.gguf/foo.gguf`). WalkDir recurses into
            // it regardless, so without this check that wrapper directory
            // matches the extension filter too — it then fails to parse as
            // a model (permission-denied opening a directory as a file) and
            // shows up as a bogus duplicate entry alongside the real file.
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("gguf"))
                    .unwrap_or(false)
        })
        .map(|e| probe(e.path()))
        .collect()
}

// ── GGUF probe ────────────────────────────────────────────────────────────────

fn probe(path: &Path) -> ModelInfo {
    let file_size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let id = stable_id(path);
    let fallback_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    match parse_header(path) {
        Ok(h) => {
            let ram = estimate_ram(h.params, &h.quant, file_size_bytes);
            let architecture = h.arch.clone();
            let supports_tools = infer_supports_tools(&fallback_name, &architecture, path);
            ModelInfo {
                id,
                name: h.name.unwrap_or_else(|| fallback_name.clone()),
                path: path.to_path_buf(),
                file_size_bytes,
                architecture,
                parameter_count: h.params,
                context_length: h.ctx_len,
                supports_tools,
                quant_label: h.quant.label().to_string(),
                quant: h.quant,
                ram_required_mb: ram,
                valid: true,
            }
        }
        Err(e) => {
            tracing::warn!(path=?path.file_name().unwrap_or_default(), error=%e, "[registry] Failed to parse GGUF");
            ModelInfo {
                id,
                name: fallback_name,
                path: path.to_path_buf(),
                file_size_bytes,
                architecture: "unknown".into(),
                parameter_count: 0,
                context_length: 4096,
                supports_tools: infer_supports_tools("unknown", "unknown", path),
                quant: Quant::Unknown(0),
                quant_label: "?".into(),
                // Conservative estimate: file size + 25% overhead
                ram_required_mb: file_size_bytes / (1024 * 1024)
                    + file_size_bytes / (1024 * 1024 * 4)
                    + 256,
                valid: false,
            }
        }
    }
}

fn stable_id(path: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn estimate_ram(params: u64, quant: &Quant, file_size: u64) -> u64 {
    let weights_mb = if params > 0 {
        (params as f64 * quant.bits_per_weight() / 8.0 / (1024.0 * 1024.0)) as u64
    } else {
        file_size / (1024 * 1024)
    };
    // KV cache + runtime overhead ≈ 15 % on top of weights
    weights_mb + weights_mb / 7 + 256
}

// ── GGUF header parser ────────────────────────────────────────────────────────

struct Header {
    arch: String,
    name: Option<String>,
    params: u64,
    ctx_len: u32,
    quant: Quant,
}

/// Detect a GGUF file's quantization from its header alone, without a full
/// registry scan. Used by callers (e.g. `gpu_model_loader`) that only have a
/// path, not an already-scanned `ModelInfo`, but still need the same
/// GPU-unsafe-quant check the orchestrator applies.
pub fn quant_for_path(path: &Path) -> Quant {
    parse_header(path).map(|h| h.quant).unwrap_or(Quant::Unknown(u32::MAX))
}

fn parse_header(path: &Path) -> anyhow::Result<Header> {
    let mut f = std::fs::File::open(path)?;

    let magic = rd_u32(&mut f)?;
    anyhow::ensure!(
        magic == GGUF_MAGIC,
        "not a GGUF file (magic = {:#010x})",
        magic
    );

    let version = rd_u32(&mut f)?;
    let n_tensors = if version >= 2 {
        rd_u64(&mut f)?
    } else {
        rd_u32(&mut f)? as u64
    };
    let n_kv = if version >= 2 {
        rd_u64(&mut f)?
    } else {
        rd_u32(&mut f)? as u64
    };
    let _ = n_tensors; // not needed for metadata

    let mut arch = "unknown".to_string();
    let mut name = None::<String>;
    let mut ctx_len = 4096u32;
    let mut params = 0u64;
    // No default here: some GGUF writers (notably for experimental BitNet/1-bit
    // formats like IQ1_S/TQ1_0/TQ2_0) never emit `general.file_type` at all. A
    // `0u32` default would be indistinguishable from a genuine, present
    // `file_type = 0` (F32) — silently misclassifying an exotic low-bit model
    // as full-precision, which is the opposite of conservative. Track presence
    // explicitly and fall back to `Unknown` (treated as GPU-unsafe by the
    // orchestrator) when the key is truly absent.
    let mut file_type: Option<u32> = None;

    // Parse every metadata KV pair (bounded only by a large corruption
    // guard, matching `gguf.rs`'s convention), stopping early on any read
    // error. Previously capped at an arbitrary 512 entries — real files
    // with verbose metadata (custom sampling defaults, per-layer info,
    // etc.) can exceed that easily, silently truncating before reaching
    // keys like `general.file_type` that appear later in the KV section.
    // Confirmed empirically: `Bonsai-1.7B-IQ1_S.gguf` has `general.file_type`
    // present in the raw file bytes, but the old 512-entry cap missed it,
    // making the model report as `Unknown` instead of its real quant.
    for _ in 0..n_kv.min(1_000_000) {
        let key = match rd_str(&mut f) {
            Ok(k) => k,
            Err(_) => break,
        };
        let vt = match rd_u32(&mut f) {
            Ok(t) => t,
            Err(_) => break,
        };
        let val = match rd_val(&mut f, vt) {
            Ok(v) => v,
            Err(_) => break,
        };

        match key.as_str() {
            "general.architecture" => arch = val.as_str().unwrap_or("unknown").to_string(),
            "general.name" => name = val.as_str().map(|s| s.to_string()),
            "general.parameter_count" => params = val.as_u64().unwrap_or(0),
            "general.file_type" => file_type = val.as_u64().map(|v| v as u32),
            k if k.ends_with(".context_length") => ctx_len = val.as_u64().unwrap_or(4096) as u32,
            _ => {}
        }
    }

    Ok(Header {
        arch,
        name,
        params,
        ctx_len,
        quant: file_type.map(Quant::from_file_type).unwrap_or(Quant::Unknown(u32::MAX)),
    })
}

// ── Binary reading ────────────────────────────────────────────────────────────

fn rd_u8(f: &mut impl Read) -> anyhow::Result<u8> {
    let mut b = [0u8; 1];
    f.read_exact(&mut b)?;
    Ok(b[0])
}
fn rd_i8(f: &mut impl Read) -> anyhow::Result<i8> {
    let mut b = [0u8; 1];
    f.read_exact(&mut b)?;
    Ok(b[0] as i8)
}
fn rd_u16(f: &mut impl Read) -> anyhow::Result<u16> {
    let mut b = [0u8; 2];
    f.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b))
}
fn rd_i16(f: &mut impl Read) -> anyhow::Result<i16> {
    let mut b = [0u8; 2];
    f.read_exact(&mut b)?;
    Ok(i16::from_le_bytes(b))
}
fn rd_u32(f: &mut impl Read) -> anyhow::Result<u32> {
    let mut b = [0u8; 4];
    f.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}
fn rd_i32(f: &mut impl Read) -> anyhow::Result<i32> {
    let mut b = [0u8; 4];
    f.read_exact(&mut b)?;
    Ok(i32::from_le_bytes(b))
}
fn rd_u64(f: &mut impl Read) -> anyhow::Result<u64> {
    let mut b = [0u8; 8];
    f.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}
fn rd_i64(f: &mut impl Read) -> anyhow::Result<i64> {
    let mut b = [0u8; 8];
    f.read_exact(&mut b)?;
    Ok(i64::from_le_bytes(b))
}
fn rd_f32(f: &mut impl Read) -> anyhow::Result<f32> {
    let mut b = [0u8; 4];
    f.read_exact(&mut b)?;
    Ok(f32::from_le_bytes(b))
}
fn rd_f64(f: &mut impl Read) -> anyhow::Result<f64> {
    let mut b = [0u8; 8];
    f.read_exact(&mut b)?;
    Ok(f64::from_le_bytes(b))
}

fn rd_str(f: &mut impl Read) -> anyhow::Result<String> {
    let len = rd_u64(f)? as usize;
    anyhow::ensure!(len <= 131_072, "GGUF string too long: {len}");
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// GGUF metadata value-type enum, per spec:
/// https://github.com/ggerganov/ggml/blob/master/docs/gguf.md — the same
/// table `gguf.rs::read_gguf_value` already implements correctly. This
/// function previously used a completely different, incorrect numbering
/// (its `7`/`8` were swapped relative to the real `BOOL`/`STRING` values,
/// among other mismatches), which silently desynchronized the byte stream
/// as soon as a file's metadata contained a value of one of the
/// misidentified types — confirmed empirically: `Bonsai-1.7B-IQ1_S.gguf`'s
/// `general.sampling.*` keys (F32-valued) triggered exactly this, causing
/// `general.file_type` (which appears later in the same file) to never be
/// reached.
fn rd_val(f: &mut impl Read, vt: u32) -> anyhow::Result<serde_json::Value> {
    use serde_json::{Number, Value};
    Ok(match vt {
        0 => Value::Number(rd_u8(f)?.into()),
        1 => Value::Number(rd_i8(f)?.into()),
        2 => Value::Number(rd_u16(f)?.into()),
        3 => Value::Number(rd_i16(f)?.into()),
        4 => Value::Number(rd_u32(f)?.into()),
        5 => Value::Number(rd_i32(f)?.into()),
        6 => Value::Number(Number::from_f64(rd_f32(f)? as f64).unwrap_or(Number::from(0))),
        7 => Value::Bool(rd_u8(f)? != 0),
        8 => Value::String(rd_str(f)?),
        9 => {
            // Array — must consume every element to keep the byte stream
            // aligned for whatever key follows, so the skip count itself
            // needs a large corruption-guard bound (matching gguf.rs's
            // `count > 10_000_000` convention), NOT a small silent-truncate
            // cap: capping at e.g. 65536 for a real large array (tokenizer
            // vocabularies routinely exceed 100k entries) leaves the
            // remaining elements unread, desynchronizing every subsequent
            // key/value read in the file — confirmed empirically as the
            // actual cause of `general.file_type` (positioned after
            // `tokenizer.ggml.tokens`) never being reached.
            let elem_vt = rd_u32(f)?;
            let count = rd_u64(f)?;
            anyhow::ensure!(count <= 10_000_000, "GGUF array length implausibly large: {count}");
            for _ in 0..count {
                rd_val(f, elem_vt)?;
            }
            Value::Array(vec![])
        }
        10 => Value::Number(rd_u64(f)?.into()),
        11 => Value::Number(rd_i64(f)?.into()),
        12 => Value::Number(Number::from_f64(rd_f64(f)?).unwrap_or(Number::from(0))),
        t => anyhow::bail!("unknown GGUF value type {t}"),
    })
}

// ── Adapter scanning ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdapterInfo {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub file_size_bytes: u64,
    /// Base model architecture the adapter was trained on (from metrics.json if present).
    pub base_model: Option<String>,
    pub version: Option<String>,
}

/// Scan a directory for LoRA adapter subdirectories.
///
/// A directory is considered an adapter if it contains any of:
/// `adapter_model.safetensors`, `adapter_model.bin`, or `lora.gguf`.
pub fn scan_adapters(adapter_dir: &Path) -> Vec<AdapterInfo> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(adapter_dir) {
        Ok(e) => e,
        Err(_) => return out,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let has_adapter = path.join("adapter_model.safetensors").exists()
            || path.join("adapter_model.bin").exists()
            || path.join("lora.gguf").exists();

        if !has_adapter {
            continue;
        }

        let dir_size = walkdir::WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum::<u64>();

        let meta: Option<serde_json::Value> = path
            .join("metrics.json")
            .exists()
            .then(|| std::fs::read_to_string(path.join("metrics.json")).ok())
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok());

        let dir_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        out.push(AdapterInfo {
            id: format!("adapter-{dir_name}"),
            name: meta
                .as_ref()
                .and_then(|m| m["name"].as_str())
                .unwrap_or(&dir_name)
                .to_string(),
            path: path.clone(),
            file_size_bytes: dir_size,
            base_model: meta
                .as_ref()
                .and_then(|m| m["base_model"].as_str())
                .map(|s| s.to_string()),
            version: meta
                .as_ref()
                .and_then(|m| m["version"].as_str())
                .map(|s| s.to_string()),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `general.file_type` decodes `llama_ftype`, not `ggml_type` — these
    /// values are the ones empirically confirmed against real on-disk GGUF
    /// files (see `parse_gguf_file_type_matches_real_files` below).
    #[test]
    fn from_file_type_uses_llama_ftype_not_ggml_type() {
        assert_eq!(Quant::from_file_type(0), Quant::F32);
        assert_eq!(Quant::from_file_type(10), Quant::Q2_K);
        // file_type=24 is LLAMA_FTYPE_MOSTLY_IQ1_S, not ggml_type's IQ1_M —
        // this is the bug this phase fixes.
        assert_eq!(Quant::from_file_type(24), Quant::IQ1_S);
        assert_eq!(Quant::from_file_type(31), Quant::IQ1_M);
        assert_eq!(Quant::from_file_type(36), Quant::TQ1_0);
        assert_eq!(Quant::from_file_type(37), Quant::TQ2_0);
        assert_eq!(Quant::from_file_type(9999), Quant::Unknown(9999));
    }

    #[test]
    fn bits_per_weight_orders_monotonically_with_precision() {
        assert!(Quant::F32.bits_per_weight() > Quant::Q8_0.bits_per_weight());
        assert!(Quant::Q8_0.bits_per_weight() > Quant::Q4_K.bits_per_weight());
        assert!(Quant::Q4_K.bits_per_weight() > Quant::Q2_K.bits_per_weight());
        assert!(Quant::Q2_K.bits_per_weight() > Quant::IQ1_S.bits_per_weight());
        assert!(Quant::TQ2_0.bits_per_weight() > Quant::TQ1_0.bits_per_weight());
    }

    /// Cross-checks the corrected `from_file_type` table against real
    /// on-disk GGUF files parsed end-to-end via `parse_header`. Gated behind
    /// `#[ignore]` since these files only exist on the machine that owns the
    /// local model library, not in CI: `cargo test -- --ignored` to run
    /// locally.
    #[test]
    #[ignore]
    fn parse_gguf_file_type_matches_real_files() {
        let cases: &[(&str, Quant)] = &[
            (
                r"D:\Models\general\Bonsai-1.7B-IQ1_S\Bonsai-1.7B-IQ1_S.gguf",
                Quant::IQ1_S,
            ),
            (
                r"D:\Models\general\Gliese-Qwen3.5-0.8B-Abliterated-Caption.i1-IQ1_S.gguf",
                Quant::IQ1_S,
            ),
            (
                r"D:\Models\general\Bonsai-1.7B-TQ1_0\Bonsai-1.7B-TQ1_0.gguf",
                Quant::TQ1_0,
            ),
            (
                r"D:\Models\general\Bonsai-1.7B-TQ2_0\Bonsai-1.7B-TQ2_0.gguf",
                Quant::TQ2_0,
            ),
            (
                r"D:\Models\general\Bonsai-1.7B-Q2_K\Bonsai-1.7B-Q2_K.gguf",
                Quant::Q2_K,
            ),
        ];
        for (path, expected) in cases {
            let path = Path::new(path);
            if !path.exists() {
                continue; // machine-specific fixture; skip rather than fail if absent
            }
            let header = parse_header(path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
            assert_eq!(
                header.quant, *expected,
                "{path:?}: expected {expected:?}, got {:?}",
                header.quant
            );
        }
    }
}
