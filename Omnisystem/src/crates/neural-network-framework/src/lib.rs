//! Neural Network Framework - Bleeding Edge, Enterprise Grade
//!
//! A high-performance deep learning framework with:
//! - Automatic differentiation
//! - GPU/TPU support
//! - Distributed training
//! - Production serving
//! - Full Omnisystem integration

pub mod tensor;
pub mod graph;
pub mod ops;
pub mod autodiff;
pub mod execution;
pub mod error;
pub mod types;

// Re-export commonly used items
pub use tensor::Tensor;
pub use graph::ComputationGraph;
pub use ops::OperationRegistry;
pub use autodiff::AutoDiff;
pub use execution::ExecutionEngine;
pub use error::{Error, Result};
pub use types::TensorType;

/// Framework version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the neural network framework
pub fn initialize() -> Result<()> {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    // Initialize device discovery
    execution::device::discover_devices()?;

    log::info!("Neural Network Framework v{} initialized", VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_initialization() {
        let result = initialize();
        assert!(result.is_ok());
    }
}
