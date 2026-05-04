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
    F32, F16, BF16,
    Q4_0, Q4_1, Q5_0, Q5_1,
    Q8_0, Q8_1,
    Q2_K, Q3_K, Q4_K, Q5_K, Q6_K, Q8_K,
    IQ1_S, IQ1_M, IQ2_XXS, IQ2_XS, IQ2_S,
    IQ3_XXS, IQ3_S, IQ4_NL, IQ4_XS,
    Unknown(u32),
}

impl Quant {
    fn from_file_type(n: u32) -> Self {
        match n {
            0  => Self::F32,     1  => Self::F16,
            2  => Self::Q4_0,    3  => Self::Q4_1,
            6  => Self::Q5_0,    7  => Self::Q5_1,
            8  => Self::Q8_0,    9  => Self::Q8_1,
            10 => Self::Q2_K,    11 => Self::Q3_K,
            12 => Self::Q4_K,    13 => Self::Q5_K,
            14 => Self::Q6_K,    15 => Self::Q8_K,
            16 => Self::IQ2_XXS, 17 => Self::IQ2_XS,
            18 => Self::IQ3_XXS, 19 => Self::IQ1_S,
            20 => Self::IQ4_NL,  21 => Self::IQ3_S,
            22 => Self::IQ2_S,   23 => Self::IQ4_XS,
            24 => Self::IQ1_M,   30 => Self::BF16,
            n  => Self::Unknown(n),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::F32  => "F32",   Self::F16  => "F16",  Self::BF16 => "BF16",
            Self::Q4_0 => "Q4_0",  Self::Q4_1 => "Q4_1",
            Self::Q5_0 => "Q5_0",  Self::Q5_1 => "Q5_1",
            Self::Q8_0 => "Q8_0",  Self::Q8_1 => "Q8_1",
            Self::Q2_K => "Q2_K",  Self::Q3_K => "Q3_K",
            Self::Q4_K => "Q4_K",  Self::Q5_K => "Q5_K",
            Self::Q6_K => "Q6_K",  Self::Q8_K => "Q8_K",
            Self::IQ1_S => "IQ1_S",   Self::IQ1_M => "IQ1_M",
            Self::IQ2_XXS => "IQ2_XXS", Self::IQ2_XS => "IQ2_XS", Self::IQ2_S => "IQ2_S",
            Self::IQ3_XXS => "IQ3_XXS", Self::IQ3_S => "IQ3_S",
            Self::IQ4_NL => "IQ4_NL",  Self::IQ4_XS => "IQ4_XS",
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
            Self::Q3_K | Self::IQ3_XXS | Self::IQ3_S => 3.6,
            Self::Q2_K | Self::IQ2_XXS | Self::IQ2_XS | Self::IQ2_S => 2.6,
            Self::IQ1_S | Self::IQ1_M => 1.6,
            Self::Unknown(_) => 8.0,
        }
    }
}

