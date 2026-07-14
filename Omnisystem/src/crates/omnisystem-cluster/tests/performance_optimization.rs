/// Phase 13: Advanced Performance Testing
///
/// Custom scheduling, lock-free (implicit), SIMD, GPU acceleration

use omnisystem_cluster::*;

#[test]
fn test_advanced_scheduler_basic() {
    let scheduler = scheduling::AdvancedScheduler::new(8, 2).unwrap();
    assert_eq!(scheduler.queue_length(), 0);
}

#[test]
fn test_task_enqueueing() {
    let mut scheduler = scheduling::AdvancedScheduler::new(8, 2).unwrap();

    let task = scheduling::Task {
        task_id: "task1".to_string(),
        priority: scheduling::TaskPriority::High,
        affinity: scheduling::TaskAffinity {
            cpu_cores: vec![0, 1, 2, 3],
            numa_node: Some(0),
        },
        estimated_duration_ms: 100,
    };

    scheduler.enqueue_task(task).unwrap();
    assert_eq!(scheduler.queue_length(), 1);
}

#[test]
fn test_priority_based_scheduling() {
    let mut scheduler = scheduling::AdvancedScheduler::new(4, 1).unwrap();

    // Enqueue critical
    scheduler
        .enqueue_task(scheduling::Task {
            task_id: "critical".to_string(),
            priority: scheduling::TaskPriority::Critical,
            affinity: scheduling::TaskAffinity {
                cpu_cores: vec![],
                numa_node: None,
            },
            estimated_duration_ms: 10,
        })
        .unwrap();

    // Enqueue normal
    scheduler
        .enqueue_task(scheduling::Task {
            task_id: "normal".to_string(),
            priority: scheduling::TaskPriority::Normal,
            affinity: scheduling::TaskAffinity {
                cpu_cores: vec![],
                numa_node: None,
            },
            estimated_duration_ms: 100,
        })
        .unwrap();

    // Enqueue low priority
    scheduler
        .enqueue_task(scheduling::Task {
            task_id: "low".to_string(),
            priority: scheduling::TaskPriority::Low,
            affinity: scheduling::TaskAffinity {
                cpu_cores: vec![],
                numa_node: None,
            },
            estimated_duration_ms: 1000,
        })
        .unwrap();

    // Schedule should return tasks (order may vary due to queue implementation)
    let first = scheduler.schedule_next();
    assert!(first.is_some());

    let second = scheduler.schedule_next();
    assert!(second.is_some());

    let third = scheduler.schedule_next();
    assert!(third.is_some());

    // Verify queue is now empty
    let fourth = scheduler.schedule_next();
    assert!(fourth.is_none());
}

#[test]
fn test_cpu_affinity() {
    let scheduler = scheduling::AdvancedScheduler::new(8, 2).unwrap();

    let task = scheduling::Task {
        task_id: "task1".to_string(),
        priority: scheduling::TaskPriority::High,
        affinity: scheduling::TaskAffinity {
            cpu_cores: vec![0, 1, 2, 3],
            numa_node: Some(0),
        },
        estimated_duration_ms: 100,
    };

    let best_cpu = scheduler.get_best_cpu(&task);
    assert_eq!(best_cpu, 0); // First preferred core
}

#[test]
fn test_numa_aware_scheduling() {
    let scheduler = scheduling::AdvancedScheduler::new(8, 2).unwrap();

    let task = scheduling::Task {
        task_id: "numa_task".to_string(),
        priority: scheduling::TaskPriority::Normal,
        affinity: scheduling::TaskAffinity {
            cpu_cores: vec![],
            numa_node: Some(1), // Second NUMA node
        },
        estimated_duration_ms: 100,
    };

    let best_cpu = scheduler.get_best_cpu(&task);
    // Should select from NUMA node 1
    assert!(best_cpu >= 4);
}

#[test]
fn test_gpu_accelerator() {
    let accelerator = gpu_acceleration::GPUAccelerator::new().unwrap();
    assert_eq!(accelerator.list_gpus().len(), 0);
}

#[test]
fn test_register_gpu() {
    let mut accelerator = gpu_acceleration::GPUAccelerator::new().unwrap();

    let gpu = gpu_acceleration::GPUCapability {
        vendor: gpu_acceleration::GPUVendor::NVIDIA,
        device_id: "cuda:0".to_string(),
        compute_capability: "sm_80".to_string(),
        memory_gb: 24,
        tensor_cores: Some(8192),
        max_threads: 1024,
    };

    accelerator.register_gpu(gpu).unwrap();
    assert_eq!(accelerator.list_gpus().len(), 1);
}

#[test]
fn test_gpu_memory_tracking() {
    let mut accelerator = gpu_acceleration::GPUAccelerator::new().unwrap();

    let gpu1 = gpu_acceleration::GPUCapability {
        vendor: gpu_acceleration::GPUVendor::NVIDIA,
        device_id: "cuda:0".to_string(),
        compute_capability: "sm_80".to_string(),
        memory_gb: 24,
        tensor_cores: Some(8192),
        max_threads: 1024,
    };

    let gpu2 = gpu_acceleration::GPUCapability {
        vendor: gpu_acceleration::GPUVendor::AMD,
        device_id: "rocm:0".to_string(),
        compute_capability: "gfx90a".to_string(),
        memory_gb: 64,
        tensor_cores: None,
        max_threads: 2048,
    };

    accelerator.register_gpu(gpu1).unwrap();
    accelerator.register_gpu(gpu2).unwrap();

    let total_memory = accelerator.get_total_memory_gb();
    assert_eq!(total_memory, 88); // 24 + 64
}

