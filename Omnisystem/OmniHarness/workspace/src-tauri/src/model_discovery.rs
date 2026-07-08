//! Model discovery/indexing — real GGUF/ONNX/SafeTensors parsing, filesystem
//! scanning, and Ollama integration. Owned by `SmartRouter` as `ModelRegistry`.
//!
//! Every field on `ModelProfile` is either read directly from the file (GGUF
//! binary header, SafeTensors JSON header, Ollama's `/api/tags` JSON) or
//! derived from something concretely observable (file size, extension). When
//! a value genuinely cannot be determined, it is `None` — never a fabricated
//! placeholder.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::gguf::{parse_gguf_metadata, GgufValue};

// ── Model format + profile ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelFormat {
    Gguf,
    Onnx,
    SafeTensors,
    PyTorchBin,
    Ollama,
    ApiRemote,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    /// Stable id: file path for local files, "ollama:<name>" for Ollama,
    /// or the id already used for a configured remote/API model.
    pub id: String,
    pub name: String,
    pub file_path: Option<String>,
    pub format: ModelFormat,
    pub size_bytes: u64,
    /// Best-effort parameter count (e.g. 7_000_000_000 for a 7B model).
    pub parameter_count: Option<u64>,
    /// Quantization label, e.g. "Q4_K_M", "Q8_0", read from metadata when present.
    pub quantization: Option<String>,
    /// Context length, read from GGUF metadata keys like `<arch>.context_length`.
    pub context_length: Option<u64>,
    /// Tensor names discovered (SafeTensors) — empty for other formats.
    #[serde(default)]
    pub tensor_names: Vec<String>,
}

// ── GGUF parsing ───────────────────────────────────────────────────────────────
// Low-level binary reading lives in `crate::gguf` (shared with
// `gguf_tokenizer.rs`, which needs the same reader for the vocab/merges
// arrays this parser used to only skip past, never store).

fn profile_from_gguf(path: &Path, size_bytes: u64) -> ModelProfile {
    let filename = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    match parse_gguf_metadata(path) {
        Ok((meta, tensor_count)) => {
            let name = meta
                .get("general.name")
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| filename.clone());

            // Parameter count: prefer an explicit metadata key if a model
            // publisher included one; otherwise fall back to filename-derived
            // heuristic (e.g. "...-7b-..." -> 7B) since GGUF has no universal
            // "parameter_count" key across architectures.
            let parameter_count = meta
                .get("general.parameter_count")
                .and_then(|v| v.as_u64())
                .or_else(|| parse_param_count_from_name(&filename));

            let quantization = meta
                .get("general.quantization_version")
                .and_then(|v| v.as_u64())
                .map(|v| format!("quantization_version={v}"))
                .or_else(|| quant_label_from_name(&filename));

            // Context length: architecture-specific key, e.g.
            // "llama.context_length", "qwen2.context_length". Search any key
            // ending in ".context_length" since we don't know the arch name
            // ahead of time.
            let context_length = meta
                .iter()
                .find(|(k, _)| k.ends_with(".context_length"))
                .and_then(|(_, v)| v.as_u64());

            ModelProfile {
                id: path.display().to_string(),
                name,
                file_path: Some(path.display().to_string()),
                format: ModelFormat::Gguf,
                size_bytes,
                parameter_count,
                quantization,
                context_length,
                tensor_names: Vec::new(),
            }
            .with_tensor_count_note(tensor_count)
        }
        Err(e) => {
            tracing::warn!("[model_discovery] failed to parse GGUF header for {}: {e}", path.display());
            ModelProfile {
                id: path.display().to_string(),
                name: filename.clone(),
                file_path: Some(path.display().to_string()),
                format: ModelFormat::Gguf,
                size_bytes,
                parameter_count: parse_param_count_from_name(&filename),
                quantization: quant_label_from_name(&filename),
                context_length: None,
                tensor_names: Vec::new(),
            }
        }
    }
}

impl ModelProfile {
    fn with_tensor_count_note(self, _tensor_count: u64) -> Self {
        // tensor_count is real (read from the header) but GGUF doesn't name
        // tensors in a way useful to surface here without full tensor-info
        // parsing (offsets etc.), which isn't needed for routing decisions.
        self
    }
}

