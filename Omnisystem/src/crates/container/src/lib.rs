//! Bonsai Container Fabric (BCF) building blocks: a Kubernetes-style
//! Blueprint format (containers/services/volumes with real validation and
//! YAML/JSON (de)serialization), an in-memory BlueprintManager store, a
//! broadcast EventBus for deployment/container lifecycle events, and node
//! configuration.

pub mod blueprint;
pub mod config;
pub mod errors;
pub mod events;

pub use blueprint::{
    Blueprint, BlueprintManager, ContainerSpec, CpuPriority, ExecProbe, GpuResource,
    HealthProbes, HttpProbe, LoadBalancingPolicy, NetworkSpec, PersistentVolume, PortMapping,
    Probe, ResourceSpec, ServiceSpec, StorageSpec, UpdateStrategy, VolumeSpec, VolumeType,
};
pub use config::BcfConfig;
pub use errors::{BcfError, Result};
pub use events::{Event, EventBus};
