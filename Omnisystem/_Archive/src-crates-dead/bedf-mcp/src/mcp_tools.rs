use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub name: String,
    pub description: String,
}

pub struct MCPToolRegistry {
    tools: HashMap<String, String>,
}

impl MCPToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, description: String) {
        self.tools.insert(name, description);
    }

    pub fn list_tools(&self) -> Vec<MCPTool> {
        self.tools
            .iter()
            .map(|(name, desc)| MCPTool {
                name: name.clone(),
                description: desc.clone(),
            })
            .collect()
    }
}

impl Default for MCPToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let mut registry = MCPToolRegistry::new();
        registry.register("tool1".to_string(), "desc1".to_string());
        assert_eq!(registry.list_tools().len(), 1);
    }
}
