//! CLI that exercises the omnisystem-runtime task scheduler/executor.

use omnisystem_runtime::{Priority, ResourcePool, RuntimeMetrics, Scheduler, Task, TaskExecutor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scheduler = Scheduler::new();
    let executor = TaskExecutor::new();
    let metrics = RuntimeMetrics::new();
    let pool = ResourcePool::new(100);

    let tasks = vec![
        Task::new("warmup".to_string(), Priority::Low),
        Task::new("ingest".to_string(), Priority::Normal),
        Task::new("failover".to_string(), Priority::Critical),
    ];

    for task in &tasks {
        pool.allocate(10)?;
        scheduler.schedule(task)?;
    }

    println!("Queued {} tasks", scheduler.queue_size());

    while let Some(id) = scheduler.next_task() {
        let task = tasks
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .unwrap_or_else(|| Task::new(id.clone(), Priority::Normal));
        executor.execute(task).await?;
        metrics.record_task_completed();
        pool.release(10);
        println!("Completed task: {id}");
    }

    let snapshot = metrics.snapshot();
    println!(
        "Runtime metrics: total={} completed={} failed={}",
        snapshot.total_tasks, snapshot.completed_tasks, snapshot.failed_tasks
    );
    println!("Resource pool: {:?}", pool.metrics());

    Ok(())
}
