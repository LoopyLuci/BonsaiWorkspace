use crate::{AppId, ModuleState, Result, AppManagerError};
use dashmap::DashMap;
use std::sync::Arc;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: ModuleState,
    pub to: ModuleState,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct ModuleLifecycleManager {
    states: Arc<DashMap<AppId, ModuleState>>,
    transitions: Arc<DashMap<AppId, Vec<StateTransition>>>,
}

impl ModuleLifecycleManager {
    pub fn new() -> Self {
        ModuleLifecycleManager {
            states: Arc::new(DashMap::new()),
            transitions: Arc::new(DashMap::new()),
        }
    }

    pub fn register_module(&self, app_id: AppId) -> Result<()> {
        self.states.insert(app_id.clone(), ModuleState::Discovered);
        self.transitions.insert(app_id.clone(), Vec::new());
        tracing::debug!("Registered module: {}", app_id);
        Ok(())
    }

    pub fn get_state(&self, app_id: &AppId) -> Result<ModuleState> {
        self.states
            .get(app_id)
            .map(|r| *r)
            .ok_or_else(|| AppManagerError::ModuleNotLoaded(app_id.to_string()))
    }

    pub fn transition(&self, app_id: &AppId, from: ModuleState, to: ModuleState) -> Result<()> {
        let current = self.get_state(app_id)?;

        if current != from {
            return Err(AppManagerError::InvalidStateTransition(format!(
                "Expected {}, got {}",
                from as u8, current as u8
            )));
        }

        if !Self::is_valid_transition(from, to) {
            return Err(AppManagerError::InvalidStateTransition(format!(
                "{} -> {}",
                from as u8, to as u8
            )));
        }

        self.states.insert(app_id.clone(), to);

        let transition = StateTransition {
            from,
            to,
            timestamp: Utc::now(),
        };

        if let Some(mut transitions) = self.transitions.get_mut(app_id) {
            transitions.push(transition);
        }

        tracing::debug!("Module {} transitioned: {:?} -> {:?}", app_id, from, to);

        Ok(())
    }

    fn is_valid_transition(from: ModuleState, to: ModuleState) -> bool {
        matches!(
            (from, to),
            (ModuleState::Discovered, ModuleState::Downloading)
                | (ModuleState::Downloading, ModuleState::Downloaded)
                | (ModuleState::Downloaded, ModuleState::Verifying)
                | (ModuleState::Verifying, ModuleState::Verified)
                | (ModuleState::Verified, ModuleState::Installing)
                | (ModuleState::Installing, ModuleState::Installed)
                | (ModuleState::Installed, ModuleState::Loading)
                | (ModuleState::Loading, ModuleState::Loaded)
                | (ModuleState::Loaded, ModuleState::Running)
                | (ModuleState::Running, ModuleState::Stopped)
                | (ModuleState::Stopped, ModuleState::Running)
                | (ModuleState::Loaded, ModuleState::Unloading)
                | (ModuleState::Running, ModuleState::Unloading)
                | (ModuleState::Unloading, ModuleState::Unloaded)
                | (ModuleState::Unloaded, ModuleState::Loading)
                | (_, ModuleState::Failed)
                | (_, ModuleState::Corrupted)
        )
    }

    pub fn get_transitions(&self, app_id: &AppId) -> Result<Vec<StateTransition>> {
        self.transitions
            .get(app_id)
            .map(|r| r.clone())
            .ok_or_else(|| AppManagerError::ModuleNotLoaded(app_id.to_string()))
    }

    pub async fn download(&self, app_id: &AppId) -> Result<()> {
        self.transition(app_id, ModuleState::Discovered, ModuleState::Downloading)?;
        self.transition(app_id, ModuleState::Downloading, ModuleState::Downloaded)?;
        Ok(())
    }

    pub async fn verify(&self, app_id: &AppId) -> Result<()> {
        let current = self.get_state(app_id)?;
        self.transition(app_id, current, ModuleState::Verifying)?;
        self.transition(app_id, ModuleState::Verifying, ModuleState::Verified)?;
        Ok(())
    }

    pub async fn install(&self, app_id: &AppId) -> Result<()> {
        let current = self.get_state(app_id)?;
        if current != ModuleState::Verified && current != ModuleState::Downloaded {
            return Err(AppManagerError::InvalidStateTransition(
                "Module not ready for installation".to_string(),
            ));
        }

        self.transition(app_id, current, ModuleState::Installing)?;
        self.transition(app_id, ModuleState::Installing, ModuleState::Installed)?;
        Ok(())
    }

