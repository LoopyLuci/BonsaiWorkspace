// NuMarkdown-8B-Thinking — document OCR / image-to-Markdown VLM.
//
// `mradermacher/NuMarkdown-8B-Thinking-GGUF` is loaded as a vision-language
// model slot via the existing `llama-server` with `--mmproj`.  The tool
// sends the image to the running slot and requests Markdown output.
//
// ## Model placement
//   - `~/.bonsai/models/numarkdown/NuMarkdown-8B-Q4_K_M.gguf`
//   - `~/.bonsai/models/numarkdown/mmproj.gguf`   (vision encoder)
//   - `$NUMARKDOWN_MODEL_PATH`
//
// The tool connects to whichever llama-server slot is currently running.
// When the dedicated NuMarkdown slot is loaded, results will use its OCR
// capability; otherwise the general VLM attempts a best-effort conversion.

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

use crate::tool_registry::{Tool, ToolResult};

fn find_model() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("NUMARKDOWN_MODEL_PATH") {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let base = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/models/numarkdown");
    for name in &[
        "NuMarkdown-8B-Q4_K_M.gguf",
        "NuMarkdown-8B-Q5_K_M.gguf",
        "NuMarkdown-8B-Q8_0.gguf",
    ] {
        let p = base.join(name);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn find_mmproj() -> Option<PathBuf> {
    let base = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/models/numarkdown");
    for name in &["mmproj.gguf", "mmproj-f16.gguf", "mmproj-q4_0.gguf"] {
        let p = base.join(name);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

pub fn is_available() -> bool {
    find_model().is_some()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownResult {
    pub markdown: String,
    /// The internal reasoning trace (thinking tokens), if present.
    pub thinking_trace: Option<String>,
    pub detected_tables: usize,
    pub detected_headers: usize,
    pub word_count: usize,
    pub model: String,
    pub elapsed_ms: u64,
}

/// Call the running llama-server slot with a vision request.
async fn call_vlm(image_path: &str, slot_url: &str, prompt: &str) -> Result<String, String> {
    let image_bytes = tokio::fs::read(image_path)
        .await
        .map_err(|e| format!("Cannot read image: {e}"))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
    let ext = std::path::Path::new(image_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpeg");
    let mime = match ext {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };

    let payload = json!({
        "messages": [{
            "role": "user",
            "content": [
                { "type": "image_url", "image_url": { "url": format!("data:{mime};base64,{b64}") } },
                { "type": "text", "text": prompt }
            ]
        }],
        "max_tokens": 4096,
        "temperature": 0.1
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{slot_url}/v1/chat/completions"))
        .json(&payload)
        .timeout(Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("VLM request failed: {e}"))?;

    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

fn parse_thinking(raw: &str) -> (String, Option<String>) {
    if let (Some(start), Some(end)) = (raw.find("<think>"), raw.find("</think>")) {
        let think = raw[start + 7..end].trim().to_string();
        let rest = raw[end + 8..].trim().to_string();
        (rest, Some(think))
    } else {
        (raw.to_string(), None)
    }
}

fn count_tables(md: &str) -> usize {
    md.lines()
        .filter(|l| l.contains('|') && l.trim().starts_with('|'))
        .count()
        / 3
}

fn count_headers(md: &str) -> usize {
    md.lines().filter(|l| l.starts_with('#')).count()
}

async fn run_ocr(image_path: &str, slot_url: &str) -> Result<MarkdownResult, String> {
    info!(image = image_path, "[numarkdown] converting to markdown");
    let t0 = std::time::Instant::now();

    let prompt = "Convert this document image to clean, well-structured Markdown. \
                  Preserve all text, tables, headers, and lists exactly. \
                  Use proper Markdown syntax. Output ONLY the Markdown content.";

    let raw = call_vlm(image_path, slot_url, prompt).await?;
    let (markdown, thinking_trace) = parse_thinking(&raw);

    let tables = count_tables(&markdown);
    let headers = count_headers(&markdown);
    let words = markdown.split_whitespace().count();

    Ok(MarkdownResult {
        markdown,
        thinking_trace,
        detected_tables: tables,
        detected_headers: headers,
        word_count: words,
        model: find_model()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "vlm-fallback".into()),
        elapsed_ms: t0.elapsed().as_millis() as u64,
    })
}

// ── Tools ─────────────────────────────────────────────────────────────────────

pub struct ImageToMarkdownTool;

#[async_trait]
impl Tool for ImageToMarkdownTool {
    fn name(&self) -> &str {
        "image_to_markdown"
    }
    fn description(&self) -> &str {
        "Convert a document image (PDF screenshot, scan, spreadsheet) to clean Markdown \
         using NuMarkdown-8B-Thinking VLM. \
         Args: {image_path: string, slot_url?: string}. \
         Returns: markdown, thinking_trace, detected_tables, detected_headers, word_count."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        if !std::path::Path::new(image_path).exists() {
            return Err(format!("Image not found: {image_path}"));
        }
        let slot_url = args["slot_url"].as_str().unwrap_or("http://127.0.0.1:8080");
        info!(image = image_path, "[numarkdown] tool call");
        let result = run_ocr(image_path, slot_url).await?;
        Ok(ToolResult::json(
            &serde_json::to_value(&result).unwrap_or_default(),
        ))
    }
}

pub struct ExtractDocStructureTool;

#[async_trait]
impl Tool for ExtractDocStructureTool {
    fn name(&self) -> &str {
        "extract_document_structure"
    }
    fn description(&self) -> &str {
        "Extract structural outline (sections, table count, image count) from a document image. \
         Args: {image_path: string, slot_url?: string}. \
         Returns: sections[], table_count, image_count, summary."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        if !std::path::Path::new(image_path).exists() {
            return Err(format!("Image not found: {image_path}"));
        }
        let slot_url = args["slot_url"].as_str().unwrap_or("http://127.0.0.1:8080");
        let prompt = "Analyse this document image. List: 1) all section headings in order, \
                      2) count of tables, 3) count of images/figures, 4) a one-sentence summary. \
                      Reply as JSON: {sections:[str], table_count:int, image_count:int, summary:str}";
        let raw = call_vlm(image_path, slot_url, prompt).await?;
        let (clean, _) = parse_thinking(&raw);
        // Try to parse as JSON, else wrap as plain
        let out: Value =
            serde_json::from_str(&clean).unwrap_or_else(|_| json!({ "summary": clean }));
        Ok(ToolResult::json(&out))
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn image_to_markdown(
    image_path: String,
    slot_url: Option<String>,
) -> Result<MarkdownResult, String> {
    if !std::path::Path::new(&image_path).exists() {
        return Err(format!("Image not found: {image_path}"));
    }
    run_ocr(
        &image_path,
        slot_url.as_deref().unwrap_or("http://127.0.0.1:8080"),
    )
    .await
}

#[tauri::command]
pub async fn numarkdown_model_path() -> Result<Option<String>, String> {
    Ok(find_model().map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn numarkdown_mmproj_path() -> Result<Option<String>, String> {
    Ok(find_mmproj().map(|p| p.to_string_lossy().into_owned()))
}
