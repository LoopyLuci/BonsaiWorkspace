use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BluebonnetManifest {
    pub model: String,
    pub version: String,
    pub quantization: String,
    pub system_prompt: String,
    pub parameters: InferenceParameters,
    pub tools: ToolConfig,
    pub security: SecurityConfig,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceParameters {
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
    pub repeat_penalty: f32,
    pub context_window: u32,
    pub max_tokens: u32,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub allowed: Vec<String>,
    pub require_capability: bool,
    pub execution_mode: ToolExecutionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolExecutionMode {
    Sandbox,
    Direct,
    AskPermission,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub allowed_hosts: Vec<String>,
    pub require_capability: String,
    pub max_requests_per_minute: u32,
    pub audit_log: bool,
}

impl Default for BluebonnetManifest {
    fn default() -> Self {
        Self {
            model: "llama-3-8b".into(),
            version: "1.0.0".into(),
            quantization: "q4_k_m".into(),
            system_prompt: "You are BonsAI, the assistant of Bonsai Workspace.".into(),
            parameters: InferenceParameters {
                temperature: 0.7,
                top_p: 0.9,
                top_k: 40,
                repeat_penalty: 1.1,
                context_window: 8192,
                max_tokens: 4096,
                stream: true,
            },
            tools: ToolConfig {
                allowed: vec![],
                require_capability: true,
                execution_mode: ToolExecutionMode::Sandbox,
            },
            security: SecurityConfig {
                allowed_hosts: vec![],
                require_capability: "ModelCap:inference".into(),
                max_requests_per_minute: 60,
                audit_log: true,
            },
            metadata: std::collections::HashMap::new(),
        }
    }
}

impl BluebonnetManifest {
    pub fn parse(blueprint: &str) -> Result<Self, anyhow::Error> {
        let manifest: BluebonnetManifest = toml::from_str(blueprint)?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<(), anyhow::Error> {
        if self.model.is_empty() {
            anyhow::bail!("Model name is required");
        }
        if self.version.is_empty() {
            anyhow::bail!("Version is required");
        }
        let valid_quantizations = [
            "q2_k", "q3_k_m", "q4_0", "q4_k_m", "q5_k_m",
            "q6_k", "q8_0", "f16", "f32",
        ];
        if !valid_quantizations.contains(&self.quantization.as_str()) {
            anyhow::bail!("Invalid quantization: {}", self.quantization);
        }
        if self.parameters.temperature < 0.0 || self.parameters.temperature > 2.0 {
            anyhow::bail!("Temperature must be between 0.0 and 2.0");
        }
        Ok(())
    }
}
