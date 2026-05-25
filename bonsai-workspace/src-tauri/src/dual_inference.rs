use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use serde::Deserialize;
use serde_json::json;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DualSessionConfig {
    pub base_model_path: String,
    pub bonsai_lora_path: Option<String>,
    pub reference_lora_path: Option<String>,
    pub gpu_layers: u32,
    pub context_size: u32,
}

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct DualResponse {
    pub prompt: String,
    pub bonsai: ParsedModelOutput,
    pub reference: ParsedModelOutput,
    pub timestamp: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ParsedModelOutput {
    pub raw_content: String,
    pub intent: Option<String>,
    pub tools: Vec<String>,
    pub confidence: Option<f32>,
    pub reasoning: Option<String>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ComparisonResult {
    pub prompt: String,
    pub intent_match: bool,
    pub tool_overlap_pct: f64,
    pub bonsai_tools: Vec<String>,
    pub reference_tools: Vec<String>,
    pub bonsai_confidence: Option<f32>,
    pub gaps: Vec<GapDetail>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GapDetail {
    pub gap_type: String,
    pub description: String,
}

// ── Internal JSON parsing ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct BonsaiJsonOutput {
    intent: Option<String>,
    plan: Option<Vec<BonsaiPlanStep>>,
    confidence: Option<f32>,
    reasoning: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BonsaiPlanStep {
    tool: String,
}

fn parse_bonsai_output(raw: &str) -> ParsedModelOutput {
    // Strip markdown code fences if present
    let json_str = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(parsed) = serde_json::from_str::<BonsaiJsonOutput>(json_str) {
        let tools = parsed
            .plan
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.tool)
            .collect();
        return ParsedModelOutput {
            raw_content: raw.to_string(),
            intent: parsed.intent,
            tools,
            confidence: parsed.confidence,
            reasoning: parsed.reasoning,
            latency_ms: 0,
        };
    }

    // Fallback: structured string search against known JSON field patterns
    let intent_candidates = [
        "tool_use", "chat", "swarm_task", "model_query", "system_command",
    ];
    let intent = intent_candidates
        .iter()
        .find(|i| raw.contains(&format!("\"intent\": \"{}\"", i)))
        .map(|s| s.to_string());

    let tool_names = [
        "read_file", "write_file", "list_files", "grep_files", "run_command",
        "search_knowledge", "get_datetime", "get_system_stats", "get_weather", "fetch_url",
    ];
    let tools: Vec<String> = tool_names
        .iter()
        .filter(|t| raw.contains(&format!("\"tool\": \"{}\"", t)))
        .map(|s| s.to_string())
        .collect();

    ParsedModelOutput {
        raw_content: raw.to_string(),
        intent,
        tools,
        confidence: None,
        reasoning: None,
        latency_ms: 0,
    }
}

// ── Shared Server Manager ─────────────────────────────────────────────────────

/// One llama-server process shared across sessions. LoRA switching is done
/// per-request via the `lora` inference parameter.
pub struct SharedServer {
    port: u16,
    child: Mutex<Option<tokio::process::Child>>,
}

impl SharedServer {
    pub async fn launch(
        model_path: &str,
        lora_paths: &[String],
        gpu_layers: u32,
        context_size: u32,
    ) -> Result<Self, String> {
        let port = find_free_port();
        let llama_bin = find_llama_server()?;

        let mut args = vec![
            "-m".to_string(),
            model_path.to_string(),
            "--port".to_string(),
            port.to_string(),
            "--host".to_string(),
            "127.0.0.1".to_string(),
            "--ctx-size".to_string(),
            context_size.to_string(),
            "--n-gpu-layers".to_string(),
            gpu_layers.to_string(),
            "--no-warmup".to_string(),
        ];
        for lora in lora_paths {
            args.push("--lora".to_string());
            args.push(lora.clone());
        }

        info!(
            port,
            loras = lora_paths.len(),
            "[dual_inference] launching shared llama-server"
        );

        let mut child = tokio::process::Command::new(&llama_bin)
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to start llama-server: {e}"))?;

        // Poll health for up to 120 s
        for attempt in 0..60 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            match reqwest::get(format!("http://127.0.0.1:{}/health", port)).await {
                Ok(r) if r.status().is_success() => {
                    info!(port, "[dual_inference] shared server healthy");
                    return Ok(Self {
                        port,
                        child: Mutex::new(Some(child)),
                    });
                }
                Ok(r) => warn!(status=%r.status(), "[dual_inference] health check returned non-ok"),
                Err(e) => warn!(attempt, error=%e, "[dual_inference] health check attempt failed"),
            }
        }

        let _ = child.kill().await;
        Err(format!("Server on port {port} did not become healthy within 120s"))
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn shutdown(&self) {
        if let Some(mut child) = self.child.lock().await.take() {
            info!(port = self.port, "[dual_inference] shutting down shared server");
            let _ = child.kill().await;
        }
    }
}

impl Drop for SharedServer {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.child.try_lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.start_kill();
            }
        }
    }
}

// ── Dual Model Session ────────────────────────────────────────────────────────

pub struct DualModelSession {
    server: Arc<SharedServer>,
    bonsai_lora: Option<String>,
    client: reqwest::Client,
}

