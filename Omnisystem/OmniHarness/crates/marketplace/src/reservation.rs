use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractStatus {
    Active,
    Completed,
    Cancelled,
}

/// A time-bounded rental contract between a renter and a device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentalContract {
    pub contract_id: Uuid,
    pub renter_id: Uuid,
    pub device_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub reserved_minutes: u32,
    pub price_per_minute: f64,
    pub status: ContractStatus,
}

/// Manages active rental contracts in memory.
#[derive(Clone)]
pub struct ReservationManager {
    contracts: Arc<RwLock<HashMap<Uuid, RentalContract>>>,
}

impl ReservationManager {
    pub fn new() -> Self {
        ReservationManager {
            contracts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn reserve(
        &self,
        device_id: Uuid,
        renter_id: Uuid,
        minutes: u32,
        price_per_min: f64,
    ) -> RentalContract {
        let contract = RentalContract {
            contract_id: Uuid::new_v4(),
            renter_id,
            device_id,
            started_at: Utc::now(),
            reserved_minutes: minutes,
            price_per_minute: price_per_min,
            status: ContractStatus::Active,
        };
        let mut map = self.contracts.write().unwrap();
        map.insert(contract.contract_id, contract.clone());
        contract
    }

    pub fn complete(&self, contract_id: Uuid) {
        let mut map = self.contracts.write().unwrap();
        if let Some(c) = map.get_mut(&contract_id) {
            c.status = ContractStatus::Completed;
        }
    }

    pub fn cancel(&self, contract_id: Uuid) {
        let mut map = self.contracts.write().unwrap();
        if let Some(c) = map.get_mut(&contract_id) {
            c.status = ContractStatus::Cancelled;
        }
    }

    pub fn active_for_device(&self, device_id: Uuid) -> Option<RentalContract> {
        let map = self.contracts.read().unwrap();
        map.values()
            .find(|c| c.device_id == device_id && c.status == ContractStatus::Active)
            .cloned()
    }

    pub fn active_for_renter(&self, renter_id: Uuid) -> Vec<RentalContract> {
        let map = self.contracts.read().unwrap();
        map.values()
            .filter(|c| c.renter_id == renter_id && c.status == ContractStatus::Active)
            .cloned()
            .collect()
    }
}

impl Default for ReservationManager {
    fn default() -> Self {
        Self::new()
    }
}
