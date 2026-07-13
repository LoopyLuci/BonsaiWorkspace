use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct AlertManager {
    alerts: Arc<DashMap<String, Alert>>,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub id: String,
    pub threshold_exceeded: bool,
    pub current_value: f32,
    pub threshold: f32,
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            alerts: Arc::new(DashMap::new()),
        }
    }

    pub fn check_alert(&self, metric_value: f32, threshold: f32) -> Result<Alert> {
        let alert = Alert {
            id: uuid::Uuid::new_v4().to_string(),
            threshold_exceeded: metric_value > threshold,
            current_value: metric_value,
            threshold,
        };
        
        if alert.threshold_exceeded {
            tracing.info!("Alert triggered");
            self.alerts.insert(alert.id.clone(), alert.clone());
        }
        
        Ok(alert)
    }

    pub fn alert_count(&self) -> usize {
        self.alerts.len()
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert() {
        let manager = AlertManager::new();
        let alert = manager.check_alert(85.0, 80.0).unwrap();
        assert!(alert.threshold_exceeded);
    }
}
