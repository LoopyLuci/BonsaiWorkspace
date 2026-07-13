use crate::{GpuDevice, ShaderProgram, KernelExecution, KernelResult, GpuError, GpuResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct GpuKernelManager {
    devices: Arc<DashMap<u32, GpuDevice>>,
    shaders: Arc<DashMap<Uuid, ShaderProgram>>,
    kernels: Arc<DashMap<Uuid, KernelExecution>>,
}

impl GpuKernelManager {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(DashMap::new()),
            shaders: Arc::new(DashMap::new()),
            kernels: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_device(&self, device: GpuDevice) -> GpuResult<()> {
        self.devices.insert(device.device_id, device);
        Ok(())
    }

    pub async fn compile_shader(&self, source: &str) -> GpuResult<Uuid> {
        let program_id = Uuid::new_v4();

        if source.is_empty() {
            return Err(GpuError::CompilationFailed);
        }

        let shader = ShaderProgram {
            program_id,
            source: source.to_string(),
            compiled: true,
            optimization_level: 2,
        };

        self.shaders.insert(program_id, shader);
        Ok(program_id)
    }

    pub async fn execute_kernel(
        &self,
        program_id: Uuid,
        device_id: u32,
        grid_size: (u32, u32, u32),
        block_size: (u32, u32, u32),
    ) -> GpuResult<KernelResult> {
        if !self.devices.contains_key(&device_id) {
            return Err(GpuError::DeviceNotFound);
        }

        if !self.shaders.contains_key(&program_id) {
            return Err(GpuError::KernelNotFound);
        }

        let kernel_id = Uuid::new_v4();
        let execution = KernelExecution {
            kernel_id,
            grid_size,
            block_size,
            execution_time_us: 100,
        };

        self.kernels.insert(kernel_id, execution);

        Ok(KernelResult {
            kernel_id,
            device_id,
            execution_time_us: 100,
            success: true,
        })
    }

    pub async fn get_kernel_result(&self, kernel_id: Uuid) -> GpuResult<KernelResult> {
        if let Some(kernel) = self.kernels.get(&kernel_id) {
            Ok(KernelResult {
                kernel_id,
                device_id: 0,
                execution_time_us: kernel.execution_time_us,
                success: true,
            })
        } else {
            Err(GpuError::KernelNotFound)
        }
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn shader_count(&self) -> usize {
        self.shaders.len()
    }
}

impl Default for GpuKernelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_device() {
        let manager = GpuKernelManager::new();
        let device = GpuDevice {
            device_id: 0,
            name: "NVIDIA RTX 3090".to_string(),
            memory_mb: 24576,
            compute_capability: "8.6".to_string(),
            available: true,
        };

        let result = manager.register_device(device).await;
        assert!(result.is_ok());
        assert_eq!(manager.device_count(), 1);
    }

    #[tokio::test]
    async fn test_compile_shader() {
        let manager = GpuKernelManager::new();
        let source = "kernel void add(global float *a, global float *b) { }";
        let program_id = manager.compile_shader(source).await.unwrap();

        assert!(manager.shaders.contains_key(&program_id));
    }

    #[tokio::test]
    async fn test_execute_kernel() {
        let manager = GpuKernelManager::new();

        let device = GpuDevice {
            device_id: 0,
            name: "GPU0".to_string(),
            memory_mb: 4096,
            compute_capability: "7.0".to_string(),
            available: true,
        };
        manager.register_device(device).await.unwrap();

        let source = "kernel void compute() { }";
        let program_id = manager.compile_shader(source).await.unwrap();

        let result = manager
            .execute_kernel(program_id, 0, (256, 1, 1), (256, 1, 1))
            .await
            .unwrap();

        assert!(result.success);
    }
}