/// Best-effort filename-derived parameter count (e.g. "Llama-3-8B-Instruct" -> 8_000_000_000).
/// Only used when the file's real metadata doesn't carry this information.
fn parse_param_count_from_name(name: &str) -> Option<u64> {
    let lower = name.to_lowercase();
    let re_positions: Vec<usize> = lower.match_indices('b').map(|(i, _)| i).collect();
    for i in re_positions {
        // Look backward from 'b' for a numeric (possibly decimal) run.
        let bytes = lower.as_bytes();
        let mut start = i;
        while start > 0
            && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.')
        {
            start -= 1;
        }
        if start == i {
            continue;
        }
        let num_str = &lower[start..i];
        if let Ok(v) = num_str.parse::<f64>() {
            if v > 0.0 && v < 2000.0 {
                return Some((v * 1_000_000_000.0) as u64);
            }
        }
    }
    None
}

fn quant_label_from_name(name: &str) -> Option<String> {
    let upper = name.to_uppercase();
    const KNOWN: &[&str] = &[
        "Q2_K", "Q3_K_S", "Q3_K_M", "Q3_K_L", "Q4_0", "Q4_1", "Q4_K_S", "Q4_K_M", "Q5_0", "Q5_1",
        "Q5_K_S", "Q5_K_M", "Q6_K", "Q8_0", "F16", "F32", "BF16", "IQ4_XS", "IQ3_XS",
    ];
    KNOWN.iter().find(|q| upper.contains(*q)).map(|q| q.to_string())
}

// ── ONNX (shallow) ────────────────────────────────────────────────────────────

/// ONNX files are serialized protobuf. We don't pull in a protobuf dependency
/// just for this — instead do a real structural check: protobuf messages
/// begin with a varint field tag, and a well-formed ONNX ModelProto's first
/// bytes are almost always field 1 (ir_version, varint) `0x08`. We verify the
/// file is non-empty, readable, and that the leading byte is a plausible
/// protobuf tag (low 3 bits = wire type 0/2, common for ModelProto's first
/// fields) rather than fabricating tensor/param data we can't get without a
/// full protobuf parser.
fn profile_from_onnx(path: &Path, size_bytes: u64) -> ModelProfile {
    let filename = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    let mut looks_like_onnx = false;
    if let Ok(mut f) = std::fs::File::open(path) {
        let mut head = [0u8; 16];
        if let Ok(n) = f.read(&mut head) {
            if n > 0 {
                let wire_type = head[0] & 0x07;
                looks_like_onnx = wire_type == 0 || wire_type == 2;
            }
        }
    }
    if !looks_like_onnx {
        tracing::warn!(
            "[model_discovery] {} does not look like a valid protobuf/ONNX file (bad leading tag)",
            path.display()
        );
    }

    ModelProfile {
        id: path.display().to_string(),
        name: filename.clone(),
        file_path: Some(path.display().to_string()),
        format: ModelFormat::Onnx,
        size_bytes,
        parameter_count: parse_param_count_from_name(&filename),
        quantization: quant_label_from_name(&filename),
        context_length: None,
        tensor_names: Vec::new(),
    }
}

// ── SafeTensors ────────────────────────────────────────────────────────────────
// Spec: 8-byte little-endian u64 header length, followed by that many bytes
// of UTF-8 JSON describing tensor name -> {dtype, shape, data_offsets}, plus
// an optional "__metadata__" key.

