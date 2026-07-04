use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct Pipeline {
    id: String,
    name: String,
    stages: Arc<DashMap<u32, PipelineStage>>,
    execution_log: Arc<std::sync::Mutex<Vec<ExecutionRecord>>>,
}

#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub order: u32,
    pub stage_type: StageType,
    pub status: StageStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageType {
    Extract,
    Transform,
    Load,
    Validate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub timestamp: u64,
    pub stage: StageType,
    pub status: StageStatus,
    pub duration_ms: u64,
}

impl Pipeline {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            stages: Arc::new(DashMap::new()),
            execution_log: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn add_stage(&self, order: u32, stage_type: StageType) -> Result<()> {
        let stage = PipelineStage {
            order,
            stage_type,
            status: StageStatus::Pending,
        };
        self.stages.insert(order, stage);
        Ok(())
    }

    pub async fn execute(&self) -> Result<()> {
        let mut stages: Vec<_> = self.stages.iter().collect();
        stages.sort_by_key(|s| s.value().order);
        
        for stage in stages {
            let start = std::time::Instant::now();
            tracing::info!("Executing stage: {:?}", stage.value().stage_type);
            
            let duration = start.elapsed().as_millis() as u64;
            
            let mut log = self.execution_log.lock().unwrap();
            log.push(ExecutionRecord {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                stage: stage.value().stage_type,
                status: StageStatus::Completed,
                duration_ms: duration,
            });
        }
        
        Ok(())
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    pub fn execution_count(&self) -> usize {
        self.execution_log.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline() {
        let pipeline = Pipeline::new("p1".to_string(), "ETL Pipeline".to_string());
        pipeline.add_stage(1, StageType::Extract).unwrap();
        pipeline.add_stage(2, StageType::Transform).unwrap();
        pipeline.add_stage(3, StageType::Load).unwrap();
        
        assert_eq!(pipeline.stage_count(), 3);
        assert!(pipeline.execute().await.is_ok());
        assert!(pipeline.execution_count() > 0);
    }
}
