use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ComplianceManager {
    frameworks: Arc<DashMap<String, ComplianceFramework>>,
}

#[derive(Debug, Clone)]
pub struct ComplianceFramework {
    pub name: String,
    pub requirements: Vec<String>,
    pub compliance_score: f32,
}

impl ComplianceManager {
    pub fn new() -> Self {
        Self {
            frameworks: Arc::new(DashMap::new()),
        }
    }

    pub fn register_framework(&self, framework: ComplianceFramework) -> Result<()> {
        self.frameworks.insert(framework.name.clone(), framework);
        Ok(())
    }

    pub fn framework_count(&self) -> usize {
        self.frameworks.len()
    }
}

impl Default for ComplianceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance() {
        let mgr = ComplianceManager::new();
        let framework = ComplianceFramework {
            name: "SOC2".to_string(),
            requirements: vec!["encryption".to_string()],
            compliance_score: 0.85,
        };
        assert!(mgr.register_framework(framework).is_ok());
    }
}
