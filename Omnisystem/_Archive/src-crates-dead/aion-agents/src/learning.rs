use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct LearningEngine {
    knowledge_base: Arc<DashMap<String, f32>>,
    experience_buffer: Arc<DashMap<String, Vec<f32>>>,
}

impl LearningEngine {
    pub fn new() -> Self {
        Self {
            knowledge_base: Arc::new(DashMap::new()),
            experience_buffer: Arc::new(DashMap::new()),
        }
    }

    pub fn learn(&self, key: String, value: f32) -> Result<()> {
        self.knowledge_base.insert(key, value);
        tracing::info!("Learning update recorded");
        Ok(())
    }

    pub fn recall(&self, key: &str) -> Option<f32> {
        self.knowledge_base.get(key).map(|ref_| *ref_.value())
    }

    pub fn store_experience(&self, experience_id: String, data: Vec<f32>) -> Result<()> {
        self.experience_buffer.insert(experience_id, data);
        Ok(())
    }

    pub fn knowledge_size(&self) -> usize {
        self.knowledge_base.len()
    }
}

impl Default for LearningEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_learning_engine() {
        let engine = LearningEngine::new();
        assert!(engine.learn("k1".to_string(), 0.95).is_ok());
        assert_eq!(engine.recall("k1"), Some(0.95));
    }
}
