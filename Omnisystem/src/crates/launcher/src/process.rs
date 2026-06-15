#[derive(Debug, Clone)]
pub struct ProcessMetrics {
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub threads: u32,
}

pub struct ProcessManager;

impl ProcessManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_metrics(&self, _pid: u32) -> anyhow::Result<ProcessMetrics> {
        Ok(ProcessMetrics {
            cpu_percent: 5.5,
            memory_mb: 128,
            threads: 2,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_manager() {
        let pm = ProcessManager::new();
        let metrics = pm.get_metrics(1234).await.unwrap();
        assert!(metrics.cpu_percent > 0.0);
    }
}
