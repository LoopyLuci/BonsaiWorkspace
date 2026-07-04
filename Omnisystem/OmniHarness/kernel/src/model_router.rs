use anyhow::{anyhow, Result};
use dashmap::DashMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tracing::{info, warn};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role:        String,
    pub content:     String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name:        Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id:        String,
    pub name:      String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name:        String,
    pub description: String,
    pub schema:      String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model_id:    String,
    pub messages:    Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens:  i32,
    pub system:      Option<String>,
    pub tools:       Vec<ToolDef>,
    pub session_id:  String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content:       String,
    pub model_used:    String,
    pub finish_reason: String,
    pub input_tokens:  i32,
    pub output_tokens: i32,
    pub tool_calls:    Vec<ToolCall>,
    pub latency_ms:    f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id:              String,
    pub provider:        String,
    pub display_name:    String,
    pub context_window:  i32,
    pub supports_tools:  bool,
    pub supports_vision: bool,
    pub available:       bool,
}

#[derive(Debug, Clone)]
pub struct ModelBackend {
    pub provider: String,
    pub api_key:  String,
    pub base_url: String,
    pub params:   std::collections::HashMap<String, String>,
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub struct ModelRegistry {
    backends: DashMap<String, ModelBackend>,
    client:   Client,
}

impl ModelRegistry {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .pool_max_idle_per_host(10)
            .build()
            .expect("reqwest client");
        Self { backends: DashMap::new(), client }
    }

    pub fn register(&self, provider: &str, backend: ModelBackend) {
        info!("[ModelRegistry] Registered provider: {}", provider);
        self.backends.insert(provider.to_string(), backend);
    }

    pub fn register_from_env(&self) {
        let env_map = [
            ("ANTHROPIC_API_KEY",  "anthropic",  "https://api.anthropic.com"),
            ("OPENAI_API_KEY",     "openai",     "https://api.openai.com"),
            ("COHERE_API_KEY",     "cohere",     "https://api.cohere.ai"),
            ("MISTRAL_API_KEY",    "mistral",    "https://api.mistral.ai"),
            ("GOOGLE_API_KEY",     "google",     "https://generativelanguage.googleapis.com"),
            ("GROQ_API_KEY",       "groq",       "https://api.groq.com"),
            ("OPENROUTER_API_KEY", "openrouter", "https://openrouter.ai"),
            ("TOGETHER_API_KEY",   "together",   "https://api.together.xyz"),
            ("FIREWORKS_API_KEY",  "fireworks",  "https://api.fireworks.ai"),
        ];
        for (env_var, provider, base_url) in env_map {
            if let Ok(key) = std::env::var(env_var) {
                if !key.is_empty() {
                    self.register(provider, ModelBackend {
                        provider: provider.to_string(),
                        api_key:  key,
                        base_url: base_url.to_string(),
                        params:   Default::default(),
                    });
                }
            }
        }
        // Ollama: always available locally (no key needed)
        let ollama_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        self.register("ollama", ModelBackend {
            provider: "ollama".to_string(),
            api_key:  String::new(),
            base_url: ollama_url,
            params:   Default::default(),
        });
    }

    pub fn len(&self) -> usize { self.backends.len() }

    pub fn list_backends(&self) -> Vec<String> {
        self.backends.iter().map(|e| e.key().clone()).collect()
    }

    // ── Route chat request to appropriate provider ─────────────────

    pub async fn route_chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let provider = self.infer_provider(&req.model_id);
        let backend = self.backends.get(&provider)
            .ok_or_else(|| anyhow!("No backend registered for provider '{}'", provider))?
            .clone();

        let t0 = Instant::now();
        let resp = match provider.as_str() {
            "anthropic"  => self.call_anthropic(&backend, &req).await,
            "openai"     => self.call_openai_compat(&backend, "/v1/chat/completions", &req).await,
            "groq"       => self.call_openai_compat(&backend, "/openai/v1/chat/completions", &req).await,
            "mistral"    => self.call_openai_compat(&backend, "/v1/chat/completions", &req).await,
            "openrouter" => self.call_openai_compat(&backend, "/api/v1/chat/completions", &req).await,
            "together"   => self.call_openai_compat(&backend, "/v1/chat/completions", &req).await,
            "fireworks"  => self.call_openai_compat(&backend, "/inference/v1/chat/completions", &req).await,
            "ollama"     => self.call_ollama(&backend, &req).await,
            "cohere"     => self.call_cohere(&backend, &req).await,
            "google"     => self.call_google(&backend, &req).await,
            other        => Err(anyhow!("Unknown provider: {}", other)),
        };