    pub async fn load(&self, app_id: &AppId) -> Result<()> {
        let current = self.get_state(app_id)?;
        if current != ModuleState::Installed && current != ModuleState::Loaded {
            return Err(AppManagerError::InvalidStateTransition(
                "Module not ready for loading".to_string(),
            ));
        }

        self.transition(app_id, current, ModuleState::Loading)?;
        self.transition(app_id, ModuleState::Loading, ModuleState::Loaded)?;
        Ok(())
    }

    pub async fn start(&self, app_id: &AppId) -> Result<()> {
        let current = self.get_state(app_id)?;
        if current != ModuleState::Loaded {
            return Err(AppManagerError::InvalidStateTransition(
                "Module not loaded".to_string(),
            ));
        }

        self.transition(app_id, ModuleState::Loaded, ModuleState::Running)?;
        Ok(())
    }

    pub async fn stop(&self, app_id: &AppId) -> Result<()> {
        let current = self.get_state(app_id)?;
        if current != ModuleState::Running {
            return Err(AppManagerError::InvalidStateTransition(
                "Module not running".to_string(),
            ));
        }

        self.transition(app_id, ModuleState::Running, ModuleState::Stopped)?;
        Ok(())
    }

    pub async fn unload(&self, app_id: &AppId) -> Result<()> {
        let current = self.get_state(app_id)?;
        if !matches!(current, ModuleState::Loaded | ModuleState::Running | ModuleState::Stopped) {
            return Err(AppManagerError::InvalidStateTransition(
                "Module cannot be unloaded from this state".to_string(),
            ));
        }

        self.transition(app_id, current, ModuleState::Unloading)?;
        self.transition(app_id, ModuleState::Unloading, ModuleState::Unloaded)?;
        Ok(())
    }

    pub async fn mark_failed(&self, app_id: &AppId) -> Result<()> {
        self.states.insert(app_id.clone(), ModuleState::Failed);
        Ok(())
    }

    pub async fn mark_corrupted(&self, app_id: &AppId) -> Result<()> {
        self.states.insert(app_id.clone(), ModuleState::Corrupted);
        Ok(())
    }

    pub fn list_all_states(&self) -> Vec<(AppId, ModuleState)> {
        self.states
            .iter()
            .map(|r| (r.key().clone(), *r.value()))
            .collect()
    }

    pub fn count_by_state(&self, state: ModuleState) -> usize {
        self.states.iter().filter(|r| *r.value() == state).count()
    }
}

impl Default for ModuleLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_module_registration() {
        let manager = ModuleLifecycleManager::new();
        let app_id = AppId::new("test-app").unwrap();

        manager.register_module(app_id.clone()).unwrap();

        let state = manager.get_state(&app_id).unwrap();
        assert_eq!(state, ModuleState::Discovered);
    }

    #[tokio::test]
    async fn test_full_lifecycle() {
        let manager = ModuleLifecycleManager::new();
        let app_id = AppId::new("test-app").unwrap();

        manager.register_module(app_id.clone()).unwrap();
        manager.download(&app_id).await.unwrap();

        let state = manager.get_state(&app_id).unwrap();
        assert_eq!(state, ModuleState::Downloaded);
    }

    #[tokio::test]
    async fn test_state_transitions() {
        let manager = ModuleLifecycleManager::new();
        let app_id = AppId::new("test-app").unwrap();

        manager.register_module(app_id.clone()).unwrap();
        manager.download(&app_id).await.unwrap();
        manager.verify(&app_id).await.unwrap();
        manager.install(&app_id).await.unwrap();
        manager.load(&app_id).await.unwrap();
        manager.start(&app_id).await.unwrap();

        let state = manager.get_state(&app_id).unwrap();
        assert_eq!(state, ModuleState::Running);
    }

    #[tokio::test]
    async fn test_invalid_transition() {
        let manager = ModuleLifecycleManager::new();
        let app_id = AppId::new("test-app").unwrap();

        manager.register_module(app_id.clone()).unwrap();

        let result = manager.start(&app_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transition_history() {
        let manager = ModuleLifecycleManager::new();
        let app_id = AppId::new("test-app").unwrap();

        manager.register_module(app_id.clone()).unwrap();
        manager.download(&app_id).await.unwrap();

        let transitions = manager.get_transitions(&app_id).unwrap();
        assert_eq!(transitions.len(), 2);
    }

    #[test]
    fn test_count_by_state() {
        let manager = ModuleLifecycleManager::new();

        for i in 0..3 {
            let app_id = AppId::new(&format!("app{}", i)).unwrap();
            manager.register_module(app_id).unwrap();
        }

        let count = manager.count_by_state(ModuleState::Discovered);
        assert_eq!(count, 3);
    }
}
