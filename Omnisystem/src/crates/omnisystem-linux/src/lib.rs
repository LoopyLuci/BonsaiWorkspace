//! Omnisystem Linux - Linux OS-integration modules
//!
//! Real availability detection (checks the actual kernel paths/devices)
//! plus real state management, with honest "would call the real API"
//! stubs where a genuine syscall/D-Bus/netlink implementation would be
//! required (loading eBPF bytecode, querying systemd over D-Bus, etc.):
//! - [`cgroup`]: resource limits via cgroups v1/v2
//! - [`ebpf`]: kernel tracing program lifecycle
//! - [`kvm`]: virtual machine lifecycle
//! - [`netlink`]: network interface/route configuration
//! - [`perf`]: performance counter monitoring
//! - [`systemd`]: service unit generation and management

pub mod cgroup;
pub mod ebpf;
pub mod error;
pub mod kvm;
pub mod netlink;
pub mod perf;
pub mod systemd;

pub use cgroup::{Cgroup, CgroupLimits, CgroupManager, CgroupVersion};
pub use ebpf::{AttachmentId, EBpfRuntime, ProgramId, Tracepoint};
pub use error::{LinuxError, Result};
pub use kvm::{KVMController, VMConfig, VMState, VirtualMachine};
pub use netlink::{InterfaceConfig, NetlinkSocket, NetworkInterface, Route};
pub use perf::{EventHandle, PerfData, PerfEvent, PerfMonitor};
pub use systemd::{RestartPolicy, ServiceStatus, ServiceUnit, SystemdManager};