        resp.map(|mut r| {
            r.latency_ms = t0.elapsed().as_secs_f64() * 1000.0;
            r
        })
    }

    pub async fn health_check(&self, provider: &str) -> Result<f64> {
        let backend = self.backends.get(provider)
            .ok_or_else(|| anyhow!("Provider '{}' not registered", provider))?
            .clone();
        let t0 = Instant::now();
        match provider {
            "anthropic" => {
                self.client
                    .get(format!("{}/v1/models", backend.base_url))
                    .header("x-api-key", &backend.api_key)
                    .header("anthropic-version", "2023-06-01")
                    .send().await?.error_for_status()?;
            }
            "ollama" => {
                self.client
                    .get(format!("{}/api/tags", backend.base_url))
                    .send().await?.error_for_status()?;
            }
            _ => {
                self.client
                    .get(format!("{}/v1/models", backend.base_url))
                    .bearer_auth(&backend.api_key)
                    .send().await?.error_for_status()?;
            }
        }
        Ok(t0.elapsed().as_secs_f64() * 1000.0)
    }

    pub fn list_known_models(&self) -> Vec<ModelInfo> {
        let mut models = Vec::new();
        if self.backends.contains_key("anthropic") {
            for (id, name, ctx) in [
                ("claude-sonnet-4-6",       "Claude Sonnet 4.6",       200_000),
                ("claude-opus-4-8",         "Claude Opus 4.8",         200_000),
                ("claude-haiku-4-5-20251001","Claude Haiku 4.5",       200_000),
                ("claude-sonnet-5",         "Claude Sonnet 5",         200_000),
                ("claude-fable-5",          "Claude Fable 5",          200_000),
            ] {
                models.push(ModelInfo {
                    id: id.to_string(), provider: "anthropic".to_string(),
                    display_name: name.to_string(), context_window: ctx,
                    supports_tools: true, supports_vision: true, available: true,
                });
            }
        }
        if self.backends.contains_key("openai") {
            for (id, name, ctx) in [
                ("gpt-4o",        "GPT-4o",       128_000),
                ("gpt-4o-mini",   "GPT-4o Mini",  128_000),
                ("o1",            "o1",            200_000),
                ("o3-mini",       "o3-mini",       200_000),
                ("gpt-4-turbo",   "GPT-4 Turbo",  128_000),
            ] {
                models.push(ModelInfo {
                    id: id.to_string(), provider: "openai".to_string(),
                    display_name: name.to_string(), context_window: ctx,
                    supports_tools: true, supports_vision: true, available: true,
                });
            }
        }
        if self.backends.contains_key("ollama") {
            models.push(ModelInfo {
                id: "llama3.2".to_string(), provider: "ollama".to_string(),
                display_name: "Llama 3.2 (Local)".to_string(), context_window: 128_000,
                supports_tools: false, supports_vision: false, available: true,
            });
        }
        if self.backends.contains_key("groq") {
            for (id, name) in [
                ("llama-3.3-70b-versatile", "Llama 3.3 70B (Groq)"),
                ("mixtral-8x7b-32768",      "Mixtral 8x7B (Groq)"),
            ] {
                models.push(ModelInfo {
                    id: id.to_string(), provider: "groq".to_string(),
                    display_name: name.to_string(), context_window: 32_768,
                    supports_tools: true, supports_vision: false, available: true,
                });
            }
        }
        models
    }

    // ── Provider-specific callers ─────────────────────────────────

    async fn call_anthropic(&self, b: &ModelBackend, req: &ChatRequest) -> Result<ChatResponse> {
        let mut body = json!({
            "model":      req.model_id,
            "max_tokens": req.max_tokens,
            "messages":   self.convert_messages_anthropic(&req.messages),
        });
        if req.temperature > 0.0 {
            body["temperature"] = json!(req.temperature);
        }
        if let Some(sys) = &req.system {
            body["system"] = json!(sys);
        }
        if !req.tools.is_empty() {
            let tools: Vec<Value> = req.tools.iter().map(|t| json!({
                "name":        t.name,
                "description": t.description,
                "input_schema": serde_json::from_str::<Value>(&t.schema).unwrap_or(json!({})),
            })).collect();
            body["tools"] = json!(tools);
        }

        let resp = self.client
            .post(format!("{}/v1/messages", b.base_url))
            .header("x-api-key", &b.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send().await?
            .error_for_status()?
            .json::<Value>().await?;

        let mut content = String::new();
        let mut tool_calls = Vec::new();
        if let Some(arr) = resp["content"].as_array() {
            for block in arr {
                match block["type"].as_str() {
                    Some("text") => {
                        content.push_str(block["text"].as_str().unwrap_or(""));
                    }
                    Some("tool_use") => {
                        tool_calls.push(ToolCall {
                            id:        block["id"].as_str().unwrap_or("").to_string(),
                            name:      block["name"].as_str().unwrap_or("").to_string(),
                            arguments: serde_json::to_string(&block["input"]).unwrap_or_default(),
                        });
                    }
                    _ => {}
                }
            }
        }

        Ok(ChatResponse {
            content,
            model_used:    resp["model"].as_str().unwrap_or(&req.model_id).to_string(),
            finish_reason: resp["stop_reason"].as_str().unwrap_or("stop").to_string(),
            input_tokens:  resp["usage"]["input_tokens"].as_i64().unwrap_or(0) as i32,
            output_tokens: resp["usage"]["output_tokens"].as_i64().unwrap_or(0) as i32,
            tool_calls,
            latency_ms:    0.0,
        })
    }

    async fn call_openai_compat(&self, b: &ModelBackend, path: &str, req: &ChatRequest) -> Result<ChatResponse> {
        let mut messages = Vec::new();
        if let Some(sys) = &req.system {
            messages.push(json!({"role": "system", "content": sys}));
        }
        for m in &req.messages {
            messages.push(json!({"role": m.role, "content": m.content}));
        }

        let mut body = json!({
            "model":       req.model_id,
            "messages":    messages,
            "max_tokens":  req.max_tokens,
            "temperature": req.temperature,
        });
        if !req.tools.is_empty() {
            let functions: Vec<Value> = req.tools.iter().map(|t| json!({
                "type": "function",
                "function": {
                    "name":        t.name,
                    "description": t.description,
                    "parameters":  serde_json::from_str::<Value>(&t.schema).unwrap_or(json!({})),
                }
            })).collect();
            body["tools"] = json!(functions);
        }

        let mut builder = self.client
            .post(format!("{}{}", b.base_url, path))
            .header("content-type", "application/json")
            .json(&body);
        if !b.api_key.is_empty() {
            builder = builder.bearer_auth(&b.api_key);
        }
        // OpenRouter extra headers
        if b.provider == "openrouter" {
            builder = builder
                .header("HTTP-Referer", "https://omnisystem.dev")
                .header("X-Title", "OmniHarness");
        }

        let resp = builder.send().await?.error_for_status()?.json::<Value>().await?;

        let choice = &resp["choices"][0];
        let msg    = &choice["message"];
        let content = msg["content"].as_str().unwrap_or("").to_string();
        let mut tool_calls = Vec::new();
        if let Some(tcs) = msg["tool_calls"].as_array() {
            for tc in tcs {
                tool_calls.push(ToolCall {
                    id:        tc["id"].as_str().unwrap_or("").to_string(),
                    name:      tc["function"]["name"].as_str().unwrap_or("").to_string(),
                    arguments: tc["function"]["arguments"].as_str().unwrap_or("{}").to_string(),
                });
            }
        }

        Ok(ChatResponse {
            content,
            model_used:    resp["model"].as_str().unwrap_or(&req.model_id).to_string(),
            finish_reason: choice["finish_reason"].as_str().unwrap_or("stop").to_string(),
            input_tokens:  resp["usage"]["prompt_tokens"].as_i64().unwrap_or(0) as i32,
            output_tokens: resp["usage"]["completion_tokens"].as_i64().unwrap_or(0) as i32,
            tool_calls,
            latency_ms:    0.0,
        })
    }

    async fn call_ollama(&self, b: &ModelBackend, req: &ChatRequest) -> Result<ChatResponse> {
        let messages: Vec<Value> = req.messages.iter().map(|m| {
            json!({"role": m.role, "content": m.content})
        }).collect();
        let body = json!({
            "model":    req.model_id,
            "messages": messages,
            "stream":   false,
            "options":  {"temperature": req.temperature},
        });
        let resp = self.client
            .post(format!("{}/api/chat", b.base_url))
            .json(&body)
            .send().await?.error_for_status()?.json::<Value>().await?;

        Ok(ChatResponse {
            content:       resp["message"]["content"].as_str().unwrap_or("").to_string(),
            model_used:    resp["model"].as_str().unwrap_or(&req.model_id).to_string(),
            finish_reason: if resp["done"].as_bool().unwrap_or(false) { "stop".into() } else { "length".into() },
            input_tokens:  resp["prompt_eval_count"].as_i64().unwrap_or(0) as i32,
            output_tokens: resp["eval_count"].as_i64().unwrap_or(0) as i32,
            tool_calls:    Vec::new(),
            latency_ms:    0.0,
        })
    }

    async fn call_cohere(&self, b: &ModelBackend, req: &ChatRequest) -> Result<ChatResponse> {
        let chat_history: Vec<Value> = req.messages.iter().rev().skip(1).rev().map(|m| {
            let role = if m.role == "user" { "USER" } else { "CHATBOT" };
            json!({"role": role, "message": m.content})
        }).collect();
        let last_msg = req.messages.last()
            .map(|m| m.content.clone())
            .unwrap_or_default();
        let body = json!({
            "model":        req.model_id,
            "message":      last_msg,
            "chat_history": chat_history,
            "temperature":  req.temperature,
            "max_tokens":   req.max_tokens,
        });
        let resp = self.client
            .post(format!("{}/v1/chat", b.base_url))
            .bearer_auth(&b.api_key)
            .json(&body)
            .send().await?.error_for_status()?.json::<Value>().await?;

        Ok(ChatResponse {
            content:       resp["text"].as_str().unwrap_or("").to_string(),
            model_used:    resp["generation_id"].as_str().unwrap_or(&req.model_id).to_string(),
            finish_reason: resp["finish_reason"].as_str().unwrap_or("stop").to_string(),
            input_tokens:  resp["meta"]["tokens"]["input_tokens"].as_i64().unwrap_or(0) as i32,
            output_tokens: resp["meta"]["tokens"]["output_tokens"].as_i64().unwrap_or(0) as i32,
            tool_calls:    Vec::new(),
            latency_ms:    0.0,
        })
    }

    async fn call_google(&self, b: &ModelBackend, req: &ChatRequest) -> Result<ChatResponse> {
        let parts: Vec<Value> = req.messages.iter().map(|m| {
            let role = if m.role == "user" { "user" } else { "model" };
            json!({"role": role, "parts": [{"text": m.content}]})
        }).collect();
        let body = json!({
            "contents":         parts,
            "generationConfig": {
                "temperature":  req.temperature,
                "maxOutputTokens": req.max_tokens,
            },
        });
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            b.base_url, req.model_id, b.api_key
        );
        let resp = self.client.post(&url).json(&body)
            .send().await?.error_for_status()?.json::<Value>().await?;

        let content = resp["candidates"][0]["content"]["parts"][0]["text"]
            .as_str().unwrap_or("").to_string();
        let finish = resp["candidates"][0]["finishReason"]
            .as_str().unwrap_or("STOP").to_string();

        Ok(ChatResponse {
            content,
            model_used:    req.model_id.clone(),
            finish_reason: finish,
            input_tokens:  resp["usageMetadata"]["promptTokenCount"].as_i64().unwrap_or(0) as i32,
            output_tokens: resp["usageMetadata"]["candidatesTokenCount"].as_i64().unwrap_or(0) as i32,
            tool_calls:    Vec::new(),
            latency_ms:    0.0,
        })
    }

    // ── Helpers ───────────────────────────────────────────────────

    fn infer_provider(&self, model_id: &str) -> String {
        if model_id.starts_with("claude-")          { return "anthropic".to_string(); }
        if model_id.starts_with("gpt-") || model_id.starts_with("o1") || model_id.starts_with("o3") {
            return "openai".to_string();
        }
        if model_id.starts_with("gemini-")          { return "google".to_string(); }
        if model_id.starts_with("command-")         { return "cohere".to_string(); }
        if model_id.starts_with("mistral-") || model_id.starts_with("codestral-") {
            return "mistral".to_string();
        }
        if model_id.starts_with("llama-") || model_id.starts_with("mixtral-") || model_id.starts_with("gemma") {
            if self.backends.contains_key("groq") { return "groq".to_string(); }
        }
        if model_id.contains('/') {
            let provider = model_id.split('/').next().unwrap_or("openrouter");
            if self.backends.contains_key(provider) { return provider.to_string(); }
            return "openrouter".to_string();
        }
        // default to first registered backend
        self.backends.iter().next().map(|e| e.key().clone()).unwrap_or_else(|| "ollama".to_string())
    }

    fn convert_messages_anthropic(&self, msgs: &[ChatMessage]) -> Vec<Value> {
        msgs.iter().map(|m| json!({"role": m.role, "content": m.content})).collect()
    }
}
