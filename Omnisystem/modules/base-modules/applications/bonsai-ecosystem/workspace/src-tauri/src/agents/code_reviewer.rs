use async_trait::async_trait;
use serde_json::json;

use crate::agent::{
    Agent, AgentCapability, AgentContext, AgentMessage, AgentMetadata, AgentOutput,
};
use crate::error::BonsaiError;

pub struct CodeReviewer;

// ── Inference ─────────────────────────────────────────────────────────────────

async fn call_model(model_url: &str, system: &str, user: &str) -> Result<String, BonsaiError> {
    let url = format!("{}/v1/chat/completions", model_url.trim_end_matches('/'));
    let body = json!({
        "messages": [
            { "role": "system", "content": system },
            { "role": "user",   "content": user   }
        ],
        "temperature": 0.3,
        "max_tokens": 2048,
        "stream": false
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| BonsaiError::Network(e.to_string()))?;

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| BonsaiError::Network(format!("model request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(BonsaiError::Network(format!(
            "model returned {status}: {text}"
        )));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| BonsaiError::Serde(e.to_string()))?;

    json["choices"][0]["message"]["content"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| BonsaiError::Internal("model response missing content".into()))
}

// ── Agent impl ────────────────────────────────────────────────────────────────

#[async_trait]
impl Agent for CodeReviewer {
    fn metadata(&self) -> AgentMetadata {
        AgentMetadata {
            id: "code-reviewer".into(),
            name: "Code Reviewer".into(),
            description: "Reviews code files and provides actionable suggestions.".into(),
            capabilities: vec![
                AgentCapability::TextGeneration,
                AgentCapability::CodeEditing,
            ],
        }
    }

    async fn handle_message(
        &self,
        ctx: AgentContext,
        msg: AgentMessage,
    ) -> Result<AgentOutput, BonsaiError> {
        let model_url = ctx.model_url.as_deref().ok_or_else(|| {
            BonsaiError::Orchestrator("No model slot is ready — load a model first".into())
        })?;

        // Optionally read a file supplied via metadata["file_path"]
        let file_content = msg
            .metadata
            .as_ref()
            .and_then(|m| m.get("file_path"))
            .and_then(|v| v.as_str())
            .map(|fp| {
                std::fs::read_to_string(fp)
                    .map(|c| format!("\n\nFile `{fp}`:\n```\n{c}\n```"))
                    .unwrap_or_else(|e| format!("\n\n(could not read `{fp}`: {e})"))
            })
            .unwrap_or_default();

        let system = "You are an expert code reviewer. Analyse the code carefully and provide:\n\
                      1. A brief summary of what the code does\n\
                      2. Issues found (bugs, security, performance, style)\n\
                      3. Specific improvement suggestions with corrected code snippets\n\
                      Be concise and actionable.";

        let user = format!("{}{}", msg.content, file_content);

        let review = call_model(model_url, system, &user).await?;

        Ok(AgentOutput {
            content: review,
            actions: vec![],
            metadata: None,
        })
    }

    async fn shutdown(&self) {}
}
