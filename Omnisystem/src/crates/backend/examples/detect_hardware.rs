use backend::*;

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .ok();

    println!("\n🔧 BUEB Hardware Detection Example\n");
    println!("═══════════════════════════════════════════════════════════════");

    // Initialize BUEB (detects all hardware)
    initialize()?;

    let profile = profile();

    println!("\n📊 SYSTEM CONFIGURATION:\n");

    // CPU information
    println!("CPU Information:");
    println!("  Vendor: {}", profile.cpu.vendor);
    println!("  Model: {}", profile.cpu.model);
    println!("  Physical Cores: {}", profile.cpu.physical_cores);
    println!("  Logical Cores: {}", profile.cpu.logical_cores);
    println!("  Frequency: {} MHz", profile.cpu.frequency_mhz);
    println!("  L3 Cache: {} MB", profile.cpu.cache_l3_mb);
    println!("  SIMD Features: {:?}", profile.cpu.simd_features);

    // Memory information
    println!("\nMemory:");
    println!("  Total: {:.2} GB", profile.memory.total_bytes as f64 / 1e9);
    println!("  Available: {:.2} GB", profile.memory.available_bytes as f64 / 1e9);

    // GPU information
    if profile.gpus.is_empty() {
        println!("\nGPUs: None detected");
    } else {
        println!("\nGPUs ({}):", profile.gpus.len());
        for (i, gpu) in profile.gpus.iter().enumerate() {
            println!("  [{}] {}", i, gpu.name);
            println!("      VRAM: {:.2} GB", gpu.vram_bytes as f64 / 1e9);
            println!("      Backend: {}", gpu.backend);
            println!("      Compute Units: {}", gpu.compute_units);
            println!("      FP16: {}, BF16: {}, INT8: {}",
                gpu.supports_fp16, gpu.supports_bf16, gpu.supports_int8);
        }
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");

    // Demonstrate device allocation for different task types
    println!("🎯 DEVICE ALLOCATION EXAMPLES:\n");

    let tasks = vec![
        (
            "Inference",
            TaskRequirements {
                task_type: TaskType::Inference,
                estimated_memory_bytes: 4_000_000_000,
                min_compute_units: 0,
                precision: Precision::Auto,
                allow_fallback: true,
            },
        ),
        (
            "Training",
            TaskRequirements {
                task_type: TaskType::Training,
                estimated_memory_bytes: 8_000_000_000,
                min_compute_units: 4,
                precision: Precision::Auto,
                allow_fallback: true,
            },
        ),
        (
            "Embedding",
            TaskRequirements {
                task_type: TaskType::Embedding,
                estimated_memory_bytes: 2_000_000_000,
                min_compute_units: 0,
                precision: Precision::Auto,
                allow_fallback: true,
            },
        ),
        (
            "Encoding",
            TaskRequirements {
                task_type: TaskType::Encoding,
                estimated_memory_bytes: 6_000_000_000,
                min_compute_units: 2,
                precision: Precision::Auto,
                allow_fallback: true,
            },
        ),
    ];

    for (task_name, task) in tasks {
        let allocation = allocate(&task);

        println!("{}:", task_name);
        println!("  Precision: {}", allocation.precision);
        println!("  Batch Size: {}", allocation.batch_size);
        println!("  CPU Fallback: {}", allocation.use_cpu_fallback);
        println!("  Devices: {}", allocation.devices.len());

        for (i, device) in allocation.devices.iter().enumerate() {
            println!("    [{}] {} (Index: {}, Memory: {:.2} GB)",
                i, device.device_type, device.index,
                device.memory_allocated_bytes as f64 / 1e9);
        }
        println!();
    }

    println!("═══════════════════════════════════════════════════════════════");
    println!("\n✅ BUEB Configuration Complete\n");

    // Print summary
    println!("Summary:");
    if has_gpu() {
        println!("  ✅ GPU available ({} devices)", gpu_count());
        println!("  ✅ Multi-GPU training supported");
    } else {
        println!("  ⚠️  CPU-only system");
        println!("  💡 Recommendation: Use quantized models (Q4_K_M format)");
    }
    println!("  ✅ CPU cores: {}", cpu_cores());
    println!("  ✅ Total memory: {:.2} GB", total_memory() as f64 / 1e9);

    Ok(())
}
