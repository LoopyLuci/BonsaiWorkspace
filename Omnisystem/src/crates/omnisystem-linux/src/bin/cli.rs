//! Omnisystem Linux CLI - probes availability of the Linux integration surfaces

use omnisystem_linux::{EBpfRuntime, KVMController, NetlinkSocket, PerfMonitor, SystemdManager};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kvm = KVMController::new()?;
    println!("KVM available: {} (max vcpus: {})", kvm.is_available(), kvm.max_vcpus());

    let ebpf = EBpfRuntime::new()?;
    println!("eBPF available: {}", ebpf.is_available());

    let systemd = SystemdManager::new()?;
    println!("systemd available: {}", systemd.is_available());

    let netlink = NetlinkSocket::new()?;
    println!("interfaces: {}", netlink.get_interfaces()?.len());

    let perf = PerfMonitor::new()?;
    println!("perf available: {}", perf.is_available());

    Ok(())
}
