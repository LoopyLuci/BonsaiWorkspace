use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub use credits::urv::DeviceClass;

/// A registered device available on the marketplace.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceRegistration {
    pub device_id: Uuid,
    pub owner_id: Uuid,
    pub display_name: String,
    pub urv: f64,
    pub device_class: DeviceClass,
    /// Percentage of capacity offered to the free community pool.
    pub free_tier_pct: u8,
    /// Percentage of capacity available for paid rental.
    pub paid_pct: u8,
    /// Price multiplier relative to the base rate.
    pub price_multiplier: f64,
    /// 0.0–1.0 reliability score based on historical uptime.
    pub reliability_score: f64,
    pub last_seen: DateTime<Utc>,
    pub is_online: bool,
}

/// In-memory registry of all devices.
#[derive(Clone)]
pub struct DeviceRegistry {
    inner: Arc<RwLock<HashMap<Uuid, DeviceRegistration>>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        DeviceRegistry {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register(&self, reg: DeviceRegistration) {
        let mut map = self.inner.write().unwrap();
        map.insert(reg.device_id, reg);
    }

    pub fn unregister(&self, device_id: Uuid) {
        let mut map = self.inner.write().unwrap();
        map.remove(&device_id);
    }

    pub fn heartbeat(&self, device_id: Uuid) {
        let mut map = self.inner.write().unwrap();
        if let Some(reg) = map.get_mut(&device_id) {
            reg.last_seen = Utc::now();
            reg.is_online = true;
        }
    }

    /// Mark devices as offline if they have not sent a heartbeat in the last 90 seconds.
    pub fn mark_stale(&self) {
        let cutoff = Utc::now() - chrono::Duration::seconds(90);
        let mut map = self.inner.write().unwrap();
        for reg in map.values_mut() {
            if reg.last_seen < cutoff {
                reg.is_online = false;
            }
        }
    }

    pub fn list_online(&self) -> Vec<DeviceRegistration> {
        let map = self.inner.read().unwrap();
        map.values().filter(|r| r.is_online).cloned().collect()
    }

    /// Total URV capacity offered to the free pool across all online devices.
    pub fn free_pool_urv(&self) -> f64 {
        let map = self.inner.read().unwrap();
        map.values()
            .filter(|r| r.is_online)
            .map(|r| r.urv * r.free_tier_pct as f64 / 100.0)
            .sum()
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
