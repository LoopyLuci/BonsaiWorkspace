use crate::error::BonsaiError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonsaiPlan {
    pub intent: String,
    pub reasoning: String,
    pub plan: Vec<PlanStep>,
    pub final_response: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub request: String,
    pub plan_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonsaiResponse {
    pub plan: BonsaiPlan,
    pub tool_results: Vec<String>,
}

/// Lightweight in-memory store; replaces BonsaiMemory until a full RAG
/// implementation is wired in.
pub struct CoreMemory {
    entries: RwLock<Vec<MemoryEntry>>,
    persist_path: Option<PathBuf>,
}

impl CoreMemory {
    pub fn new(persist_path: Option<PathBuf>) -> Self {
        let entries = if let Some(ref p) = persist_path {
            load_jsonl(p).unwrap_or_default()
        } else {
            Vec::new()
        };
        Self { entries: RwLock::new(entries), persist_path }
    }

    pub async fn search(&self, _query: &str, limit: usize) -> Result<Vec<MemoryEntry>, BonsaiError> {
        let entries = self.entries.read().await;
        Ok(entries.iter().rev().take(limit).cloned().collect())
    }

    pub async fn record(&self, request: &str, plan: &BonsaiPlan, _result: &BonsaiResponse) -> Result<(), BonsaiError> {
        let mut entries = self.entries.write().await;
        entries.push(MemoryEntry {
            request: request.to_string(),
            plan_summary: plan.intent.clone(),
        });
        // Truncate beyond 10k to bound memory usage
        if entries.len() > 10_000 {
            let excess = entries.len() - 10_000;
            entries.drain(0..excess);
        }
        // Auto-save every 100 entries
        if entries.len() % 100 == 0 {
            if let Some(ref p) = self.persist_path {
                let _ = save_jsonl(p, &entries);
            }
        }
        Ok(())
    }

    pub async fn save_to_disk(&self) {
        if let Some(ref p) = self.persist_path {
            let entries = self.entries.read().await;
            let _ = save_jsonl(p, &entries);
        }
    }

    pub async fn count(&self) -> usize {
        self.entries.read().await.len()
    }
}

fn load_jsonl(path: &PathBuf) -> Option<Vec<MemoryEntry>> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(content.lines().filter_map(|l| serde_json::from_str(l).ok()).collect())
}

fn save_jsonl(path: &PathBuf, entries: &[MemoryEntry]) -> std::io::Result<()> {
    let mut out = String::new();
    for e in entries {
        if let Ok(s) = serde_json::to_string(e) {
            out.push_str(&s);
            out.push('\n');
        }
    }
    crate::atomic_write(path, out.as_bytes())
}

/// Keyword-based fallback for high-confidence, zero-latency responses.
pub struct KeywordRouter {
    rules: Vec<(Vec<&'static str>, &'static str)>,
}

impl KeywordRouter {
    pub fn new() -> Self {
        Self {
            rules: vec![
                (vec!["hello", "hi", "hey"], "chat"),
                (vec!["time", "date", "clock"], "tool_use"),
                (vec!["weather"], "tool_use"),
                (vec!["list files", "ls "], "tool_use"),
            ],
        }
    }

    pub fn try_high_confidence(&self, request: &str) -> Option<BonsaiResponse> {
        let lower = request.to_lowercase();
        for (keywords, intent) in &self.rules {
            if keywords.iter().any(|k| lower.contains(k)) {
                // Only bypass for simple greetings where no tool is needed
                if *intent == "chat" {
                    let plan = BonsaiPlan {
                        intent: "chat".into(),
                        reasoning: "Keyword match".into(),
                        plan: vec![],
                        final_response: Some("Hello! How can I help?".into()),
                        confidence: 0.99,
                    };
                    return Some(BonsaiResponse { plan, tool_results: vec![] });
                }
            }
        }
        None
    }
}

pub struct BonsaiCore {
    adapter_path: RwLock<Option<PathBuf>>,
    inference_url: String,
    pub memory: CoreMemory,
    pub curator: crate::data_curator::DataCurator,
    prompt_template: String,
    fallback_router: KeywordRouter,
    allowed_commands: Vec<String>,
    workspace_root: PathBuf,
    /// Shadow mode: log model plan but execute keyword/fallback plan only.
    pub shadow_mode: RwLock<bool>,
    latency_sum_ms: RwLock<f64>,
    latency_count: RwLock<u64>,
    fallback_count: RwLock<u64>,
    request_count: RwLock<u64>,
}

impl BonsaiCore {
    pub fn new(
        adapter_path: Option<PathBuf>,
        inference_url: String,
        memory: CoreMemory,
        curator: crate::data_curator::DataCurator,
        prompt_template: String,
        workspace_root: PathBuf,
        shadow_mode: bool,
    ) -> Self {
        Self {
            adapter_path: RwLock::new(adapter_path),
            inference_url,
            memory,
            curator,
            prompt_template,
            fallback_router: KeywordRouter::new(),
            allowed_commands: vec![
                "python".into(), "py".into(), "ls".into(), "cat".into(), "echo".into(),
            ],
            workspace_root,
            shadow_mode: RwLock::new(shadow_mode),
            latency_sum_ms: RwLock::new(0.0),
            latency_count: RwLock::new(0),
            fallback_count: RwLock::new(0),
            request_count: RwLock::new(0),
        }
    }

