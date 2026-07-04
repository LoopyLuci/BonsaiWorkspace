use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Material library and profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialLibrary {
    pub library_id: String,
    pub materials: HashMap<String, MaterialSpec>,
    pub material_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialSpec {
    pub material_id: String,
    pub material_name: String,
    pub material_type: String,
    pub density_g_cm3: f32,
    pub melting_point_c: Option<u32>,
    pub properties: HashMap<String, String>,
}

impl MaterialLibrary {
    pub fn new(library_id: String) -> Self {
        MaterialLibrary {
            library_id,
            materials: HashMap::new(),
            material_count: 0,
        }
    }

    pub fn add_material(&mut self, material: MaterialSpec) {
        self.materials.insert(material.material_id.clone(), material);
        self.material_count += 1;
    }

    pub fn get_material(&self, id: &str) -> Option<&MaterialSpec> {
        self.materials.get(id)
    }
}

/// Print profile (device-specific settings)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintProfile {
    pub profile_id: String,
    pub device_id: String,
    pub material_id: String,
    pub temperature_c: u32,
    pub speed_percent: u8,
    pub layer_height_mm: f32,
    pub infill_percent: u8,
}

impl PrintProfile {
    pub fn new(profile_id: String, device_id: String, material_id: String) -> Self {
        PrintProfile {
            profile_id,
            device_id,
            material_id,
            temperature_c: 200,
            speed_percent: 100,
            layer_height_mm: 0.2,
            infill_percent: 20,
        }
    }
}

/// Multi-device job orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobOrchestrator {
    pub orchestrator_id: String,
    pub devices: Vec<String>,
    pub job_queue: Vec<OrchestrationJob>,
    pub active_jobs: HashMap<String, JobState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationJob {
    pub job_id: String,
    pub device_id: String,
    pub priority: u8,
    pub estimated_duration_sec: u32,
    pub dependencies: Vec<String>, // job_ids this depends on
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobState {
    Queued,
    Scheduled,
    Running,
    Paused,
    Completed,
    Failed,
}

impl JobOrchestrator {
    pub fn new(orchestrator_id: String) -> Self {
        JobOrchestrator {
            orchestrator_id,
            devices: vec![],
            job_queue: vec![],
            active_jobs: HashMap::new(),
        }
    }

    pub fn add_device(&mut self, device_id: String) {
        if !self.devices.contains(&device_id) {
            self.devices.push(device_id);
        }
    }

    pub async fn submit_job(&mut self, job: OrchestrationJob) -> Result<()> {
        self.job_queue.push(job);
        Ok(())
    }

    pub async fn schedule_jobs(&mut self) -> Result<u32> {
        let mut scheduled = 0;
        for job in &self.job_queue {
            self.active_jobs.insert(job.job_id.clone(), JobState::Scheduled);
            scheduled += 1;
        }
        Ok(scheduled)
    }

    pub fn job_count(&self) -> usize {
        self.job_queue.len()
    }
}

/// Multi-device coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCoordinator {
    pub coordinator_id: String,
    pub coordinated_devices: Vec<String>,
    pub sync_state: HashMap<String, DeviceSync>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSync {
    pub device_id: String,
    pub last_sync_ms: u64,
    pub status: SyncStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    InSync,
    Syncing,
    OutOfSync,
    Error,
}

impl DeviceCoordinator {
    pub fn new(coordinator_id: String) -> Self {
        DeviceCoordinator {
            coordinator_id,
            coordinated_devices: vec![],
            sync_state: HashMap::new(),
        }
    }

    pub fn add_device(&mut self, device_id: String) {
        if !self.coordinated_devices.contains(&device_id) {
            self.coordinated_devices.push(device_id.clone());
            self.sync_state.insert(device_id, DeviceSync {
                device_id: "".to_string(),
                last_sync_ms: 0,
                status: SyncStatus::InSync,
            });
        }
    }

    pub async fn synchronize_devices(&mut self) -> Result<()> {
        for device_id in &self.coordinated_devices {
            if let Some(sync) = self.sync_state.get_mut(device_id) {
                sync.status = SyncStatus::InSync;
            }
        }
        Ok(())
    }

    pub fn device_count(&self) -> usize {
        self.coordinated_devices.len()
    }
}

/// Work cell (collection of coordinated devices)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkCell {
    pub cell_id: String,
    pub devices: Vec<String>,
    pub coordinator_id: String,
    pub throughput_jobs_per_hour: u32,
}

impl WorkCell {
    pub fn new(cell_id: String, coordinator_id: String) -> Self {
        WorkCell {
            cell_id,
            devices: vec![],
            coordinator_id,
            throughput_jobs_per_hour: 0,
        }
    }

    pub fn add_device(&mut self, device_id: String) {
        if !self.devices.contains(&device_id) {
            self.devices.push(device_id);
        }
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_library() {
        let mut lib = MaterialLibrary::new("lib1".to_string());
        let mat = MaterialSpec {
            material_id: "abs".to_string(),
            material_name: "ABS Plastic".to_string(),
            material_type: "Thermoplastic".to_string(),
            density_g_cm3: 1.05,
            melting_point_c: Some(220),
            properties: HashMap::new(),
        };
        lib.add_material(mat);
        assert_eq!(lib.material_count, 1);
    }

    #[test]
    fn test_print_profile() {
        let profile = PrintProfile::new("prof1".to_string(), "device1".to_string(), "abs".to_string());
        assert_eq!(profile.temperature_c, 200);
    }

    #[test]
    fn test_job_orchestrator() {
        let mut orch = JobOrchestrator::new("orch1".to_string());
        orch.add_device("device1".to_string());
        assert_eq!(orch.devices.len(), 1);
    }

    #[test]
    fn test_job_state() {
        let states = vec![JobState::Queued, JobState::Running, JobState::Completed];
        assert_eq!(states.len(), 3);
    }

    #[test]
    fn test_device_coordinator() {
        let mut coord = DeviceCoordinator::new("coord1".to_string());
        coord.add_device("device1".to_string());
        assert_eq!(coord.device_count(), 1);
    }

    #[test]
    fn test_work_cell() {
        let mut cell = WorkCell::new("cell1".to_string(), "coord1".to_string());
        cell.add_device("device1".to_string());
        cell.add_device("device2".to_string());
        assert_eq!(cell.device_count(), 2);
    }
}
