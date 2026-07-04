use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LauncherEvent {
    AppStarted(String),
    AppStopped(String),
    SystemHealthChanged,
}

pub struct EventBus;

impl EventBus {
    pub fn new() -> Self {
        Self
    }

    pub async fn publish(&self, _event: LauncherEvent) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus() {
        let bus = EventBus::new();
        bus.publish(LauncherEvent::SystemHealthChanged)
            .await
            .unwrap();
    }
}
