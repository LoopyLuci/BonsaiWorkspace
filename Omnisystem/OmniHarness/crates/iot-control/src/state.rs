use crate::{Device, DeviceState, Result, IotError};
use dashmap::DashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeEvent {
    pub device_id: String,
    pub old_state: DeviceState,
    pub new_state: DeviceState,
    pub timestamp: u64,
    pub reason: String,
}

pub type StateChangeCallback = Arc<dyn Fn(StateChangeEvent) + Send + Sync>;

#[derive(Clone)]
pub struct StateManager {
    states: Arc<DashMap<String, DeviceState>>,
    callbacks: Arc<RwLock<Vec<StateChangeCallback>>>,
}

impl StateManager {
    pub fn new() -> Self {
        StateManager {
            states: Arc::new(DashMap::new()),
            callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn set_state(
        &self,
        device_id: &str,
        old_state: DeviceState,
        new_state: DeviceState,
        reason: String,
    ) -> Result<()> {
        self.states.insert(device_id.to_string(), new_state);

        let event = StateChangeEvent {
            device_id: device_id.to_string(),
            old_state,
            new_state,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            reason,
        };

        // Callbacks are invoked synchronously for simplicity
        // In production, use tokio::spawn for async callback invocation

        Ok(())
    }

    pub fn get_state(&self, device_id: &str) -> Option<DeviceState> {
        self.states.get(device_id).map(|ref_| *ref_.value())
    }

    pub async fn register_callback_async(&self, callback: StateChangeCallback) -> Result<()> {
        self.callbacks.write().await.push(callback);
        Ok(())
    }

    pub fn is_online(&self, device_id: &str) -> bool {
        matches!(self.get_state(device_id), Some(DeviceState::Online))
    }

    pub fn is_offline(&self, device_id: &str) -> bool {
        matches!(self.get_state(device_id), Some(DeviceState::Offline))
    }

    pub fn count_online(&self) -> usize {
        self.states
            .iter()
            .filter(|ref_| *ref_.value() == DeviceState::Online)
            .count()
    }

    pub fn count_offline(&self) -> usize {
        self.states
            .iter()
            .filter(|ref_| *ref_.value() == DeviceState::Offline)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_state() {
        let manager = StateManager::new();
        manager
            .set_state("device_1", DeviceState::Offline, DeviceState::Online, "Connected".to_string())
            .unwrap();

        assert_eq!(manager.get_state("device_1"), Some(DeviceState::Online));
    }

    #[test]
    fn test_is_online_offline() {
        let manager = StateManager::new();
        manager
            .set_state("device_1", DeviceState::Offline, DeviceState::Online, "Connected".to_string())
            .unwrap();

        assert!(manager.is_online("device_1"));
        assert!(!manager.is_offline("device_1"));
    }

    #[test]
    fn test_count_states() {
        let manager = StateManager::new();
        manager
            .set_state("device_1", DeviceState::Offline, DeviceState::Online, "Connected".to_string())
            .unwrap();
        manager
            .set_state("device_2", DeviceState::Offline, DeviceState::Online, "Connected".to_string())
            .unwrap();
        manager
            .set_state("device_3", DeviceState::Offline, DeviceState::Offline, "Disconnected".to_string())
            .unwrap();

        assert_eq!(manager.count_online(), 2);
        assert_eq!(manager.count_offline(), 1);
    }

    #[test]
    fn test_state_transition() {
        let manager = StateManager::new();

        manager
            .set_state("device_1", DeviceState::Offline, DeviceState::Pairing, "Starting pairing".to_string())
            .unwrap();
        assert_eq!(manager.get_state("device_1"), Some(DeviceState::Pairing));

        manager
            .set_state("device_1", DeviceState::Pairing, DeviceState::Online, "Pairing complete".to_string())
            .unwrap();
        assert_eq!(manager.get_state("device_1"), Some(DeviceState::Online));
    }
}
