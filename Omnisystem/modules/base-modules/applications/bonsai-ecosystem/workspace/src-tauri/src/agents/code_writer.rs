use async_trait::async_trait;
use serde_json::json;

use crate::agent::{
    Agent, AgentAction, AgentCapability, AgentContext, AgentMessage, AgentMetadata, AgentOutput,
};
use crate::error::BonsaiError;

pub struct CodeWriter;

// ── Inference ─────────────────────────────────────────────────────────────────

/// POST a chat-completions request to the local llama-server slot.
async fn call_model(model_url: &str, prompt: &str) -> Result<String, BonsaiError> {
    let url = format!("{}/v1/chat/completions", model_url.trim_end_matches('/'));
    let body = json!({
        "messages": [
            {
                "role": "system",
                "content": "You are an expert software engineer. When asked to generate code, \
                             produce complete, working files. Wrap every file in a fenced code \
                             block with the file path as a comment on the first line inside \
                             the block, like:\n```language\n// path: src/example.rs\n<code>\n```"
            },
            { "role": "user", "content": prompt }
        ],
        "temperature": 0.2,
        "max_tokens": 4096,
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

// ── Code block extraction ─────────────────────────────────────────────────────

struct ExtractedFile {
    path: String,
    content: String,
}

/// Pull fenced code blocks from model output.
/// Recognises `// path: <file>` or `# path: <file>` as the first line.
fn extract_files(text: &str) -> Vec<ExtractedFile> {
    let mut files = Vec::new();
    let mut rest = text;

    while let Some(fence_start) = rest.find("```") {
        // Skip opening fence + optional language tag
        let after_fence = &rest[fence_start + 3..];
        let block_start = after_fence
            .find('\n')
            .map(|i| i + 1)
            .unwrap_or(after_fence.len());
        let block_body = &after_fence[block_start..];

        let fence_end = match block_body.find("\n```") {
            Some(i) => i,
            None => {
                rest = &rest[fence_start + 3..];
                continue;
            }
        };
        let block = &block_body[..fence_end];

        // Advance past this block
        rest = &block_body[fence_end + 4..];

        // Extract file path from first line comment
        let first_line = block.lines().next().unwrap_or("").trim();
        let path = if let Some(p) = first_line
            .strip_prefix("// path:")
            .or_else(|| first_line.strip_prefix("# path:"))
        {
            p.trim().to_owned()
        } else {
            continue; // no path annotation — skip
        };

        // Content is everything after the first line
        let content = block.lines().skip(1).collect::<Vec<_>>().join("\n");
        if !path.is_empty() && !content.trim().is_empty() {
            files.push(ExtractedFile { path, content });
        }
    }
    files
}

// ── Atomic write ──────────────────────────────────────────────────────────────

/// Write `content` to `path` atomically: write to a temp file then rename.
fn atomic_write(path: &std::path::Path, content: &str) -> Result<(), BonsaiError> {
    use std::io::Write;

    // Ensure parent directories exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| BonsaiError::Io(e.to_string()))?;
    }

    let tmp_path = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp_path)
            .map_err(|e| BonsaiError::Io(format!("create tmp {}: {e}", tmp_path.display())))?;
        f.write_all(content.as_bytes())
            .map_err(|e| BonsaiError::Io(format!("write tmp: {e}")))?;
        f.flush().map_err(|e| BonsaiError::Io(e.to_string()))?;
    }
    std::fs::rename(&tmp_path, path)
        .map_err(|e| BonsaiError::Io(format!("rename to {}: {e}", path.display())))?;

    Ok(())
}

// ── Agent impl ────────────────────────────────────────────────────────────────

#[async_trait]
impl Agent for CodeWriter {
    fn metadata(&self) -> AgentMetadata {
        AgentMetadata {
            id: "code-writer".into(),
            name: "Code Writer".into(),
            description: "Generates and writes code files based on user descriptions.".into(),
            capabilities: vec![
                AgentCapability::TextGeneration,
                AgentCapability::CodeEditing,
                AgentCapability::FileManipulation,
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

        // Call the model
        let raw = call_model(model_url, &msg.content).await?;

        // Parse and write all extracted files
        let extracted = extract_files(&raw);
        let mut actions = Vec::new();

        for file in &extracted {
            let path = std::path::Path::new(&file.path);
            match atomic_write(path, &file.content) {
                Ok(()) => {
                    tracing::info!("[code-writer] wrote {}", file.path);
                    actions.push(AgentAction {
                        kind: "write_file".into(),
                        payload: json!({
                            "path":  file.path,
                            "bytes": file.content.len(),
                        }),
                    });
                }
                Err(e) => {
                    tracing::warn!("[code-writer] failed to write {}: {e}", file.path);
                    actions.push(AgentAction {
                        kind: "write_file_error".into(),
                        payload: json!({
                            "path":  file.path,
                            "error": e.to_string(),
                        }),
                    });
                }
            }
        }

        let summary = if extracted.is_empty() {
            raw.clone()
        } else {
            format!(
                "Wrote {} file(s): {}",
                extracted.len(),
                extracted
                    .iter()
                    .map(|f| f.path.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        Ok(AgentOutput {
            content: summary,
            actions,
            metadata: Some(json!({ "raw_model_output": raw })),
        })
    }

    async fn shutdown(&self) {}
}
