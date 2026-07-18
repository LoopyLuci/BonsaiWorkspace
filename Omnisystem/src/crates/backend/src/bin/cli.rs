//! Backend (BUEB) CLI: detects local hardware and prints a device
//! allocation plan for a representative inference task.

use backend::{allocate, cpu_cores, gpu_count, has_gpu, initialize, profile, total_memory};
use backend::{Precision, TaskRequirements, TaskType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize()?;
    let hw = profile();

    println!("CPU: {} ({} logical cores, {} MHz)", hw.cpu.model, hw.cpu.logical_cores, hw.cpu.frequency_mhz);
    println!("Memory: {:.2} GB total", total_memory() as f64 / 1e9);
    println!("GPUs: {}", gpu_count());
    println!("CPU cores available: {}", cpu_cores());

    let allocation = allocate(&TaskRequirements {
        task_type: TaskType::Inference,
        estimated_memory_bytes: 600_000_000,
        min_compute_units: 0,
        precision: Precision::Auto,
        allow_fallback: true,
    });

    println!(
        "\nAllocation plan: {} device(s), batch_size={}, precision={}, cpu_fallback={}",
        allocation.devices.len(),
        allocation.batch_size,
        allocation.precision,
        allocation.use_cpu_fallback
    );

    if has_gpu() {
        println!("GPU(s) detected -- prefer GPU-accelerated inference.");
    } else {
        println!("No GPU detected -- recommend quantized (INT8/INT4) models.");
    }

    Ok(())
}
