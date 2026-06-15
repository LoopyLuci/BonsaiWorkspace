use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub system_name: String,
    pub system_type: SystemType,
    pub status: SystemStatus,
    pub endpoints: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemType {
    Buddy,
    OmniBot,
    RemoteAccess,
    USEE,
    FTDaemon,
    TransferDaemon,
    IDE,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemStatus {
    Online,
    Offline,
    Initializing,
    Error,
}

pub struct SystemRegistry {
    systems: Arc<DashMap<String, SystemInfo>>,
}

impl SystemRegistry {
    pub fn new() -> Self {
        Self {
            systems: Arc::new(DashMap::new()),
        }
    }

    pub fn register_system(&self, system_name: String, info: SystemInfo) {
        self.systems.insert(system_name, info);
    }

    pub fn get_system(&self, system_name: &str) -> Option<SystemInfo> {
        self.systems.get(system_name).map(|s| s.clone())
    }

    pub fn set_status(&self, system_name: &str, status: SystemStatus) -> bool {
        if let Some(mut sys) = self.systems.get_mut(system_name) {
            sys.status = status;
            true
        } else {
            false
        }
    }

    pub fn get_online_systems(&self) -> Vec<SystemInfo> {
        self.systems
            .iter()
            .filter(|entry| entry.status == SystemStatus::Online)
            .map(|entry| entry.clone())
            .collect()
    }

    pub fn system_count(&self) -> usize {
        self.systems.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_registration() {
        let sr = SystemRegistry::new();
        let info = SystemInfo {
            system_name: "buddy".to_string(),
            system_type: SystemType::Buddy,
            status: SystemStatus::Online,
            endpoints: vec!["localhost:3000".to_string()],
        };
        sr.register_system("buddy".to_string(), info);
        assert!(sr.get_system("buddy").is_some());
    }

    #[test]
    fn test_online_systems() {
        let sr = SystemRegistry::new();
        let info = SystemInfo {
            system_name: "buddy".to_string(),
            system_type: SystemType::Buddy,
            status: SystemStatus::Online,
            endpoints: vec![],
        };
        sr.register_system("buddy".to_string(), info);
        let online = sr.get_online_systems();
        assert_eq!(online.len(), 1);
    }
}
