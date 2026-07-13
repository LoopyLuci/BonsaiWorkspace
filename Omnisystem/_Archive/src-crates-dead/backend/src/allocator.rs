use crate::types::*;
use log::{info, warn};

pub struct Allocator;

/// Allocate devices for a given task based on available hardware.
pub fn allocate(profile: &HardwareProfile, task: &TaskRequirements) -> DeviceAllocation {
    // If GPUs are available and task allows GPU, try to allocate GPU
    if !profile.gpus.is_empty() && !task.allow_fallback {
        return allocate_gpu(profile, task);
    }

    // If GPUs available, try GPU first with fallback
    if !profile.gpus.is_empty() {
        if let Some(allocation) = try_allocate_gpu(profile, task) {
            return allocation;
        }
    }

    // Fallback to CPU
    allocate_cpu(profile, task)
}

fn try_allocate_gpu(profile: &HardwareProfile, task: &TaskRequirements) -> Option<DeviceAllocation> {
    // Find GPUs with enough VRAM
    let suitable: Vec<&GpuProfile> = profile.gpus.iter()
        .filter(|g| g.vram_bytes >= task.estimated_memory_bytes)
        .collect();

    if suitable.is_empty() {
        warn!("⚠️  No GPU found with {} GB of memory. Falling back to CPU.",
            task.estimated_memory_bytes / 1_000_000_000);
        return None;
    }

    Some(allocate_gpu_from_suitable(profile, task, suitable))
}

fn allocate_gpu(profile: &HardwareProfile, task: &TaskRequirements) -> DeviceAllocation {
    let suitable: Vec<&GpuProfile> = profile.gpus.iter()
        .filter(|g| g.vram_bytes >= task.estimated_memory_bytes)
        .collect();

    if suitable.is_empty() {
        warn!("❌ GPU allocation failed: no GPU with sufficient VRAM. Aborting task.");
        // Return empty allocation (caller should handle)
        return DeviceAllocation {
            devices: vec![],
            batch_size: 1,
            use_cpu_fallback: false,
            precision: task.precision,
        };
    }

    allocate_gpu_from_suitable(profile, task, suitable)
}

fn allocate_gpu_from_suitable(
    _profile: &HardwareProfile,
    task: &TaskRequirements,
    suitable: Vec<&GpuProfile>,
) -> DeviceAllocation {
    let mut devices = Vec::new();

    let batch_size = match task.task_type {
        TaskType::Training => {
            // Distribute training across all suitable GPUs
            info!("📊 Training: Using {} GPU(s)", suitable.len());
            for gpu in suitable.iter() {
                devices.push(Device {
                    device_type: DeviceType::Gpu,
                    index: gpu.index,
                    memory_allocated_bytes: (gpu.vram_bytes * 80) / 100, // Use 80% of VRAM
                });
            }
            // Batch size scales with GPU count
            8 * suitable.len() as u32
        }
        TaskType::Inference => {
            // Single GPU for inference (use the one with most VRAM)
            let best = suitable.iter().max_by_key(|g| g.vram_bytes).unwrap();
            info!("🚀 Inference: Using GPU {} ({})", best.index, best.name);
            devices.push(Device {
                device_type: DeviceType::Gpu,
                index: best.index,
                memory_allocated_bytes: (best.vram_bytes * 60) / 100, // Use 60% of VRAM
            });
            1
        }
        TaskType::Embedding => {
            // Embedding can use multiple GPUs
            info!("🔍 Embedding: Using {} GPU(s)", suitable.len());
            for gpu in suitable.iter() {
                devices.push(Device {
                    device_type: DeviceType::Gpu,
                    index: gpu.index,
                    memory_allocated_bytes: (gpu.vram_bytes * 70) / 100,
                });
            }
            4 * suitable.len() as u32
        }
        TaskType::Encoding => {
            // Encoding uses all GPUs for parallel processing
            info!("🎬 Encoding: Using {} GPU(s)", suitable.len());
            for gpu in suitable.iter() {
                devices.push(Device {
                    device_type: DeviceType::Gpu,
                    index: gpu.index,
                    memory_allocated_bytes: (gpu.vram_bytes * 75) / 100,
                });
            }
            8 * suitable.len() as u32
        }
        TaskType::Other => {
            // Generic task: use single best GPU
            let best = suitable.iter().max_by_key(|g| g.vram_bytes).unwrap();
            devices.push(Device {
                device_type: DeviceType::Gpu,
                index: best.index,
                memory_allocated_bytes: (best.vram_bytes * 60) / 100,
            });
            1
        }
    };

    // Select precision based on GPU capabilities
    let precision = select_precision_gpu(task.precision, suitable[0]);

    DeviceAllocation {
        devices,
        batch_size,
        use_cpu_fallback: false,
        precision,
    }
}

fn allocate_cpu(profile: &HardwareProfile, task: &TaskRequirements) -> DeviceAllocation {
    info!("💻 CPU allocation: {} cores, {} GB RAM",
        profile.cpu.logical_cores, profile.memory.total_bytes / 1_000_000_000);

    let mut memory_use = profile.memory.available_bytes / 2; // Use up to half of available RAM

    let batch_size = match task.task_type {
        TaskType::Training => {
            memory_use = profile.memory.available_bytes / 3;
            1 // Conservative batch size for CPU training
        }
        TaskType::Inference => {
            1
        }
        TaskType::Embedding => {
            (profile.cpu.logical_cores / 4).max(1)
        }
        TaskType::Encoding => {
            memory_use = profile.memory.available_bytes / 2;
            profile.cpu.logical_cores / 2
        }
        TaskType::Other => {
            1
        }
    };

    // Select CPU-optimized precision
    let precision = select_precision_cpu(task.precision, profile);

    DeviceAllocation {
        devices: vec![Device {
            device_type: DeviceType::Cpu,
            index: 0,
            memory_allocated_bytes: memory_use,
        }],
        batch_size,
        use_cpu_fallback: true,
        precision,
    }
}

fn select_precision_gpu(requested: Precision, gpu: &GpuProfile) -> Precision {
    match requested {
        Precision::Auto => {
            // Use FP16 for GPUs with sufficient VRAM, otherwise INT8
            if gpu.vram_bytes >= 8_000_000_000 && gpu.supports_fp16 {
                Precision::FP16
            } else if gpu.supports_int8 {
                Precision::INT8
            } else {
                Precision::FP32
            }
        }
        Precision::BF16 if !gpu.supports_bf16 => Precision::FP16,
        Precision::FP16 if !gpu.supports_fp16 => Precision::FP32,
        Precision::INT8 if !gpu.supports_int8 => Precision::INT8,
        other => other,
    }
}

fn select_precision_cpu(requested: Precision, profile: &HardwareProfile) -> Precision {
    // CPU prefers quantized models for speed
    match requested {
        Precision::Auto => {
            // Use INT4 or INT8 for CPU
            if profile.memory.available_bytes < 4_000_000_000 {
                Precision::INT4
            } else {
                Precision::INT8
            }
        }
        Precision::FP16 | Precision::BF16 => {
            // CPU can do FP16 but prefers quantization
            Precision::INT8
        }
        other => other,
    }
}
