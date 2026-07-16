//! omnisystem-async CLI: exercises the task executor, spawn helpers, and an
//! async lock to demonstrate the crate end to end.

use async_trait::async_trait;
use omnisystem_async::{spawn, timeout, AsyncLock, AsyncTask, TaskExecutor};
use std::sync::Arc;
use std::time::Duration;

struct GreetTask;

#[async_trait]
impl AsyncTask for GreetTask {
    async fn execute(&self) -> Result<String, String> {
        Ok("hello from GreetTask".to_string())
    }

    fn name(&self) -> &str {
        "greet"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = TaskExecutor::new();
    let task_id = executor.register_task(Arc::new(GreetTask));
    let task = executor.get_task(task_id).expect("task just registered");
    println!("Task '{}' says: {}", task.name(), task.execute().await?);
    println!("Executor has {} task(s)", executor.task_count());

    let handle = spawn(async { 21 * 2 });
    println!("Spawned task result: {}", handle.await?);

    let lock = AsyncLock::new(0u32);
    {
        let mut guard = lock.lock().await;
        *guard += 1;
    }
    println!("Lock value after increment: {}", *lock.lock().await);

    let timed_out = timeout(Duration::from_millis(10), async {
        tokio::time::sleep(Duration::from_secs(5)).await;
    })
    .await;
    println!("10ms timeout over a 5s sleep elapsed: {}", timed_out.is_err());

    Ok(())
}