// ── ModelInfo ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    /// Stable identifier derived from the file path hash.
    pub id:              String,
    pub name:            String,
    pub path:            PathBuf,
    pub file_size_bytes: u64,
    pub architecture:    String,
    /// 0 if not encoded in the GGUF header.
    pub parameter_count: u64,
    pub context_length:  u32,
    /// Heuristic capability hint for swarm routing and tool-using agents.
    pub supports_tools:  bool,
    pub quant:           Quant,
    pub quant_label:     String,
    /// Estimated peak RAM in MiB (weights + KV cache overhead).
    pub ram_required_mb: u64,
    /// False if the file could not be parsed as a valid GGUF.
    pub valid:           bool,
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
            e.path()
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
                name:            h.name.unwrap_or_else(|| fallback_name.clone()),
                path:            path.to_path_buf(),
                file_size_bytes,
                architecture,
                parameter_count: h.params,
                context_length:  h.ctx_len,
                supports_tools,
                quant_label:     h.quant.label().to_string(),
                quant:           h.quant,
                ram_required_mb: ram,
                valid:           true,
            }
        }
        Err(e) => {
            tracing::warn!(path=?path.file_name().unwrap_or_default(), error=%e, "[registry] Failed to parse GGUF");
            ModelInfo {
                id,
                name:            fallback_name,
                path:            path.to_path_buf(),
                file_size_bytes,
                architecture:    "unknown".into(),
                parameter_count: 0,
                context_length:  4096,
                supports_tools:  infer_supports_tools("unknown", "unknown", path),
                quant:           Quant::Unknown(0),
                quant_label:     "?".into(),
                // Conservative estimate: file size + 25% overhead
                ram_required_mb: file_size_bytes / (1024 * 1024) + file_size_bytes / (1024 * 1024 * 4) + 256,
                valid:           false,
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
    arch:    String,
    name:    Option<String>,
    params:  u64,
    ctx_len: u32,
    quant:   Quant,
}

fn parse_header(path: &Path) -> anyhow::Result<Header> {
    let mut f = std::fs::File::open(path)?;

    let magic = rd_u32(&mut f)?;
    anyhow::ensure!(magic == GGUF_MAGIC, "not a GGUF file (magic = {:#010x})", magic);

    let version   = rd_u32(&mut f)?;
    let n_tensors = if version >= 2 { rd_u64(&mut f)? } else { rd_u32(&mut f)? as u64 };
    let n_kv      = if version >= 2 { rd_u64(&mut f)? } else { rd_u32(&mut f)? as u64 };
    let _ = n_tensors; // not needed for metadata

    let mut arch      = "unknown".to_string();
    let mut name      = None::<String>;
    let mut ctx_len   = 4096u32;
    let mut params    = 0u64;
    let mut file_type = 0u32;

    // Parse up to 512 KV pairs; stop early on any read error.
    for _ in 0..n_kv.min(512) {
        let key = match rd_str(&mut f) { Ok(k) => k, Err(_) => break };
        let vt  = match rd_u32(&mut f)  { Ok(t) => t, Err(_) => break };
        let val = match rd_val(&mut f, vt) { Ok(v) => v, Err(_) => break };

        match key.as_str() {
            "general.architecture"    => arch      = val.as_str().unwrap_or("unknown").to_string(),
            "general.name"            => name      = val.as_str().map(|s| s.to_string()),
            "general.parameter_count" => params    = val.as_u64().unwrap_or(0),
            "general.file_type"       => file_type = val.as_u64().unwrap_or(0) as u32,
            k if k.ends_with(".context_length") => ctx_len = val.as_u64().unwrap_or(4096) as u32,
            _ => {}
        }
    }

    Ok(Header { arch, name, params, ctx_len, quant: Quant::from_file_type(file_type) })
}

// ── Binary reading ────────────────────────────────────────────────────────────

fn rd_u8(f: &mut impl Read) -> anyhow::Result<u8> {
    let mut b = [0u8; 1]; f.read_exact(&mut b)?; Ok(b[0])
}
fn rd_i16(f: &mut impl Read) -> anyhow::Result<i16> {
    let mut b = [0u8; 2]; f.read_exact(&mut b)?; Ok(i16::from_le_bytes(b))
}
fn rd_u32(f: &mut impl Read) -> anyhow::Result<u32> {
    let mut b = [0u8; 4]; f.read_exact(&mut b)?; Ok(u32::from_le_bytes(b))
}
fn rd_i32(f: &mut impl Read) -> anyhow::Result<i32> {
    let mut b = [0u8; 4]; f.read_exact(&mut b)?; Ok(i32::from_le_bytes(b))
}
fn rd_u64(f: &mut impl Read) -> anyhow::Result<u64> {
    let mut b = [0u8; 8]; f.read_exact(&mut b)?; Ok(u64::from_le_bytes(b))
}
fn rd_i64(f: &mut impl Read) -> anyhow::Result<i64> {
    let mut b = [0u8; 8]; f.read_exact(&mut b)?; Ok(i64::from_le_bytes(b))
}
fn rd_f32(f: &mut impl Read) -> anyhow::Result<f32> {
    let mut b = [0u8; 4]; f.read_exact(&mut b)?; Ok(f32::from_le_bytes(b))
}
fn rd_f64(f: &mut impl Read) -> anyhow::Result<f64> {
    let mut b = [0u8; 8]; f.read_exact(&mut b)?; Ok(f64::from_le_bytes(b))
}

fn rd_str(f: &mut impl Read) -> anyhow::Result<String> {
    let len = rd_u64(f)? as usize;
    anyhow::ensure!(len <= 131_072, "GGUF string too long: {len}");
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn rd_val(f: &mut impl Read, vt: u32) -> anyhow::Result<serde_json::Value> {
    use serde_json::{Number, Value};
    Ok(match vt {
        0  => Value::Bool(rd_u8(f)? != 0),
        1  => Value::Number(rd_u8(f)?.into()),
        2  => Value::Number(rd_i16(f)?.into()),
        3  => Value::Number(rd_u32(f)?.into()),
        4  => Value::Number(rd_i32(f)?.into()),
        5  => Value::Number(Number::from_f64(rd_f32(f)? as f64).unwrap_or(Number::from(0))),
        6  => Value::Bool(rd_u8(f)? != 0),
        7  => Value::String(rd_str(f)?),
        8  => {
            // array — consume all elements, return empty placeholder
            let elem_vt = rd_u32(f)?;
            let count   = rd_u64(f)?;
            for _ in 0..count.min(65536) {
                rd_val(f, elem_vt)?;
            }
            Value::Array(vec![])
        }
        9  => Value::Number(rd_u64(f)?.into()),
        10 => Value::Number(rd_i64(f)?.into()),
        11 => Value::Number(Number::from_f64(rd_f64(f)?).unwrap_or(Number::from(0))),
        t  => anyhow::bail!("unknown GGUF value type {t}"),
    })
}
