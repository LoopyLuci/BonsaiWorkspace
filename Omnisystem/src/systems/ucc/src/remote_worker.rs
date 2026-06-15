//! Remote Worker Protocol - TCP-based communication with worker nodes
//!
//! Defines serializable messages for distributed compilation coordination.

use crate::core::{CompileTarget, CompileResult};
use crate::language::Language;
use crate::distributed::CompilationTask;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Request message sent from coordinator to worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerRequest {
    /// Request build information from worker
    Ping,

    /// Get worker capabilities
    GetCapabilities,

    /// Compile sources
    Compile {
        task_id: String,
        language: Language,
        sources: Vec<PathBuf>,
        target: CompileTarget,
    },

    /// Check task status
    Status { task_id: String },

    /// Shutdown worker
    Shutdown,
}

/// Response message sent from worker to coordinator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerResponse {
    /// Acknowledge ping
    Pong,

    /// Worker capabilities
    Capabilities {
        worker_id: String,
        max_concurrent_tasks: usize,
        supported_languages: Vec<Language>,
    },

    /// Compilation complete
    CompileResult {
        task_id: String,
        success: bool,
        output: String,
        errors: Vec<String>,
    },

    /// Task status
    Status {
        task_id: String,
        is_running: bool,
        progress: f32,  // 0.0 to 1.0
    },

    /// Error response
    Error { message: String },
}

/// Worker connection information
#[derive(Debug, Clone)]
pub struct WorkerConnection {
    pub worker_id: String,
    pub address: std::net::SocketAddr,
    pub max_concurrent_tasks: usize,
    pub supported_languages: Vec<Language>,
}

impl WorkerConnection {
    /// Create a new worker connection
    pub fn new(
        worker_id: String,
        address: std::net::SocketAddr,
        supported_languages: Vec<Language>,
    ) -> Self {
        Self {
            worker_id,
            address,
            max_concurrent_tasks: 4,
            supported_languages,
        }
    }

    /// Check if worker supports a language
    pub fn supports(&self, language: Language) -> bool {
        self.supported_languages.contains(&language)
    }
}

/// Worker message serialization utilities
pub mod message {
    use super::*;

    /// Serialize a request to JSON bytes
    pub fn serialize_request(req: &WorkerRequest) -> Result<Vec<u8>, serde_json::Error> {
        Ok(serde_json::to_vec(req)?)
    }

    /// Deserialize a request from JSON bytes
    pub fn deserialize_request(data: &[u8]) -> Result<WorkerRequest, serde_json::Error> {
        serde_json::from_slice(data)
    }

    /// Serialize a response to JSON bytes
    pub fn serialize_response(res: &WorkerResponse) -> Result<Vec<u8>, serde_json::Error> {
        Ok(serde_json::to_vec(res)?)
    }

    /// Deserialize a response from JSON bytes
    pub fn deserialize_response(data: &[u8]) -> Result<WorkerResponse, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

/// Worker pool for managing multiple worker connections
#[derive(Debug)]
pub struct WorkerPool {
    workers: std::collections::HashMap<String, WorkerConnection>,
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new() -> Self {
        Self {
            workers: std::collections::HashMap::new(),
        }
    }

    /// Add a worker to the pool
    pub fn add_worker(&mut self, worker: WorkerConnection) {
        self.workers.insert(worker.worker_id.clone(), worker);
    }

    /// Remove a worker from the pool
    pub fn remove_worker(&mut self, worker_id: &str) -> Option<WorkerConnection> {
        self.workers.remove(worker_id)
    }

    /// Get a worker by ID
    pub fn get_worker(&self, worker_id: &str) -> Option<&WorkerConnection> {
        self.workers.get(worker_id)
    }

    /// Get all workers that support a language
    pub fn workers_for_language(&self, language: Language) -> Vec<&WorkerConnection> {
        self.workers
            .values()
            .filter(|w| w.supports(language))
            .collect()
    }

    /// Get number of workers
    pub fn count(&self) -> usize {
        self.workers.len()
    }

    /// Get total capacity
    pub fn total_capacity(&self) -> usize {
        self.workers.values().map(|w| w.max_concurrent_tasks).sum()
    }

    /// Get all workers
    pub fn all_workers(&self) -> Vec<&WorkerConnection> {
        self.workers.values().collect()
    }
}

impl Default for WorkerPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_request_serialization() {
        let req = WorkerRequest::Ping;
        let serialized = message::serialize_request(&req).unwrap();
        let deserialized = message::deserialize_request(&serialized).unwrap();

        match deserialized {
            WorkerRequest::Ping => {}
            _ => panic!("Expected Ping"),
        }
    }

    #[test]
    fn test_worker_response_serialization() {
        let res = WorkerResponse::Pong;
        let serialized = message::serialize_response(&res).unwrap();
        let deserialized = message::deserialize_response(&serialized).unwrap();

        match deserialized {
            WorkerResponse::Pong => {}
            _ => panic!("Expected Pong"),
        }
    }

    #[test]
    fn test_worker_connection_creation() {
        let conn = WorkerConnection::new(
            "worker-1".to_string(),
            "127.0.0.1:8080".parse().unwrap(),
            vec![Language::Rust, Language::Go],
        );

        assert!(conn.supports(Language::Rust));
        assert!(!conn.supports(Language::Zig));
    }

    #[test]
    fn test_worker_pool() {
        let mut pool = WorkerPool::new();
        let worker = WorkerConnection::new(
            "worker-1".to_string(),
            "127.0.0.1:8080".parse().unwrap(),
            vec![Language::Rust],
        );

        pool.add_worker(worker);
        assert_eq!(pool.count(), 1);
        assert_eq!(pool.total_capacity(), 4);

        let rust_workers = pool.workers_for_language(Language::Rust);
        assert_eq!(rust_workers.len(), 1);
    }
}
