use fabrication_control::*;

#[test]
fn test_full_fabrication_workflow() {
    let controller = controller::DeviceController::new();

    let device = Device {
        id: "printer1".to_string(),
        name: "Prusa i3".to_string(),
        device_type: DeviceType::FDMPrinter,
        model: "Prusa i3 MK3S+".to_string(),
        online: true,
        temperature: 200.0,
    };

    controller.register_device(device).unwrap();
    assert_eq!(controller.device_count(), 1);

    let job = Job {
        id: "job1".to_string(),
        device_id: "printer1".to_string(),
        material: MaterialType::PLA,
        state: JobState::Pending,
        progress: 0.0,
    };

    controller.submit_job(job).unwrap();
    assert_eq!(controller.job_count(), 1);

    controller.update_job_state("job1", JobState::Running).unwrap();
    let updated = controller.get_job("job1").unwrap();
    assert_eq!(updated.state, JobState::Running);
}

#[test]
fn test_material_database() {
    let materials = material::MaterialDatabase::new();
    assert!(materials.get_spec(MaterialType::PLA).is_ok());
    assert!(materials.get_spec(MaterialType::ABS).is_ok());
}

#[test]
fn test_path_generation() {
    let gen = path_gen::PathGenerator::new(0.1);
    let line_path = gen.generate_line((0.0, 0.0, 0.0), (5.0, 5.0, 0.0));
    assert!(line_path.len() > 1);
    
    let circle_path = gen.generate_circle((0.0, 0.0), 2.0);
    assert!(circle_path.len() > 1);
}
