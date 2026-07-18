//! CLI: create threads at different priorities, schedule the highest
//! priority thread, and adjust priority and accounted CPU time.

use cpu_scheduler::{CpuScheduler, Priority};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scheduler = CpuScheduler::new(4);

    scheduler.create_thread(1, Priority::Low, vec![0]).await?;
    scheduler.create_thread(2, Priority::Critical, vec![0, 1]).await?;
    scheduler.create_thread(3, Priority::Normal, vec![1]).await?;

    let decision = scheduler.schedule_next().await?;
    println!(
        "scheduled thread {} on core {} (priority {:?}, timeslice {}ms)",
        decision.thread_id, decision.cpu_core, decision.priority, decision.timeslice_ms
    );

    scheduler.set_thread_priority(1, Priority::High).await?;
    scheduler.update_thread_time(1, 25).await?;

    let thread = scheduler.get_thread_info(1).await?;
    println!(
        "thread 1 now priority {:?}, {}ms cpu time",
        thread.priority, thread.cpu_time_ms
    );

    println!("total threads tracked: {}", scheduler.thread_count());

    Ok(())
}