    pub async fn process(
        &self,
        request: &str,
        history: &[ChatMessage],
    ) -> Result<BonsaiResponse, BonsaiError> {
        let start = std::time::Instant::now();
        *self.request_count.write().await += 1;

        // 1. High-confidence keyword bypass
        if let Some(resp) = self.fallback_router.try_high_confidence(request) {
            *self.fallback_count.write().await += 1;
            return Ok(resp);
        }

        // 2. Retrieve memory context
        let mem_context = self.memory.search(request, 3).await?;

        // 3. Build prompt
        let prompt = self.build_prompt(request, history, &mem_context);

        // 4. Infer plan
        let plan = self.infer_plan(&prompt).await?;

        // Shadow mode: log model plan, return keyword fallback instead
        if *self.shadow_mode.read().await {
            tracing::info!("[shadow] model plan: {:?}", plan);
            if let Some(resp) = self.fallback_router.try_high_confidence(request) {
                return Ok(resp);
            }
        }

        // 5. Policy check
        self.policy_validate(&plan)?;

        // 6. Execute plan
        let result = self.execute_plan(&plan).await?;

        // 7. Record in memory
        self.memory.record(request, &plan, &result).await?;

        // 7b. Curate as training example (non-blocking; failures are silently dropped)
        self.curator.ingest(request, &plan, &result).await;

        // 8. Update latency stats
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        *self.latency_sum_ms.write().await += elapsed;
        *self.latency_count.write().await += 1;

        Ok(result)
    }

    async fn infer_plan(&self, prompt: &str) -> Result<BonsaiPlan, BonsaiError> {
        let mut body = serde_json::json!({
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.2,
            "max_tokens": 512,
            "stop": ["</s>", "```"],
        });
        // Attach LoRA adapter path only when one is configured
        if let Some(ref path) = *self.adapter_path.read().await {
            body["lora_path"] = serde_json::json!(path.to_string_lossy());
        }

        let client = reqwest::Client::new();
        let resp = client
            .post(&self.inference_url)
            .json(&body)
            .timeout(Duration::from_millis(1500))
            .send()
            .await
            .map_err(|e| BonsaiError::Network(e.to_string()))?;

        let text = resp.text().await.map_err(|e| BonsaiError::Network(e.to_string()))?;
        let parsed: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| BonsaiError::Serde(e.to_string()))?;
        let content = parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("");

        let json_str = extract_json(content)?;
        let plan: BonsaiPlan = serde_json::from_str(&json_str)
            .map_err(|_| BonsaiError::Internal("Invalid plan JSON from model".into()))?;
        Ok(plan)
    }

    fn build_prompt(&self, request: &str, _history: &[ChatMessage], memory: &[MemoryEntry]) -> String {
        let memory_str = if memory.is_empty() {
            "None".to_string()
        } else {
            memory
                .iter()
                .map(|e| format!("- {}: {}", e.request, e.plan_summary))
                .collect::<Vec<_>>()
                .join("\n")
        };
        self.prompt_template
            .replace("{request}", request)
            .replace("{memory}", &memory_str)
    }

    fn policy_validate(&self, plan: &BonsaiPlan) -> Result<(), BonsaiError> {
        for step in &plan.plan {
            match step.tool.as_str() {
                "run_command" => {
                    let cmd = step.args["command"].as_str().unwrap_or("");
                    if !self.allowed_commands.iter().any(|c| cmd.starts_with(c.as_str())) {
                        return Err(BonsaiError::Tool(format!("Command not allowed: {cmd}")));
                    }
                }
                "write_file" | "read_file" => {
                    let path = step.args["path"].as_str().unwrap_or("");
                    let full = self.workspace_root.join(path);
                    if !full.starts_with(&self.workspace_root) {
                        return Err(BonsaiError::Tool("Path escape attempted".into()));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn execute_plan(&self, plan: &BonsaiPlan) -> Result<BonsaiResponse, BonsaiError> {
        let mut results = Vec::new();
        for step in &plan.plan {
            let result = crate::tools::execute_built_in(
                &step.tool,
                &step.args,
                Some(self.workspace_root.to_string_lossy().as_ref()),
            )
            .await
            .unwrap_or_else(|e| format!("tool error: {e}"));
            results.push(result);
        }
        Ok(BonsaiResponse { plan: plan.clone(), tool_results: results })
    }

    pub async fn adapter_loaded(&self) -> bool {
        self.adapter_path.read().await.is_some()
    }

    pub async fn avg_latency_ms(&self) -> f64 {
        let count = *self.latency_count.read().await;
        if count == 0 {
            return 0.0;
        }
        *self.latency_sum_ms.read().await / count as f64
    }

    pub async fn set_shadow_mode(&self, enabled: bool) {
        *self.shadow_mode.write().await = enabled;
    }

    pub async fn set_adapter_path(&self, path: Option<PathBuf>) {
        *self.adapter_path.write().await = path;
    }

    /// Hot-swap adapter after training; alias for set_adapter_path(Some(path)).
    pub fn load_adapter(&self, path: &PathBuf) {
        // Blocking write — only called from sync Trainer context
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            handle.block_on(async {
                *self.adapter_path.write().await = Some(path.clone());
            });
        }
    }

    pub async fn fallback_rate(&self) -> f64 {
        let total = *self.request_count.read().await;
        if total == 0 {
            return 0.0;
        }
        *self.fallback_count.read().await as f64 / total as f64
    }
}

fn extract_json(s: &str) -> Result<String, BonsaiError> {
    let s = s.trim().trim_start_matches("```json").trim_end_matches("```").trim();
    let start = s
        .find('{')
        .ok_or_else(|| BonsaiError::Internal("No JSON object found in model output".into()))?;
    let end = s
        .rfind('}')
        .ok_or_else(|| BonsaiError::Internal("No JSON object end found in model output".into()))?;
    Ok(s[start..=end].to_string())
}
