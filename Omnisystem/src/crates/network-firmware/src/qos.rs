use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QosClass {
    Premium,
    Standard,
    Best,
}

#[derive(Debug, Clone, Copy)]
pub struct QosPolicy {
    pub bandwidth_limit: u32,
    pub priority: u8,
    pub burst_size: u32,
}

pub struct QosManager {
    policies: Arc<DashMap<String, QosPolicy>>,
}

impl QosManager {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
        }
    }

    pub fn add_policy(&self, interface: String, policy: QosPolicy) {
        self.policies.insert(interface, policy);
    }

    pub fn get_policy(&self, interface: &str) -> Option<QosPolicy> {
        self.policies.get(interface).map(|p| *p)
    }

    pub fn update_policy(&self, interface: &str, bandwidth: u32) -> bool {
        if let Some(mut policy) = self.policies.get_mut(interface) {
            policy.bandwidth_limit = bandwidth;
            true
        } else {
            false
        }
    }

    pub fn get_qos_class(priority: u8) -> QosClass {
        match priority {
            0..=3 => QosClass::Best,
            4..=6 => QosClass::Standard,
            7.. => QosClass::Premium,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qos_manager() {
        let qm = QosManager::new();
        let policy = QosPolicy {
            bandwidth_limit: 1000,
            priority: 7,
            burst_size: 200,
        };
        qm.add_policy("eth0".to_string(), policy);
        assert!(qm.get_policy("eth0").is_some());
    }

    #[test]
    fn test_qos_class() {
        assert_eq!(QosManager::get_qos_class(2), QosClass::Best);
        assert_eq!(QosManager::get_qos_class(5), QosClass::Standard);
        assert_eq!(QosManager::get_qos_class(7), QosClass::Premium);
    }
}
