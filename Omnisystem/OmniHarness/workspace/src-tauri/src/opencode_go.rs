//! OpenCode Go — a subscription-tier model gateway from OpenCode Zen
//! (https://opencode.ai/docs/go/), reached over two different wire
//! protocols depending on the model:
//!
//!   - OpenAI chat-completions (`/v1/chat/completions`): DeepSeek, GLM,
//!     Kimi, MiMo.
//!   - Anthropic Messages (`/v1/messages`, `x-api-key` header instead of
//!     `Authorization: Bearer`, `content[].text` response shape): MiniMax,
//!     Qwen.
//!
//! The rest of this codebase's provider plumbing (`model_data_generator.rs`)
//! only ever calls the OpenAI-shaped endpoint, even for its "anthropic"
//! provider entry — this module is the first real Anthropic-Messages
//! client in the workspace backend.

use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const BASE_URL: &str = "https://opencode.ai/zen/go/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    OpenAiChat,
    AnthropicMessages,
}

/// Classifies a known OpenCode Go model id by which wire protocol it needs.
/// `/v1/models` returns the live catalog but (per the docs) doesn't
/// self-describe protocol, so this static table — seeded directly from the
/// documented split — is the fallback classifier. Unknown ids default to
/// `OpenAiChat` (the larger of the two families).
fn classify(model_id: &str) -> Protocol {
    let id = model_id.to_lowercase();
    if id.contains("minimax") || id.contains("qwen") {
        Protocol::AnthropicMessages
    } else {
        Protocol::OpenAiChat
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeGoModel {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub context_window: Option<u64>,
}

#[derive(Deserialize)]
struct ModelsListResponse {
    data: Vec<ModelsListEntry>,
}

#[derive(Deserialize)]
struct ModelsListEntry {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    context_length: Option<u64>,
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))
}

/// GET `/v1/models` — the live catalog, each entry classified by protocol.
pub async fn fetch_models(api_key: &str) -> Result<Vec<OpenCodeGoModel>, String> {
    let client = http_client()?;
    let resp = client
        .get(format!("{BASE_URL}/models"))
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| format!("OpenCode Go models request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("OpenCode Go returned {status}: {text}"));
    }

    let parsed: ModelsListResponse = resp
        .json()
        .await
        .map_err(|e| format!("failed to parse OpenCode Go models response: {e}"))?;

    Ok(parsed
        .data
        .into_iter()
        .map(|m| OpenCodeGoModel {
            protocol: classify(&m.id),
            name: m.name.unwrap_or_else(|| m.id.clone()),
            id: m.id,
            context_window: m.context_length,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Sends a chat message to the given model, dispatching to whichever wire
/// protocol that model needs, and returns the assistant's reply text.
pub async fn send_chat(api_key: &str, model_id: &str, messages: Vec<ChatMessage>) -> Result<String, String> {
    match classify(model_id) {
        Protocol::OpenAiChat => send_openai_chat(api_key, model_id, messages).await,
        Protocol::AnthropicMessages => send_anthropic_messages(api_key, model_id, messages).await,
    }
}

async fn send_openai_chat(api_key: &str, model_id: &str, messages: Vec<ChatMessage>) -> Result<String, String> {
    let client = http_client()?;
    let body = serde_json::json!({
        "model": model_id,
        "messages": messages,
        "stream": false,
    });

    let resp = client
        .post(format!("{BASE_URL}/chat/completions"))
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenCode Go chat-completions request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("OpenCode Go returned {status}: {text}"));
    }

    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("failed to parse OpenCode Go chat response: {e}"))?;

    v["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("unexpected OpenCode Go chat-completions response shape: {v}"))
}

async fn send_anthropic_messages(api_key: &str, model_id: &str, messages: Vec<ChatMessage>) -> Result<String, String> {
    let client = http_client()?;

    // Anthropic's Messages API splits out a top-level `system` prompt and
    // only allows `user`/`assistant` roles in the `messages` array.
    let (system, turns): (Vec<&ChatMessage>, Vec<&ChatMessage>) =
        messages.iter().partition(|m| m.role == "system");
    let system_prompt = system
        .into_iter()
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let body = serde_json::json!({
        "model": model_id,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": turns.into_iter().map(|m| serde_json::json!({
            "role": m.role,
            "content": m.content,
        })).collect::<Vec<_>>(),
    });

    let resp = client
        .post(format!("{BASE_URL}/messages"))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenCode Go messages request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("OpenCode Go returned {status}: {text}"));
    }

    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("failed to parse OpenCode Go messages response: {e}"))?;

    v["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("unexpected OpenCode Go messages response shape: {v}"))
}
