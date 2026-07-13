/// Scheduler bindings

pub use omnisystem_kernel::scheduling::Scheduler as KernelScheduler;

pub struct Scheduler;

impl Scheduler {
    pub fn total_ready_threads(kernel_sched: &KernelScheduler) -> usize {
        kernel_sched.total_ready_threads()
    }
}
