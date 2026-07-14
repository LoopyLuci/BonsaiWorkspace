//! CLI demo for fabrication-control: registers a printer, submits a
//! job, and generates a real toolpath.

use fabrication_control::controller::DeviceController;
use fabrication_control::path_gen::PathGenerator;
use fabrication_control::{Device, DeviceType, Job, JobState, MaterialType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let controller = DeviceController::new();

    controller.register_device(Device {
        id: "printer1".to_string(),
        name: "Prusa i3".to_string(),
        device_type: DeviceType::FDMPrinter,
        model: "Prusa i3 MK3S+".to_string(),
        online: true,
        temperature: 200.0,
    })?;
    println!("Devices registered: {}", controller.device_count());

    controller.submit_job(Job {
        id: "job1".to_string(),
        device_id: "printer1".to_string(),
        material: MaterialType::PLA,
        state: JobState::Pending,
        progress: 0.0,
    })?;
    controller.update_job_state("job1", JobState::Running)?;
    let job = controller.get_job("job1")?;
    println!("Job {} is now {:?}", job.id, job.state);

    let path_gen = PathGenerator::new(0.5);
    let line = path_gen.generate_line((0.0, 0.0, 0.0), (10.0, 10.0, 0.0));
    let circle = path_gen.generate_circle((0.0, 0.0), 5.0);
    println!(
        "Generated toolpaths: {} line points, {} circle points",
        line.len(),
        circle.len()
    );

    Ok(())
}