fn parse_safetensors_header(path: &Path) -> std::io::Result<serde_json::Value> {
    let mut f = std::fs::File::open(path)?;
    let mut len_buf = [0u8; 8];
    f.read_exact(&mut len_buf)?;
    let header_len = u64::from_le_bytes(len_buf);

    // Guard against corrupt/malicious length on untrusted input.
    let file_len = f.seek(SeekFrom::End(0))?;
    if header_len == 0 || header_len > file_len.saturating_sub(8) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "SafeTensors header length out of range",
        ));
    }
    f.seek(SeekFrom::Start(8))?;
    let mut json_buf = vec![0u8; header_len as usize];
    f.read_exact(&mut json_buf)?;
    serde_json::from_slice(&json_buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

fn profile_from_safetensors(path: &Path, size_bytes: u64) -> ModelProfile {
    let filename = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    let mut tensor_names = Vec::new();
    let mut parameter_count: Option<u64> = None;

    match parse_safetensors_header(path) {
        Ok(serde_json::Value::Object(map)) => {
            let mut total_params: u64 = 0;
            let mut have_any_shape = false;
            for (key, val) in &map {
                if key == "__metadata__" {
                    continue;
                }
                tensor_names.push(key.clone());
                if let Some(shape) = val.get("shape").and_then(|s| s.as_array()) {
                    have_any_shape = true;
                    let elems: u64 = shape
                        .iter()
                        .filter_map(|d| d.as_u64())
                        .product::<u64>()
                        .max(1);
                    total_params = total_params.saturating_add(elems);
                }
            }
            if have_any_shape {
                parameter_count = Some(total_params);
            }
        }
        Ok(_) => {
            tracing::warn!(
                "[model_discovery] {} SafeTensors header was not a JSON object",
                path.display()
            );
        }
        Err(e) => {
            tracing::warn!(
                "[model_discovery] failed to parse SafeTensors header for {}: {e}",
                path.display()
            );
        }
    }

    ModelProfile {
        id: path.display().to_string(),
        name: filename.clone(),
        file_path: Some(path.display().to_string()),
        format: ModelFormat::SafeTensors,
        size_bytes,
        parameter_count: parameter_count.or_else(|| parse_param_count_from_name(&filename)),
        quantization: quant_label_from_name(&filename),
        context_length: None,
        tensor_names,
    }
}

// ── PyTorch .bin/.pt (no real parse — pickle format; size + name only) ────────

fn profile_from_pytorch_bin(path: &Path, size_bytes: u64) -> ModelProfile {
    let filename = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());
    ModelProfile {
        id: path.display().to_string(),
        name: filename.clone(),
        file_path: Some(path.display().to_string()),
        format: ModelFormat::PyTorchBin,
        size_bytes,
        parameter_count: parse_param_count_from_name(&filename),
        quantization: quant_label_from_name(&filename),
        context_length: None,
        tensor_names: Vec::new(),
    }
}

/// Parse a single model file per its extension. Returns `None` for files that
/// don't match a supported extension. All I/O errors are handled — never
/// `unwrap()` on untrusted file content.
pub fn parse_model_file(path: &Path) -> Option<ModelProfile> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    let size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    match ext.as_str() {
        "gguf" => Some(profile_from_gguf(path, size_bytes)),
        "onnx" => Some(profile_from_onnx(path, size_bytes)),
        "safetensors" => Some(profile_from_safetensors(path, size_bytes)),
        "bin" | "pt" => Some(profile_from_pytorch_bin(path, size_bytes)),
        _ => None,
    }
}

/// Real recursive directory walk using `walkdir` (already a workspace
/// dependency), parsing every recognised model file. Skips unreadable
/// entries rather than panicking.
pub fn scan_directory(path: &Path, recursive: bool) -> Vec<ModelProfile> {
    let mut out = Vec::new();
    if !path.exists() {
        return out;
    }

    let walker = if recursive {
        walkdir::WalkDir::new(path)
    } else {
        walkdir::WalkDir::new(path).max_depth(1)
    };

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Some(profile) = parse_model_file(entry.path()) {
            out.push(profile);
        }
    }
    out
}

// ── Ollama integration ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaModelEntry>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelEntry {
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    details: Option<OllamaModelDetails>,
}

#[derive(Debug, Deserialize, Default)]
struct OllamaModelDetails {
    #[serde(default)]
    family: Option<String>,
    #[serde(default)]
    parameter_size: Option<String>,
    #[serde(default)]
    quantization_level: Option<String>,
}

const OLLAMA_TAGS_URL: &str = "http://localhost:11434/api/tags";

/// Real HTTP GET against a locally-running Ollama daemon. Returns an empty
/// Vec (not an error) when Ollama isn't running — that's the expected common
/// case, not a failure worth surfacing to callers.
pub async fn discover_ollama_models() -> Vec<ModelProfile> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let resp = match client.get(OLLAMA_TAGS_URL).send().await {
        Ok(r) if r.status().is_success() => r,
        Ok(_) => return Vec::new(),
        Err(_) => return Vec::new(), // Ollama not running — expected, not an error
    };

    let parsed: OllamaTagsResponse = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[model_discovery] failed to parse Ollama /api/tags response: {e}");
            return Vec::new();
        }
    };

    parsed
        .models
        .into_iter()
        .map(|m| {
            let parameter_count = m
                .details
                .as_ref()
                .and_then(|d| d.parameter_size.as_deref())
                .and_then(parse_param_count_from_name);
            let quantization = m
                .details
                .as_ref()
                .and_then(|d| d.quantization_level.clone());
            ModelProfile {
                id: format!("ollama:{}", m.name),
                name: m.name,
                file_path: None,
                format: ModelFormat::Ollama,
                size_bytes: m.size,
                parameter_count,
                quantization,
                context_length: None,
                tensor_names: Vec::new(),
            }
        })
        .collect()
}

