use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::registry::DeviceRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FreeProjectStatus {
    Queued,
    Running,
    Completed,
    Cancelled,
}

/// A project submitted to run on the free community pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeProject {
    pub project_id: Uuid,
    pub owner_id: Uuid,
    pub task_type: String,
    pub submitted_at: DateTime<Utc>,
    pub requires_gpu: bool,
    pub progress: f64,
    pub status: FreeProjectStatus,
}

/// Orchestrates free-tier project scheduling across the community pool.
pub struct FreeTierOrchestrator {
    pub registry: Arc<DeviceRegistry>,
    projects: Arc<RwLock<HashMap<Uuid, FreeProject>>>,
    last_rebalance: Arc<Mutex<DateTime<Utc>>>,
}

impl FreeTierOrchestrator {
    pub fn new(registry: Arc<DeviceRegistry>) -> Self {
        FreeTierOrchestrator {
            registry,
            projects: Arc::new(RwLock::new(HashMap::new())),
            last_rebalance: Arc::new(Mutex::new(Utc::now())),
        }
    }

    pub fn submit_project(&self, project: FreeProject) -> Uuid {
        let id = project.project_id;
        let mut map = self.projects.write().unwrap();
        map.insert(id, project);
        id
    }

    pub fn complete_project(&self, project_id: Uuid) {
        let mut map = self.projects.write().unwrap();
        if let Some(p) = map.get_mut(&project_id) {
            p.status = FreeProjectStatus::Completed;
        }
    }

    pub fn cancel_project(&self, project_id: Uuid) {
        let mut map = self.projects.write().unwrap();
        if let Some(p) = map.get_mut(&project_id) {
            p.status = FreeProjectStatus::Cancelled;
        }
    }

    pub fn update_progress(&self, project_id: Uuid, progress: f64) {
        let mut map = self.projects.write().unwrap();
        if let Some(p) = map.get_mut(&project_id) {
            p.progress = progress.clamp(0.0, 1.0);
        }
    }

    pub fn active_count(&self) -> usize {
        let map = self.projects.read().unwrap();
        map.values()
            .filter(|p| {
                p.status == FreeProjectStatus::Queued || p.status == FreeProjectStatus::Running
            })
            .count()
    }

    /// Returns (cpu_urv_per_project, gpu_urv_per_project) based on available pool capacity.
    pub fn per_project_urv(&self) -> (f64, f64) {
        let total_urv = self.registry.free_pool_urv();
        let active = self.active_count();
        if active == 0 {
            return (total_urv, 0.0);
        }
        let per_project = total_urv / active as f64;
        // Simple split: no GPU tracking yet — full URV assigned to CPU side
        (per_project, 0.0)
    }

    fn rebalance(&self) {
        let (cpu_urv, gpu_urv) = self.per_project_urv();
        let active = self.active_count();
        tracing::info!(
            active_projects = active,
            cpu_urv_per_project = cpu_urv,
            gpu_urv_per_project = gpu_urv,
            "free-tier rebalance"
        );
        *self.last_rebalance.lock().unwrap() = Utc::now();
    }

    /// Spawnable background task that rebalances every 30 seconds.
    pub async fn rebalance_loop(self: Arc<Self>) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            self.rebalance();
        }
    }
}
