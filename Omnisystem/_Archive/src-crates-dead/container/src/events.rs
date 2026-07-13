use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    DeploymentStarted {
        deployment_id: String,
        timestamp: DateTime<Utc>,
    },
    DeploymentSucceeded {
        deployment_id: String,
        timestamp: DateTime<Utc>,
    },
    DeploymentFailed {
        deployment_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    ContainerStarted {
        container_id: String,
        image_hash: String,
        node_id: String,
        timestamp: DateTime<Utc>,
    },
    ContainerCrashed {
        container_id: String,
        exit_code: Option<i32>,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    ContainerOOMKilled {
        container_id: String,
        memory_limit_mib: u64,
        memory_used_mib: u64,
        timestamp: DateTime<Utc>,
    },
    ScaleUp {
        service_name: String,
        new_replicas: u32,
        timestamp: DateTime<Utc>,
    },
    ScaleDown {
        service_name: String,
        new_replicas: u32,
        timestamp: DateTime<Utc>,
    },
    ServiceRegistered {
        service_name: String,
        container_id: String,
        timestamp: DateTime<Utc>,
    },
    RollbackInitiated {
        container_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    HealthCheckFailed {
        container_id: String,
        probe_type: String,
        timestamp: DateTime<Utc>,
    },
}

pub struct EventBus {
    tx: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(10000);
        Self { tx }
    }

    pub async fn emit(&self, event: Event) -> crate::Result<()> {
        self.tx.send(event).ok();
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