impl DualModelSession {
    pub fn new(server: Arc<SharedServer>, bonsai_lora: Option<String>) -> Self {
        Self {
            server,
            bonsai_lora,
            client: reqwest::Client::new(),
        }
    }

    pub async fn compare(&self, prompt: &str) -> Result<ComparisonResult, String> {
        let t0 = Instant::now();

        let (bonsai_res, ref_res) = tokio::join!(
            self.infer(self.bonsai_lora.as_deref(), prompt),
            self.infer(None, prompt),
        );

        let mut bonsai = bonsai_res.map_err(|e| format!("BonsAI: {e}"))?;
        let mut reference = ref_res.map_err(|e| format!("Reference: {e}"))?;
        let half_ms = t0.elapsed().as_millis() as u64 / 2;
        bonsai.latency_ms = half_ms;
        reference.latency_ms = half_ms;

        let intent_match = bonsai.intent == reference.intent;
        let overlap = bonsai
            .tools
            .iter()
            .filter(|t| reference.tools.contains(t))
            .count();
        let tool_overlap_pct = if reference.tools.is_empty() && bonsai.tools.is_empty() {
            100.0
        } else if reference.tools.is_empty() {
            0.0
        } else {
            overlap as f64 / reference.tools.len() as f64 * 100.0
        };

        let mut gaps = Vec::new();
        if !intent_match {
            gaps.push(GapDetail {
                gap_type: "intent_mismatch".into(),
                description: format!(
                    "BonsAI: {:?}, Reference: {:?}",
                    bonsai.intent, reference.intent
                ),
            });
        }
        for tool in &reference.tools {
            if !bonsai.tools.contains(tool) {
                gaps.push(GapDetail {
                    gap_type: "missing_tool".into(),
                    description: format!("BonsAI should have used '{tool}'"),
                });
            }
        }

        Ok(ComparisonResult {
            prompt: prompt.to_string(),
            intent_match,
            tool_overlap_pct,
            bonsai_tools: bonsai.tools.clone(),
            reference_tools: reference.tools.clone(),
            bonsai_confidence: bonsai.confidence,
            gaps,
        })
    }

    async fn infer(&self, lora_path: Option<&str>, prompt: &str) -> Result<ParsedModelOutput, String> {
        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.server.port());
        let mut body = json!({
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.3,
            "max_tokens": 512,
        });
        if let Some(lora) = lora_path {
            if !lora.is_empty() {
                body["lora"] = json!(lora);
            }
        }

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP: {e}"))?;

        let data: serde_json::Value = resp.json().await.map_err(|e| format!("JSON: {e}"))?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(parse_bonsai_output(&content))
    }
}

// ── Session Manager ───────────────────────────────────────────────────────────

pub struct SessionManager {
    server: RwLock<Option<Arc<SharedServer>>>,
    config: RwLock<Option<DualSessionConfig>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            server: RwLock::new(None),
            config: RwLock::new(None),
        }
    }

    pub async fn ensure_session(
        &self,
        config: DualSessionConfig,
    ) -> Result<Arc<SharedServer>, String> {
        // Reuse existing server if base model and GPU layers match
        {
            let cfg_guard = self.config.read().await;
            if let Some(cfg) = cfg_guard.as_ref() {
                if cfg.base_model_path == config.base_model_path
                    && cfg.gpu_layers == config.gpu_layers
                {
                    if let Some(srv) = self.server.read().await.as_ref() {
                        return Ok(srv.clone());
                    }
                }
            }
        }

        // Shutdown old server before starting new one
        if let Some(old) = self.server.write().await.take() {
            old.shutdown().await;
        }

        let mut lora_paths = Vec::new();
        if let Some(ref p) = config.bonsai_lora_path {
            lora_paths.push(p.clone());
        }
        if let Some(ref p) = config.reference_lora_path {
            if !lora_paths.contains(p) {
                lora_paths.push(p.clone());
            }
        }

        let server = Arc::new(
            SharedServer::launch(
                &config.base_model_path,
                &lora_paths,
                config.gpu_layers,
                config.context_size,
            )
            .await?,
        );

        *self.server.write().await = Some(server.clone());
        *self.config.write().await = Some(config);
        Ok(server)
    }

    pub async fn shutdown(&self) {
        if let Some(srv) = self.server.write().await.take() {
            srv.shutdown().await;
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn find_free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .map(|l| l.local_addr().unwrap().port())
        .unwrap_or(11440)
}

fn find_llama_server() -> Result<String, String> {
    if let Ok(path) = std::env::var("LLAMA_SERVER_PATH") {
        if Path::new(&path).exists() {
            return Ok(path);
        }
    }
    let candidates: Vec<PathBuf> = vec![
        dirs::data_dir()
            .unwrap_or_default()
            .join("com.bonsai.workspace")
            .join("sidecars")
            .join("llama-server.exe"),
        PathBuf::from("sidecars").join("llama-server.exe"),
        PathBuf::from("llama-server.exe"),
        PathBuf::from("llama-server"),
    ];
    for path in &candidates {
        if path.exists() {
            return Ok(path.to_string_lossy().into_owned());
        }
    }
    Err("llama-server not found. Set LLAMA_SERVER_PATH env var or install it.".into())
}
