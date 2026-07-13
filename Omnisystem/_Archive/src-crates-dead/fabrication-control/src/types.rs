use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    FDMPrinter,
    SLAPrinter,
    CNC,
    Laser,
    PickPlace,
    Etcher,
    Welder,
    Router,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaterialType {
    PLA,
    ABS,
    PETG,
    TPU,
    Carbon,
    Resin,
    Wood,
    Plastic,
    Metal,
    Glass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    Pending,
    Preparing,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub model: String,
    pub online: bool,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub device_id: String,
    pub material: MaterialType,
    pub state: JobState,
    pub progress: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialSpec {
    pub material_type: MaterialType,
    pub temp_min: f32,
    pub temp_max: f32,
    pub print_speed: f32,
    pub bed_temp: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        let device = Device {
            id: "dev1".to_string(),
            name: "Printer A".to_string(),
            device_type: DeviceType::FDMPrinter,
            model: "Prusa".to_string(),
            online: true,
            temperature: 200.0,
        };
        assert_eq!(device.device_type, DeviceType::FDMPrinter);
    }

    #[test]
    fn test_job_creation() {
        let job = Job {
            id: "job1".to_string(),
            device_id: "dev1".to_string(),
            material: MaterialType::PLA,
            state: JobState::Pending,
            progress: 0.0,
        };
        assert_eq!(job.state, JobState::Pending);
    }

    #[test]
    fn test_material_spec() {
        let spec = MaterialSpec {
            material_type: MaterialType::PLA,
            temp_min: 190.0,
            temp_max: 220.0,
            print_speed: 60.0,
            bed_temp: 60.0,
        };
        assert!(spec.temp_min < spec.temp_max);
    }
}
