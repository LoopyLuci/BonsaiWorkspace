use crate::types::*;
use sysinfo::System;
use anyhow::Result;

/// Detect all available hardware on this system.
pub fn detect_hardware() -> Result<HardwareProfile> {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Detect CPU
    let cpu_profile = detect_cpu(&sys)?;

    // Detect GPUs
    let gpus = detect_gpus()?;

    // Detect memory
    let memory = MemoryProfile {
        total_bytes: sys.total_memory(),
        available_bytes: sys.available_memory(),
    };

    Ok(HardwareProfile {
        cpu: cpu_profile,
        gpus,
        memory,
    })
}

fn detect_cpu(sys: &System) -> Result<CpuProfile> {
    let cpu = sys.cpus().first()
        .ok_or_else(|| anyhow::anyhow!("No CPU detected"))?;

    let physical_cores = sys.physical_core_count().unwrap_or(1) as u32;
    let logical_cores = sys.cpus().len() as u32;

    Ok(CpuProfile {
        vendor: detect_cpu_vendor(),
        model: cpu.brand().to_string(),
        physical_cores,
        logical_cores,
        frequency_mhz: cpu.frequency() as u64,
        cache_l3_mb: detect_l3_cache(),
        simd_features: detect_simd_features(),
    })
}

fn detect_cpu_vendor() -> String {
    #[cfg(target_arch = "x86_64")]
    {
        if std::env::consts::OS == "windows" {
            "Intel/AMD (x86_64)".to_string()
        } else {
            "Intel/AMD (x86_64)".to_string()
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        #[cfg(target_os = "macos")]
        { "Apple (ARM)".to_string() }
        #[cfg(not(target_os = "macos"))]
        { "ARM64".to_string() }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    { "Unknown".to_string() }
}

fn detect_simd_features() -> Vec<String> {
    let mut features = Vec::new();

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            features.push("avx2".to_string());
        }
        if is_x86_feature_detected!("avx512f") {
            features.push("avx512f".to_string());
        }
        if is_x86_feature_detected!("fma") {
            features.push("fma".to_string());
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        features.push("neon".to_string());
    }

    features
}

fn detect_l3_cache() -> u64 {
    // Typical values: Intel = 8-36MB, AMD = 64MB+
    #[cfg(target_arch = "x86_64")]
    { 32 } // Default estimate
    #[cfg(target_arch = "aarch64")]
    { 16 }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    { 4 }
}

fn detect_gpus() -> Result<Vec<GpuProfile>> {
    #[allow(unused_mut)]
    let mut gpus = Vec::new();

    // Try CUDA (NVIDIA)
    #[cfg(feature = "cuda")]
    {
        if let Ok(cuda_gpus) = detect_nvidia_gpus() {
            gpus.extend(cuda_gpus);
        }
    }

    // Try ROCm (AMD)
    #[cfg(feature = "rocm")]
    {
        if let Ok(rocm_gpus) = detect_amd_gpus() {
            gpus.extend(rocm_gpus);
        }
    }

    // Try Metal (Apple)
    #[cfg(target_os = "macos")]
    {
        if let Ok(metal_gpus) = detect_metal_gpus() {
            gpus.extend(metal_gpus);
        }
    }

    // Try DirectML (Windows)
    #[cfg(feature = "directml")]
    {
        if let Ok(dml_gpus) = detect_directml_gpus() {
            gpus.extend(dml_gpus);
        }
    }

    // Fallback: Try Vulkan
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        if let Ok(vulkan_gpus) = detect_vulkan_gpus() {
            gpus.extend(vulkan_gpus);
        }
    }

    Ok(gpus)
}

#[allow(dead_code)]
#[cfg(feature = "cuda")]
fn detect_nvidia_gpus() -> Result<Vec<GpuProfile>> {
    // In production: use nvml_wrapper or cuda-sys to query NVIDIA GPUs
    // For now, return empty (safe fallback)
    Ok(vec![])
}

#[allow(dead_code)]
#[cfg(feature = "rocm")]
fn detect_amd_gpus() -> Result<Vec<GpuProfile>> {
    // In production: use rocm-smi or hipGetDeviceCount
    Ok(vec![])
}

#[allow(dead_code)]
#[cfg(target_os = "macos")]
fn detect_metal_gpus() -> Result<Vec<GpuProfile>> {
    // Metal is always available on macOS with a GPU
    // For now, return a generic entry
    Ok(vec![
        GpuProfile {
            index: 0,
            name: "Apple Metal (Integrated)".to_string(),
            vram_bytes: 4_000_000_000, // Shared system memory
            compute_units: 4,
            backend: GpuBackend::Metal,
            supports_fp16: true,
            supports_bf16: false,
            supports_int8: true,
        }
    ])
}

#[allow(dead_code)]
#[cfg(feature = "directml")]
fn detect_directml_gpus() -> Result<Vec<GpuProfile>> {
    // DirectML can use any GPU on Windows
    // For now, return empty (would need WinAPI calls)
    Ok(vec![])
}

#[allow(dead_code)]
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn detect_vulkan_gpus() -> Result<Vec<GpuProfile>> {
    // Vulkan is a universal backend
    // For now, return empty (would need Vulkan library)
    Ok(vec![])
}

#[allow(dead_code)]
#[cfg(not(feature = "cuda"))]
fn detect_nvidia_gpus() -> Result<Vec<GpuProfile>> {
    Ok(vec![])
}

#[allow(dead_code)]
#[cfg(not(feature = "rocm"))]
fn detect_amd_gpus() -> Result<Vec<GpuProfile>> {
    Ok(vec![])
}

#[allow(dead_code)]
#[cfg(not(target_os = "macos"))]
fn detect_metal_gpus() -> Result<Vec<GpuProfile>> {
    Ok(vec![])
}

#[allow(dead_code)]
#[cfg(not(feature = "directml"))]
fn detect_directml_gpus() -> Result<Vec<GpuProfile>> {
    Ok(vec![])
}

#[allow(dead_code)]
#[cfg(not(all(not(target_os = "windows"), not(target_os = "macos"))))]
fn detect_vulkan_gpus() -> Result<Vec<GpuProfile>> {
    Ok(vec![])
}
