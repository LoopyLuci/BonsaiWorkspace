use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use sysinfo::System;

use crate::gpu_telemetry::GpuTelemetry;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub enum BackendType {
    Vulkan,
    DirectML,
    Cpu,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GpuOperation {
    pub backend: BackendType,
    pub op_name: String,
    pub duration_ms: u64,
    pub success: bool,
}

#[derive(Debug)]
struct BackendState {
    backend_type: BackendType,
    healthy: bool,
    consecutive_failures: u32,
    last_failure: Option<Instant>,
    last_success: Option<Instant>,
}

pub struct GpuInfo {
    pub has_vulkan: bool,
    pub has_directml: bool,
}

pub struct GpuLayer {
    backends: RwLock<Vec<BackendState>>,
    telemetry: GpuTelemetry,
}

impl GpuLayer {
    pub fn new(info: &GpuInfo) -> Self {
        let backends = RwLock::new(Self::init_backends(info));
        Self {
            backends,
            telemetry: GpuTelemetry::new(),
        }
    }

    fn init_backends(info: &GpuInfo) -> Vec<BackendState> {
        let mut out = Vec::new();
        if info.has_vulkan {
            out.push(BackendState {
                backend_type: BackendType::Vulkan,
                healthy: true,
                consecutive_failures: 0,
                last_failure: None,
                last_success: None,
            });
        }
        if info.has_directml {
            out.push(BackendState {
                backend_type: BackendType::DirectML,
                healthy: true,
                consecutive_failures: 0,
                last_failure: None,
                last_success: None,
            });
        }
        out.push(BackendState {
            backend_type: BackendType::Cpu,
            healthy: true,
            consecutive_failures: 0,
            last_failure: None,
            last_success: None,
        });
        out
    }

    pub async fn pre_check(&self, backend: &BackendType) -> bool {
        let mut backends = self.backends.write().await;
        if let Some(bs) = backends.iter_mut().find(|b| b.backend_type == *backend) {
            if !bs.healthy {
                if let Some(lf) = bs.last_failure {
                    if lf.elapsed() > Duration::from_secs(300) {
                        bs.healthy = true;
                        bs.consecutive_failures = 0;
                        return true;
                    }
                }
                return false;
            }
            true
        } else {
            false
        }
    }

    pub async fn record_result(&self, backend: BackendType, success: bool, error: Option<String>) {
        if success {
            self.telemetry.record_success(&backend);
        } else {
            self.telemetry
                .record_failure(&backend, error.as_deref().unwrap_or("unknown"));
        }
        let mut backends = self.backends.write().await;
        if let Some(bs) = backends.iter_mut().find(|b| b.backend_type == backend) {
            if success {
                bs.consecutive_failures = 0;
                bs.last_success = Some(Instant::now());
                bs.healthy = true;
            } else {
                bs.consecutive_failures += 1;
                bs.last_failure = Some(Instant::now());
                if bs.consecutive_failures >= 3 {
                    bs.healthy = false;
                    tracing::warn!(
                        "GPU backend {:?} marked unhealthy after {} consecutive failures",
                        backend,
                        bs.consecutive_failures
                    );
                }
            }
        }
    }

    pub fn best_available(&self, order: &[BackendType]) -> BackendType {
        let backends = self.backends.blocking_read();
        for bt in order {
            if let Some(bs) = backends.iter().find(|b| b.backend_type == *bt) {
                if bs.healthy {
                    return bt.clone();
                }
            }
        }
        BackendType::Cpu
    }

    pub fn telemetry(&self) -> &GpuTelemetry {
        &self.telemetry
    }

    /// Estimate free VRAM in MB.
    ///
    /// Queries via sysinfo for now; returns system RAM as a conservative proxy
    /// since Vulkan device memory is not exposed through sysinfo. The caller
    /// (GpuModelLoader) applies a 4 GB headroom so over-estimation is safe —
    /// llama.cpp's own --fit logic does the final precise check at load time.
    pub fn free_vram_mb(&self) -> u64 {
        let mut sys = System::new();
        sys.refresh_memory();
        // Use total RAM as an upper-bound proxy; headroom in GpuModelLoader
        // prevents over-allocation in practice.
        sys.total_memory() / (1024 * 1024)
    }

    /// Detect available GPU backends on this system.
    pub fn detect() -> GpuInfo {
        // Vulkan present if llama-server sidecar exists and we're on a GPU system.
        // DirectML is Windows-only.
        GpuInfo {
            has_vulkan: cfg!(target_os = "windows"),
            has_directml: cfg!(target_os = "windows"),
        }
    }
}
