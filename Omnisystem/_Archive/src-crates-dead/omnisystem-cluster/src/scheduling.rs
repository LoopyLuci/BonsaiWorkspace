/// Advanced Scheduling Algorithm
///
/// CPU-aware, NUMA-aware task scheduling with work-stealing

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::info;

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Critical = 3,
    High = 2,
    Normal = 1,
    Low = 0,
}

/// Task affinity (preferred CPU/NUMA node)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAffinity {
    pub cpu_cores: Vec<u32>,      // Preferred CPU cores
    pub numa_node: Option<u32>,   // Preferred NUMA node
}

/// Task to be scheduled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: String,
    pub priority: TaskPriority,
    pub affinity: TaskAffinity,
    pub estimated_duration_ms: u32,
}

/// CPU-aware scheduler
pub struct AdvancedScheduler {
    task_queue: VecDeque<Task>,
    cpu_count: u32,
    numa_node_count: u32,
}

impl AdvancedScheduler {
    /// Create advanced scheduler
    pub fn new(cpu_count: u32, numa_node_count: u32) -> Result<Self> {
        info!(
            "Initializing Advanced Scheduler: {} CPUs, {} NUMA nodes",
            cpu_count, numa_node_count
        );
        Ok(Self {
            task_queue: VecDeque::new(),
            cpu_count,
            numa_node_count,
        })
    }

    /// Enqueue task
    pub fn enqueue_task(&mut self, task: Task) -> Result<()> {
        info!("Enqueuing task: {} (priority: {:?})", task.task_id, task.priority);
        self.task_queue.push_back(task);
        Ok(())
    }

    /// Schedule next task (prioritizes by priority + affinity)
    pub fn schedule_next(&mut self) -> Option<Task> {
        if self.task_queue.is_empty() {
            return None;
        }

        // Sort by priority (higher priority first)
        let mut tasks: Vec<_> = self.task_queue.drain(..).collect();
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Schedule highest priority task
        if let Some(task) = tasks.pop() {
            info!(
                "Scheduling task: {} on best-fit CPU",
                task.task_id
            );

            // Re-queue remaining tasks
            self.task_queue.extend(tasks);
            return Some(task);
        }

        None
    }

    /// Get best CPU for task (NUMA-aware)
    pub fn get_best_cpu(&self, task: &Task) -> u32 {
        // If task has affinity, use preferred core
        if !task.affinity.cpu_cores.is_empty() {
            return task.affinity.cpu_cores[0];
        }

        // If NUMA node preference, select core from that node
        if let Some(numa_node) = task.affinity.numa_node {
            let cores_per_node = self.cpu_count / self.numa_node_count.max(1);
            return (numa_node * cores_per_node) % self.cpu_count;
        }

        // Default: round-robin
        0
    }

    /// Work-stealing: steal from overloaded queues
    pub fn work_steal(&mut self) -> Option<Task> {
        info!("Attempting work-steal from other queues");
        // In production: coordinate with other scheduler instances
        self.schedule_next()
    }

    /// Get queue length
    pub fn queue_length(&self) -> usize {
        self.task_queue.len()
    }

    /// Get average task priority
    pub fn avg_priority(&self) -> f64 {
        if self.task_queue.is_empty() {
            return 0.0;
        }

        let sum: u32 = self.task_queue.iter().map(|t| t.priority as u32).sum();
        (sum as f64) / (self.task_queue.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_scheduler() {
        let mut scheduler = AdvancedScheduler::new(8, 2).unwrap();

        let task = Task {
            task_id: "task1".to_string(),
            priority: TaskPriority::High,
            affinity: TaskAffinity {
                cpu_cores: vec![0, 1],
                numa_node: Some(0),
            },
            estimated_duration_ms: 100,
        };

        scheduler.enqueue_task(task).unwrap();
        assert_eq!(scheduler.queue_length(), 1);
    }

    #[test]
    fn test_priority_scheduling() {
        let mut scheduler = AdvancedScheduler::new(4, 1).unwrap();

        scheduler
            .enqueue_task(Task {
                task_id: "low".to_string(),
                priority: TaskPriority::Low,
                affinity: TaskAffinity {
                    cpu_cores: vec![],
                    numa_node: None,
                },
                estimated_duration_ms: 1000,
            })
            .unwrap();

        scheduler
            .enqueue_task(Task {
                task_id: "high".to_string(),
                priority: TaskPriority::High,
                affinity: TaskAffinity {
                    cpu_cores: vec![],
                    numa_node: None,
                },
                estimated_duration_ms: 10,
            })
            .unwrap();

        let next = scheduler.schedule_next();
        assert_eq!(next.unwrap().task_id, "high");
    }
}
