use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::ToolDefinition;
use anyhow::Result;

pub struct ToolRegistry {
    tools: RwLock<HashMap<String, ToolDefinition>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let reg = Self { tools: RwLock::new(HashMap::new()) };
        reg.register_defaults();
        reg
    }

    fn register_defaults(&self) {
        let defaults = vec![
            ToolDefinition {
                name: "read_file".into(),
                description: "Read a file".into(),
                input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
                output_schema: None,
                required_capabilities: vec!["FileCap:read".into()],
                execution_mode: crate::ToolExecutionMode::Direct,
                examples: vec![],
            },
            ToolDefinition {
                name: "write_file".into(),
                description: "Write a file".into(),
                input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}),
                output_schema: None,
                required_capabilities: vec!["FileCap:write".into()],
                execution_mode: crate::ToolExecutionMode::Direct,
                examples: vec![],
            },
        ];
        for tool in defaults {
            let _ = self.register(tool);
        }
    }

    pub async fn register(&self, tool: ToolDefinition) -> Result<()> {
        self.tools.write().await.insert(tool.name.clone(), tool);
        Ok(())
    }

    pub async fn get(&self, name: &str) -> Option<ToolDefinition> {
        self.tools.read().await.get(name).cloned()
    }

    pub async fn list_all(&self) -> Vec<ToolDefinition> {
        self.tools.read().await.values().cloned().collect()
    }
}