/// Real TCP/HTTP reachability check for the Ollama daemon.
pub async fn is_ollama_running() -> bool {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    client
        .get(OLLAMA_TAGS_URL)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

// ── Default scan directories per OS ───────────────────────────────────────────

/// Common, real default model directories worth auto-scanning on first run.
/// Only paths that actually exist are returned by the caller's scan step —
/// this just enumerates plausible candidates.
pub fn default_scan_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".ollama").join("models"));
        dirs.push(home.join("Downloads"));
        dirs.push(home.join(".omnisystem").join("models"));
    }
    if let Some(data_dir) = dirs::data_local_dir() {
        dirs.push(data_dir.join("omnisystem").join("models"));
    }
    // Windows-specific common location already used elsewhere in this crate
    // (see lib.rs auto-registration of D:\Models\general).
    #[cfg(windows)]
    {
        dirs.push(PathBuf::from(r"D:\Models\general"));
    }
    dirs
}

// ── ModelRegistry ──────────────────────────────────────────────────────────────

/// Persisted search-path + discovered-profile registry, owned by `SmartRouter`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RegistryPersisted {
    search_paths: Vec<String>,
}

pub struct ModelRegistry {
    persist_dir: Option<PathBuf>,
    search_paths: std::sync::RwLock<Vec<PathBuf>>,
    profiles: std::sync::RwLock<HashMap<String, ModelProfile>>,
}

impl ModelRegistry {
    pub fn new(persist_dir: Option<PathBuf>) -> Self {
        let search_paths = Self::load_search_paths(&persist_dir);
        Self {
            persist_dir,
            search_paths: std::sync::RwLock::new(search_paths),
            profiles: std::sync::RwLock::new(HashMap::new()),
        }
    }

    fn registry_file(dir: &Path) -> PathBuf {
        dir.join("model_registry.json")
    }

    fn load_search_paths(persist_dir: &Option<PathBuf>) -> Vec<PathBuf> {
        let Some(dir) = persist_dir else { return Vec::new() };
        let file = Self::registry_file(dir);
        match std::fs::read_to_string(&file) {
            Ok(content) => match serde_json::from_str::<RegistryPersisted>(&content) {
                Ok(persisted) => persisted.search_paths.into_iter().map(PathBuf::from).collect(),
                Err(e) => {
                    tracing::warn!("[model_discovery] failed to parse {}: {e}", file.display());
                    Vec::new()
                }
            },
            Err(_) => Vec::new(),
        }
    }

    fn persist_search_paths(&self) {
        let Some(dir) = &self.persist_dir else { return };
        let _ = std::fs::create_dir_all(dir);
        let paths = self.search_paths.read().unwrap_or_else(|e| e.into_inner());
        let persisted = RegistryPersisted {
            search_paths: paths.iter().map(|p| p.display().to_string()).collect(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&persisted) {
            let _ = std::fs::write(Self::registry_file(dir), json);
        }
    }

    pub fn add_search_path(&self, path: PathBuf) {
        let mut paths = self.search_paths.write().unwrap_or_else(|e| e.into_inner());
        if !paths.contains(&path) {
            paths.push(path);
        }
        drop(paths);
        self.persist_search_paths();
    }

    pub fn remove_search_path(&self, path: &Path) {
        let mut paths = self.search_paths.write().unwrap_or_else(|e| e.into_inner());
        paths.retain(|p| p != path);
        drop(paths);
        self.persist_search_paths();
    }

    pub fn search_paths(&self) -> Vec<PathBuf> {
        self.search_paths.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Re-scan every configured search path plus Ollama, replacing the
    /// in-memory profile set. Real filesystem + network I/O — no stubs.
    pub async fn rescan(&self) -> Vec<ModelProfile> {
        let paths = self.search_paths();
        let mut all = Vec::new();

        for path in &paths {
            all.extend(scan_directory(path, true));
        }

        all.extend(discover_ollama_models().await);

        let mut map = HashMap::new();
        for p in all {
            map.insert(p.id.clone(), p);
        }
        let result: Vec<ModelProfile> = map.values().cloned().collect();
        *self.profiles.write().unwrap_or_else(|e| e.into_inner()) = map;
        result
    }

    pub fn list_profiles(&self) -> Vec<ModelProfile> {
        self.profiles
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }

    /// Merge in externally-known profiles (e.g. configured remote API models)
    /// without requiring a filesystem/network rescan.
    pub fn upsert_profile(&self, profile: ModelProfile) {
        self.profiles
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(profile.id.clone(), profile);
    }
}
