use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn subscriber_receives_emitted_events_in_order() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        bus.emit(Event::DeploymentStarted { deployment_id: "d1".to_string(), timestamp: Utc::now() })
            .await
            .unwrap();
        bus.emit(Event::DeploymentSucceeded { deployment_id: "d1".to_string(), timestamp: Utc::now() })
            .await
            .unwrap();

        match rx.recv().await.unwrap() {
            Event::DeploymentStarted { deployment_id, .. } => assert_eq!(deployment_id, "d1"),
            other => panic!("unexpected event: {other:?}"),
        }
        match rx.recv().await.unwrap() {
            Event::DeploymentSucceeded { deployment_id, .. } => assert_eq!(deployment_id, "d1"),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn multiple_subscribers_all_get_the_same_events() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        bus.emit(Event::ScaleUp { service_name: "web".to_string(), new_replicas: 5, timestamp: Utc::now() })
            .await
            .unwrap();

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert!(matches!(e1, Event::ScaleUp { new_replicas: 5, .. }));
        assert!(matches!(e2, Event::ScaleUp { new_replicas: 5, .. }));
    }

    #[tokio::test]
    async fn emit_without_subscribers_does_not_error() {
        let bus = EventBus::new();
        let result = bus
            .emit(Event::ContainerCrashed {
                container_id: "c1".to_string(),
                exit_code: Some(137),
                reason: "oom".to_string(),
                timestamp: Utc::now(),
            })
            .await;
        assert!(result.is_ok());
    }
}
