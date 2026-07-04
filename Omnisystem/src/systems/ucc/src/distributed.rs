//! Distributed Compilation Framework
//!
//! Orchestrates compilation across multiple remote worker nodes.
//! Handles work distribution, load balancing, and result aggregation.

use crate::error::Result;
use crate::core::{CompileTarget, CompileResult, BuildStats};
use crate::language::Language;
use std::path::PathBuf;
use std::net::SocketAddr;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Unique identifier for a compilation task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(uuid::Uuid);

impl TaskId {
    /// Generate a new unique task ID
    pub fn new() -> Self {
        TaskId(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a compilation task
#[derive(Debug, Clone)]
pub struct CompilationTask {
    pub id: TaskId,
    pub language: Language,
    pub sources: Vec<PathBuf>,
    pub target: CompileTarget,
    pub priority: u8,  // 0-255, higher = more urgent
    pub timestamp: std::time::SystemTime,
}

impl CompilationTask {
    /// Create a new compilation task
    pub fn new(
        language: Language,
        sources: Vec<PathBuf>,
        target: CompileTarget,
    ) -> Self {
        Self {
            id: TaskId::new(),
            language,
            sources,
            target,
            priority: 0,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Set task priority (0-255, higher = more urgent)
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

/// Information about a remote worker node
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub id: String,
    pub address: SocketAddr,
    pub max_concurrent_tasks: usize,
    pub supported_languages: Vec<Language>,
    pub available: bool,
    pub current_load: usize,
}

impl WorkerInfo {
    /// Create a new worker info
    pub fn new(
        address: SocketAddr,
        supported_languages: Vec<Language>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            address,
            max_concurrent_tasks: 4,
            supported_languages,
            available: true,
            current_load: 0,
        }
    }

    /// Check if worker can take more tasks
    pub fn can_take_task(&self) -> bool {
        self.available && self.current_load < self.max_concurrent_tasks
    }

    /// Get available capacity
    pub fn available_capacity(&self) -> usize {
        if self.available {
            self.max_concurrent_tasks.saturating_sub(self.current_load)
        } else {
            0
        }
    }

    /// Check if worker supports a language
    pub fn supports(&self, language: Language) -> bool {
        self.supported_languages.contains(&language)
    }
}

/// Tracks compilation results for a task
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub language: Language,
    pub success: bool,
    pub result: Option<CompileResult>,
    pub error: Option<String>,
    pub worker_id: String,
    pub duration_ms: u128,
}

/// Coordinates distributed compilation across worker nodes
pub struct BuildCoordinator {
    /// Pending tasks waiting for assignment
    work_queue: Arc<parking_lot::Mutex<std::collections::VecDeque<CompilationTask>>>,

    /// Registered worker nodes
    workers: DashMap<String, WorkerInfo>,

    /// Running tasks mapped by task ID
    running_tasks: DashMap<TaskId, (String, std::time::Instant)>,  // (worker_id, start_time)

    /// Completed task results
    results: DashMap<TaskId, TaskResult>,

    /// Project hash for cache coordination
    pub project_hash: String,
}

impl BuildCoordinator {
    /// Create a new build coordinator
    pub fn new(project_hash: String) -> Self {
        Self {
            work_queue: Arc::new(parking_lot::Mutex::new(std::collections::VecDeque::new()),),
            workers: DashMap::new(),
            running_tasks: DashMap::new(),
            results: DashMap::new(),
            project_hash,
        }
    }

    /// Register a worker node
    pub fn register_worker(&self, worker: WorkerInfo) -> Result<String> {
        let worker_id = worker.id.clone();
        self.workers.insert(worker_id.clone(), worker);
        Ok(worker_id)
    }

    /// Unregister a worker node
    pub fn unregister_worker(&self, worker_id: &str) {
        self.workers.remove(worker_id);
        // Mark any running tasks from this worker as failed
        let to_remove: Vec<_> = self
            .running_tasks
            .iter()
            .filter(|entry| entry.value().0 == worker_id)
            .map(|entry| *entry.key())
            .collect();

        for task_id in to_remove {
            self.running_tasks.remove(&task_id);
        }
    }

    /// Get all registered workers
    pub fn get_workers(&self) -> Vec<WorkerInfo> {
        self.workers.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Queue a compilation task
    pub fn queue_task(&self, task: CompilationTask) -> TaskId {
        let task_id = task.id;
        self.work_queue.lock().push_back(task);
        task_id
    }

    /// Get next task for a worker
    pub fn next_task_for_worker(&self, worker_id: &str) -> Option<CompilationTask> {
        let worker = self.workers.get(worker_id)?;

        // Check if worker can take tasks
        if !worker.can_take_task() {
            return None;
        }

        // Get next task that worker supports
        let mut queue = self.work_queue.lock();
        let task_index = queue.iter().position(|task| {
            worker.supports(task.language) && !self.running_tasks.contains_key(&task.id)
        })?;

        let task = queue.remove(task_index)?;
        drop(queue);  // Release lock before inserting into running_tasks

        // Mark as running
        self.running_tasks.insert(task.id, (worker_id.to_string(), std::time::Instant::now()));

        Some(task)
    }

    /// Report task completion
    pub fn complete_task(
        &self,
        task_id: TaskId,
        success: bool,
        result: Option<CompileResult>,
        error: Option<String>,
    ) -> Result<()> {
        if let Some((_, (worker_id, start_time))) = self.running_tasks.remove(&task_id) {
            let duration = start_time.elapsed().as_millis();

            let task_result = TaskResult {
                task_id,
                language: result.as_ref().map(|r| r.language).unwrap_or(Language::Rust),
                success,
                result: result.clone(),
                error,
                worker_id,
                duration_ms: duration,
            };

            self.results.insert(task_id, task_result);
        }

        Ok(())
    }

    /// Get a task result
    pub fn get_result(&self, task_id: TaskId) -> Option<TaskResult> {
        self.results.get(&task_id).map(|entry| entry.clone())
    }

    /// Get all results
    pub fn get_all_results(&self) -> Vec<TaskResult> {
        self.results.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Get statistics
    pub fn stats(&self) -> DistributedBuildStats {
        let all_results = self.get_all_results();
        let total_time: u128 = all_results.iter().map(|r| r.duration_ms).sum();

        DistributedBuildStats {
            total_tasks: all_results.len(),
            successful_tasks: all_results.iter().filter(|r| r.success).count(),
            failed_tasks: all_results.iter().filter(|r| !r.success).count(),
            total_time_ms: total_time,
            queued_tasks: self.work_queue.lock().len(),
            running_tasks: self.running_tasks.len(),
            worker_count: self.workers.len(),
            average_task_time_ms: if !all_results.is_empty() {
                total_time / all_results.len() as u128
            } else {
                0
            },
        }
    }

    /// Get least-loaded worker for a language
    pub fn get_least_loaded_worker(&self, language: Language) -> Option<String> {
        self.workers
            .iter()
            .filter(|entry| entry.value().supports(language) && entry.value().can_take_task())
            .min_by_key(|entry| entry.value().current_load)
            .map(|entry| entry.key().clone())
    }

    /// Convert results to BuildStats
    pub fn to_build_stats(&self) -> BuildStats {
        let stats = self.stats();
        let mut result = BuildStats::new();
        result.total_units = stats.total_tasks;
        result.compiled_units = stats.successful_tasks;
        result.failed_units = stats.failed_tasks;
        result.total_duration_ms = stats.total_time_ms;
        result.output = format!(
            "Distributed build: {} tasks on {} workers, {} succeeded, {} failed, {}ms",
            stats.total_tasks,
            stats.worker_count,
            stats.successful_tasks,
            stats.failed_tasks,
            stats.total_time_ms
        );
        result
    }
}

impl Default for BuildCoordinator {
    fn default() -> Self {
        Self::new("".to_string())
    }
}

/// Statistics for distributed builds
#[derive(Debug, Clone)]
pub struct DistributedBuildStats {
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub total_time_ms: u128,
    pub queued_tasks: usize,
    pub running_tasks: usize,
    pub worker_count: usize,
    pub average_task_time_ms: u128,
}

impl DistributedBuildStats {
    /// Calculate speedup vs sequential
    pub fn speedup(&self, sequential_time_ms: u128) -> f32 {
        if self.total_time_ms == 0 {
            1.0
        } else {
            sequential_time_ms as f32 / self.total_time_ms as f32
        }
    }

    /// Calculate efficiency (speedup / worker_count)
    pub fn efficiency(&self, sequential_time_ms: u128) -> f32 {
        if self.worker_count == 0 {
            0.0
        } else {
            self.speedup(sequential_time_ms) / self.worker_count as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_generation() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_worker_info_capacity() {
        let worker = WorkerInfo::new(
            "127.0.0.1:8080".parse().unwrap(),
            vec![Language::Rust, Language::Go],
        );
        assert!(worker.can_take_task());
        assert_eq!(worker.available_capacity(), 4);
    }

    #[test]
    fn test_coordinator_creation() {
        let coordinator = BuildCoordinator::new("hash123".to_string());
        assert_eq!(coordinator.workers.len(), 0);
        assert_eq!(coordinator.work_queue.lock().len(), 0);
    }

    #[test]
    fn test_worker_registration() {
        let coordinator = BuildCoordinator::new("hash123".to_string());
        let worker = WorkerInfo::new(
            "127.0.0.1:8080".parse().unwrap(),
            vec![Language::Rust],
        );

        let worker_id = worker.id.clone();
        coordinator.register_worker(worker).unwrap();
        assert!(coordinator.workers.contains_key(&worker_id));
    }

    #[test]
    fn test_task_queueing() {
        let coordinator = BuildCoordinator::new("hash123".to_string());
        let task = CompilationTask::new(
            Language::Rust,
            vec![PathBuf::from("main.rs")],
            CompileTarget::native(),
        );

        let task_id = coordinator.queue_task(task);
        assert_eq!(coordinator.work_queue.lock().len(), 1);
        assert!(!coordinator.running_tasks.contains_key(&task_id));
    }

    #[test]
    fn test_stats_calculation() {
        let coordinator = BuildCoordinator::new("hash123".to_string());
        let stats = coordinator.stats();
        assert_eq!(stats.total_tasks, 0);
        assert_eq!(stats.worker_count, 0);
    }
}
