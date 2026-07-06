pub mod coordinator;
pub mod types;
pub mod catalog;

pub use coordinator::CoordinatorActor;
pub use types::{ComputeNode, ComputeProject, FabricTask, TaskResult, TaskStatus, TaskType};
pub use catalog::{
    CATALOG, TaskProfile, TaskCategory, SchedulingStrategy, GpuRequirement,
    NetworkClass, DataVolume, ResourceProfile,
};
