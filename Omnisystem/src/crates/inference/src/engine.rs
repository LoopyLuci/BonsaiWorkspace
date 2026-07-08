use crate::{InferenceRequest, GenerationOutput, GenerationChunk, ChatMessage, ToolCall};
use crate::gpu::GpuManager;
use crate::tokenizer::Tokenizer;
use crate::types::{InferenceBackend, LlamaBackend};
use model_registry::registry::ModelRegistry;
use kv_cache::KVCacheStore;
use tool_registry_vault::registry::ToolRegistry;
use tool_registry_vault::executor::ToolExecutor;
use std::sync::Arc;
use anyhow::Result;

pub struct InferenceEngine {
    model_registry: Arc<ModelRegistry>,
    gpu_manager: GpuManager,
    kv_cache: Arc<KVCacheStore>,
    tool_registry: Arc<ToolRegistry>,
    tool_executor: Arc<ToolExecutor>,
    active_model: std::sync::RwLock<Option<ActiveModel>>,
}

struct ActiveModel {
    name: String,
    version: String,
    backend: Box<dyn InferenceBackend + Send + Sync>,
    tokenizer: Tokenizer,
    config: model_registry::manifest::BluebonnetManifest,
}

impl InferenceEngine {
    pub fn new(
        model_registry: Arc<ModelRegistry>,
        kv_cache: Arc<KVCacheStore>,
        tool_registry: Arc<ToolRegistry>,
        tool_executor: Arc<ToolExecutor>,
    ) -> Self {
        Self {
            model_registry,
            gpu_manager: GpuManager::new(),
            kv_cache,
            tool_registry,
            tool_executor,
            active_model: std::sync::RwLock::new(None),
        }
    }

    pub async fn load_model(&self, name: &str, version: &str) -> Result<()> {
        let crystal_path = self.model_registry.load_model(name, version).await?;
        let model_path = crystal_path.join("model.bin");
        let tokenizer_path = crystal_path.join("tokenizer.json");
        let config_path = crystal_path.join("config.json");

        let config_str = std::fs::read_to_string(&config_path)?;
        let manifest: model_registry::manifest::BluebonnetManifest =
            serde_json::from_str(&config_str)?;

        let gpu_layers = self.gpu_manager.detect_optimal_layers(&model_path)?;

        let backend = Box::new(LlamaBackend::new(&model_path, gpu_layers, manifest.parameters.context_window)?);
        let tokenizer = Tokenizer::load(&tokenizer_path)?;

        let mut active = self.active_model.write().unwrap();
        *active = Some(ActiveModel {
            name: name.to_string(),
            version: version.to_string(),
            backend,
            tokenizer,
            config: manifest,
        });
        Ok(())
    }

    pub fn generate(&self, request: &InferenceRequest) -> Result<GenerationOutput> {
        let active = self.active_model.read().unwrap();
        let model = active.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No model loaded"))?;
        let prompt = if let Some(messages) = &request.messages {
            self.build_chat_prompt(messages, &model.config.system_prompt)
        } else {
            request.prompt.clone()
        };
        let tokens = model.tokenizer.encode(&prompt);
        let output = model.backend.generate(&tokens, request.temperature, request.top_p, request.max_tokens)?;
        let text = model.tokenizer.decode(&output.tokens);
        let tool_calls = self.extract_tool_calls(&text);
        Ok(GenerationOutput {
            text,
            tool_calls,
            finish_reason: "stop".to_string(),
            usage: crate::Usage {
                prompt_tokens: tokens.len() as u32,
                completion_tokens: output.tokens.len() as u32,
                total_tokens: (tokens.len() + output.tokens.len()) as u32,
            },
        })
    }

    pub fn generate_stream(
        &self,
        request: &InferenceRequest,
        callback: Box<dyn Fn(GenerationChunk) + Send>,
    ) -> Result<()> {
        let active = self.active_model.read().unwrap();
        let model = active.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No model loaded"))?;
        let prompt = if let Some(messages) = &request.messages {
            self.build_chat_prompt(messages, &model.config.system_prompt)
        } else {
            request.prompt.clone()
        };
        let tokens = model.tokenizer.encode(&prompt);
        model.backend.generate_stream(&tokens, request.temperature, request.top_p, request.max_tokens, |token| {
            callback(GenerationChunk {
                token,
                finish_reason: None,
                tool_call: None,
            });
        })?;
        callback(GenerationChunk {
            token: String::new(),
            finish_reason: Some("stop".to_string()),
            tool_call: None,
        });
        Ok(())
    }

    fn build_chat_prompt(&self, messages: &[ChatMessage], system_prompt: &str) -> String {
        let mut prompt = format!("<|system|>\n{}\n", system_prompt);
        for msg in messages {
            match msg.role.as_str() {
                "user" => prompt.push_str(&format!("<|user|>\n{}\n", msg.content)),
                "assistant" => prompt.push_str(&format!("<|assistant|>\n{}\n", msg.content)),
                "tool" => prompt.push_str(&format!("<|tool|>\n{}\n", msg.content)),
                _ => {}
            }
        }
        prompt.push_str("<|assistant|>\n");
        prompt
    }

    fn extract_tool_calls(&self, text: &str) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        let re = regex::Regex::new(r"<tool_call>\s*(\{.*?\})\s*</tool_call>").unwrap();
        for cap in re.captures_iter(text) {
            if let Some(json_str) = cap.get(1) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str.as_str()) {
                    calls.push(ToolCall {
                        id: uuid::Uuid::new_v4().to_string(),
                        function_name: parsed["name"].as_str().unwrap_or("").to_string(),
                        arguments: parsed["arguments"].clone(),
                    });
                }
            }
        }
        calls
    }
}
