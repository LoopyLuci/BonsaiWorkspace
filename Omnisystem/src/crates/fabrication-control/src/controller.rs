use crate::{Device, Job, JobState, Result, FabricationError, DeviceRegistry, MaterialDatabase};
use dashmap::DashMap;
use std::sync::Arc;

pub struct DeviceController {
    devices: Arc<DeviceRegistry>,
    materials: Arc<MaterialDatabase>,
    jobs: Arc<DashMap<String, Job>>,
}

impl DeviceController {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(DeviceRegistry::new()),
            materials: Arc::new(MaterialDatabase::new()),
            jobs: Arc::new(DashMap::new()),
        }
    }

    pub fn register_device(&self, device: Device) -> Result<()> {
        self.devices.register(device)
    }

    pub fn submit_job(&self, job: Job) -> Result<()> {
        self.devices.get(&job.device_id)?;
        self.jobs.insert(job.id.clone(), job);
        tracing::info!("Job submitted");
        Ok(())
    }

    pub fn get_job(&self, id: &str) -> Result<Job> {
        self.jobs
            .get(id)
            .map(|ref_| ref_.value().clone())
            .ok_or_else(|| FabricationError::InvalidJob(format!("Job not found: {}", id)))
    }

    pub fn update_job_state(&self, id: &str, state: JobState) -> Result<()> {
        if let Some(mut job) = self.jobs.get_mut(id) {
            job.state = state;
            Ok(())
        } else {
            Err(FabricationError::InvalidJob(format!("Job not found: {}", id)))
        }
    }

    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }

    pub fn device_count(&self) -> usize {
        self.devices.device_count()
    }
}

impl Default for DeviceController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeviceType;

    #[test]
    fn test_controller_lifecycle() {
        let controller = DeviceController::new();
        let device = Device {
            id: "dev1".to_string(),
            name: "Printer".to_string(),
            device_type: DeviceType::FDMPrinter,
            model: "Prusa".to_string(),
            online: true,
            temperature: 200.0,
        };
        assert!(controller.register_device(device).is_ok());
        assert_eq!(controller.device_count(), 1);
    }

    #[test]
    fn test_submit_job() {
        let controller = DeviceController::new();
        let device = Device {
            id: "dev1".to_string(),
            name: "Printer".to_string(),
            device_type: DeviceType::FDMPrinter,
            model: "Prusa".to_string(),
            online: true,
            temperature: 200.0,
        };
        controller.register_device(device).unwrap();

        let job = Job {
            id: "job1".to_string(),
            device_id: "dev1".to_string(),
            material: crate::MaterialType::PLA,
            state: JobState::Pending,
            progress: 0.0,
        };
        assert!(controller.submit_job(job).is_ok());
        assert_eq!(controller.job_count(), 1);
    }
}