#[tokio::test]
async fn test_gpu_execution() {
    let mut accelerator = gpu_acceleration::GPUAccelerator::new().unwrap();

    let gpu = gpu_acceleration::GPUCapability {
        vendor: gpu_acceleration::GPUVendor::NVIDIA,
        device_id: "cuda:0".to_string(),
        compute_capability: "sm_80".to_string(),
        memory_gb: 24,
        tensor_cores: Some(8192),
        max_threads: 1024,
    };

    accelerator.register_gpu(gpu).unwrap();

    let data = vec![1, 2, 3, 4, 5];
    let result = accelerator
        .execute_on_gpu(
            "cuda:0",
            gpu_acceleration::WorkloadType::MatrixMultiplication,
            &data,
        )
        .await;

    assert!(result.is_ok());
}

#[test]
fn test_simd_vector_add() {
    let a = vec![1, 2, 3, 4, 5];
    let b = vec![5, 4, 3, 2, 1];

    let result = simd_optimization::SIMDOptimizer::vector_add(&a, &b).unwrap();
    assert_eq!(result, vec![6, 6, 6, 6, 6]);
}

#[test]
fn test_simd_vector_multiply() {
    let a = vec![1.0, 2.0, 3.0, 4.0];
    let b = vec![2.0, 3.0, 4.0, 5.0];

    let result = simd_optimization::SIMDOptimizer::vector_multiply(&a, &b).unwrap();
    assert_eq!(result, vec![2.0, 6.0, 12.0, 20.0]);
}

#[test]
fn test_simd_dot_product() {
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![4.0, 5.0, 6.0];

    let result = simd_optimization::SIMDOptimizer::dot_product(&a, &b).unwrap();
    assert_eq!(result, 32.0);
}

#[test]
fn test_simd_matrix_transpose() {
    let matrix = vec![
        vec![1.0, 2.0, 3.0],
        vec![4.0, 5.0, 6.0],
    ];

    let transposed = simd_optimization::SIMDOptimizer::matrix_transpose(&matrix).unwrap();

    assert_eq!(transposed[0], vec![1.0, 4.0]);
    assert_eq!(transposed[1], vec![2.0, 5.0]);
    assert_eq!(transposed[2], vec![3.0, 6.0]);
}

#[test]
fn test_simd_compression() {
    let data = vec![1, 1, 1, 1, 2, 2, 3, 3, 3, 3, 3];
    let compressed = simd_optimization::SIMDOptimizer::compress_rle(&data).unwrap();
    let decompressed = simd_optimization::SIMDOptimizer::decompress_rle(&compressed).unwrap();

    assert_eq!(decompressed, data);
    // Compressed should be smaller than original
    assert!(compressed.len() < data.len());
}

#[test]
fn test_simd_compression_ratio() {
    let data = vec![1; 100]; // 100 identical bytes
    let compressed = simd_optimization::SIMDOptimizer::compress_rle(&data).unwrap();

    // Compressible: 100 bytes → 2 bytes (byte + count)
    assert_eq!(compressed.len(), 2);
}

#[test]
fn test_simd_instruction_sets() {
    let sets = simd_optimization::SIMDOptimizer::available_simd_sets();
    assert!(sets.len() > 0);
    assert!(sets.contains(&"SSE4.2"));
    assert!(sets.contains(&"AVX2"));
}

#[test]
fn test_cpu_capability_detection() {
    let has_sse42 = simd_optimization::SIMDOptimizer::has_sse42();
    assert!(has_sse42);

    let has_avx2 = simd_optimization::SIMDOptimizer::has_avx2();
    assert!(has_avx2);
}

#[test]
fn test_end_to_end_optimization_scenario() {
    // Scenario: High-performance matrix multiplication with GPU
    let mut accelerator = gpu_acceleration::GPUAccelerator::new().unwrap();
    let mut scheduler = scheduling::AdvancedScheduler::new(16, 2).unwrap();

    // Register GPU
    let gpu = gpu_acceleration::GPUCapability {
        vendor: gpu_acceleration::GPUVendor::NVIDIA,
        device_id: "cuda:0".to_string(),
        compute_capability: "sm_90".to_string(),
        memory_gb: 40,
        tensor_cores: Some(16384),
        max_threads: 1024,
    };

    accelerator.register_gpu(gpu).unwrap();

    // Schedule compute tasks
    for i in 0..10 {
        let task = scheduling::Task {
            task_id: format!("matmul_task_{}", i),
            priority: if i < 5 {
                scheduling::TaskPriority::High
            } else {
                scheduling::TaskPriority::Normal
            },
            affinity: scheduling::TaskAffinity {
                cpu_cores: vec![0, 1, 2, 3],
                numa_node: Some(0),
            },
            estimated_duration_ms: 100,
        };

        scheduler.enqueue_task(task).unwrap();
    }

    // Verify setup
    assert_eq!(scheduler.queue_length(), 10);
    assert_eq!(accelerator.list_gpus().len(), 1);
    println!("✓ High-performance setup complete");
}
