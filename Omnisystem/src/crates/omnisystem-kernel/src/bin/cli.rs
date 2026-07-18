//! CLI demo for omnisystem-kernel: creates a process/thread, allocates
//! physical memory pages, and schedules the thread.

use omnisystem_kernel::{MemoryManager, ProcessManager, Scheduler};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pm = ProcessManager::new();
    let process = pm.create_process(None)?;
    let thread = pm.create_thread(process.id)?;
    println!(
        "Created process {} with thread {}",
        process.id, thread.id
    );

    let mm = MemoryManager::new()?;
    let pages = mm.allocate_pages(16)?;
    let stats = mm.get_stats();
    println!(
        "Allocated {} pages; {}/{} frames in use ({} bytes)",
        pages.len(),
        stats.allocated_frames,
        stats.total_frames,
        stats.allocated_memory_bytes
    );

    let scheduler = Scheduler::new();
    scheduler.add_thread(thread.clone())?;
    if let Some(next) = scheduler.schedule_next() {
        println!("Scheduler picked thread {} to run next", next.id);
    }

    Ok(())
}
