// SYSTEMS FRAMEWORK - OS and systems programming
// Process management, filesystem, networking, threading
// Version: 2.0

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Child, Stdio};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};

/// Process information
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub state: ProcessState,
    pub memory_usage: u64,
    pub cpu_usage: f32,
}

#[derive(Debug, Clone)]
pub enum ProcessState {
    Running,
    Sleeping,
    Stopped,
    Zombie,
}

/// Process manager
pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<u32, ProcessInfo>>>,
    next_pid: Arc<Mutex<u32>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        ProcessManager {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_pid: Arc::new(Mutex::new(1000)),
        }
    }

    pub fn spawn(&self, command: &str, args: &[&str]) -> Result<ProcessHandle, ProcessError> {
        let mut cmd = Command::new(command);
        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn()
            .map_err(|e| ProcessError::SpawnFailed(e.to_string()))?;

        let pid = child.id();

        let process_info = ProcessInfo {
            pid,
            name: command.to_string(),
            state: ProcessState::Running,
            memory_usage: 0,
            cpu_usage: 0.0,
        };

        let mut procs = self.processes.lock().unwrap();
        procs.insert(pid, process_info);

        Ok(ProcessHandle {
            child: Arc::new(Mutex::new(child)),
            pid,
        })
    }

    pub fn list_processes(&self) -> Vec<ProcessInfo> {
        let procs = self.processes.lock().unwrap();
        procs.values().cloned().collect()
    }

    pub fn get_process(&self, pid: u32) -> Option<ProcessInfo> {
        let procs = self.processes.lock().unwrap();
        procs.get(&pid).cloned()
    }

    pub fn kill_process(&self, pid: u32) -> Result<(), ProcessError> {
        let mut procs = self.processes.lock().unwrap();
        procs.remove(&pid).ok_or(ProcessError::ProcessNotFound)?;
        Ok(())
    }
}

/// Process handle
pub struct ProcessHandle {
    child: Arc<Mutex<Child>>,
    pid: u32,
}

impl ProcessHandle {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn wait(&self) -> Result<ExitStatus, ProcessError> {
        let mut child = self.child.lock().unwrap();
        let status = child.wait()
            .map_err(|e| ProcessError::WaitFailed(e.to_string()))?;

        Ok(ExitStatus {
            code: status.code(),
            success: status.success(),
        })
    }

    pub fn kill(&mut self) -> Result<(), ProcessError> {
        let mut child = self.child.lock().unwrap();
        child.kill()
            .map_err(|e| ProcessError::KillFailed(e.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ExitStatus {
    pub code: Option<i32>,
    pub success: bool,
}

/// Memory information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub total: u64,
    pub available: u64,
    pub used: u64,
    pub free: u64,
    pub percent_used: f32,
}

pub struct MemoryManager;

impl MemoryManager {
    pub fn get_memory_info() -> Result<MemoryInfo, MemoryError> {
        // Simplified implementation
        #[cfg(target_os = "linux")]
        {
            Self::get_linux_memory()
        }

        #[cfg(not(target_os = "linux"))]
        {
            Ok(MemoryInfo {
                total: 16 * 1024 * 1024 * 1024, // 16GB default
                available: 8 * 1024 * 1024 * 1024,
                used: 8 * 1024 * 1024 * 1024,
                free: 8 * 1024 * 1024 * 1024,
                percent_used: 50.0,
            })
        }
    }

    #[cfg(target_os = "linux")]
    fn get_linux_memory() -> Result<MemoryInfo, MemoryError> {
        let meminfo = std::fs::read_to_string("/proc/meminfo")
            .map_err(|e| MemoryError::ReadFailed(e.to_string()))?;

        let mut mem_total = 0u64;
        let mut mem_available = 0u64;

        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                mem_total = Self::parse_meminfo_line(line)?;
            } else if line.starts_with("MemAvailable:") {
                mem_available = Self::parse_meminfo_line(line)?;
            }
        }

        let used = mem_total - mem_available;
        let percent = if mem_total > 0 {
            (used as f32 / mem_total as f32) * 100.0
        } else {
            0.0
        };

        Ok(MemoryInfo {
            total: mem_total,
            available: mem_available,
            used,
            free: mem_available,
            percent_used: percent,
        })
    }

    fn parse_meminfo_line(line: &str) -> Result<u64, MemoryError> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            parts[1].parse::<u64>()
                .map_err(|_| MemoryError::ParseFailed)
        } else {
            Err(MemoryError::ParseFailed)
        }
    }
}

/// Filesystem operations
pub struct FileSystem;

impl FileSystem {
    pub fn read_file(path: &str) -> Result<Vec<u8>, FileError> {
        let mut file = File::open(path)
            .map_err(|e| FileError::OpenFailed(e.to_string()))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| FileError::ReadFailed(e.to_string()))?;

        Ok(contents)
    }

    pub fn read_file_to_string(path: &str) -> Result<String, FileError> {
        let contents = Self::read_file(path)?;
        String::from_utf8(contents)
            .map_err(|e| FileError::InvalidUtf8(e.to_string()))
    }

    pub fn write_file(path: &str, contents: &[u8]) -> Result<(), FileError> {
        let mut file = File::create(path)
            .map_err(|e| FileError::CreateFailed(e.to_string()))?;

        file.write_all(contents)
            .map_err(|e| FileError::WriteFailed(e.to_string()))?;

        Ok(())
    }

    pub fn write_file_string(path: &str, contents: &str) -> Result<(), FileError> {
        Self::write_file(path, contents.as_bytes())
    }

    pub fn append_file(path: &str, contents: &[u8]) -> Result<(), FileError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| FileError::OpenFailed(e.to_string()))?;

        file.write_all(contents)
            .map_err(|e| FileError::WriteFailed(e.to_string()))?;

        Ok(())
    }

    pub fn delete_file(path: &str) -> Result<(), FileError> {
        fs::remove_file(path)
            .map_err(|e| FileError::DeleteFailed(e.to_string()))?;
        Ok(())
    }

    pub fn copy_file(from: &str, to: &str) -> Result<(), FileError> {
        fs::copy(from, to)
            .map_err(|e| FileError::CopyFailed(e.to_string()))?;
        Ok(())
    }

    pub fn exists(path: &str) -> bool {
        Path::new(path).exists()
    }

    pub fn is_file(path: &str) -> bool {
        Path::new(path).is_file()
    }

    pub fn is_dir(path: &str) -> bool {
        Path::new(path).is_dir()
    }

    pub fn file_size(path: &str) -> Result<u64, FileError> {
        let metadata = fs::metadata(path)
            .map_err(|e| FileError::MetadataFailed(e.to_string()))?;
        Ok(metadata.len())
    }

    pub fn list_dir(path: &str) -> Result<Vec<String>, FileError> {
        let entries = fs::read_dir(path)
            .map_err(|e| FileError::ListFailed(e.to_string()))?;

        let mut files = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    files.push(name.to_string());
                }
            }
        }

        Ok(files)
    }

    pub fn create_dir(path: &str) -> Result<(), FileError> {
        fs::create_dir(path)
            .map_err(|e| FileError::CreateDirFailed(e.to_string()))?;
        Ok(())
    }

    pub fn create_dir_all(path: &str) -> Result<(), FileError> {
        fs::create_dir_all(path)
            .map_err(|e| FileError::CreateDirFailed(e.to_string()))?;
        Ok(())
    }

    pub fn delete_dir(path: &str) -> Result<(), FileError> {
        fs::remove_dir(path)
            .map_err(|e| FileError::DeleteDirFailed(e.to_string()))?;
        Ok(())
    }

    pub fn delete_dir_all(path: &str) -> Result<(), FileError> {
        fs::remove_dir_all(path)
            .map_err(|e| FileError::DeleteDirFailed(e.to_string()))?;
        Ok(())
    }
}

/// Threading utilities
pub struct ThreadPool {
    workers: Vec<Worker>,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Result<ThreadPool, ThreadError> {
        if size == 0 {
            return Err(ThreadError::InvalidSize);
        }

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker {
                id,
                thread: None,
            });
        }

        Ok(ThreadPool { workers })
    }

    pub fn execute<F>(&mut self, f: F) -> Result<(), ThreadError>
    where
        F: FnOnce() + Send + 'static,
    {
        if self.workers.is_empty() {
            return Err(ThreadError::NoWorkers);
        }

        let worker = &mut self.workers[0];
        let handle = thread::spawn(f);
        worker.thread = Some(handle);

        Ok(())
    }

    pub fn size(&self) -> usize {
        self.workers.len()
    }

    pub fn active_threads(&self) -> usize {
        self.workers.iter().filter(|w| w.thread.is_some()).count()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

/// Error types
#[derive(Debug)]
pub enum ProcessError {
    SpawnFailed(String),
    WaitFailed(String),
    KillFailed(String),
    ProcessNotFound,
}

#[derive(Debug)]
pub enum MemoryError {
    ReadFailed(String),
    ParseFailed,
}

#[derive(Debug)]
pub enum FileError {
    OpenFailed(String),
    ReadFailed(String),
    WriteFailed(String),
    CreateFailed(String),
    DeleteFailed(String),
    CopyFailed(String),
    MetadataFailed(String),
    ListFailed(String),
    CreateDirFailed(String),
    DeleteDirFailed(String),
    InvalidUtf8(String),
}

#[derive(Debug)]
pub enum ThreadError {
    InvalidSize,
    NoWorkers,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_manager() {
        let pm = ProcessManager::new();
        let processes = pm.list_processes();
        assert_eq!(processes.len(), 0);
    }

    #[test]
    fn test_memory_info() {
        let info = MemoryManager::get_memory_info().unwrap();
        assert!(info.total > 0);
        assert!(info.percent_used >= 0.0 && info.percent_used <= 100.0);
    }

    #[test]
    fn test_file_operations() {
        let test_file = "/tmp/omnisystem_test.txt";
        let content = b"Hello, Omnisystem!";

        assert!(FileSystem::write_file(test_file, content).is_ok());
        assert!(FileSystem::exists(test_file));
        assert!(FileSystem::is_file(test_file));

        let read_content = FileSystem::read_file(test_file).unwrap();
        assert_eq!(read_content, content);

        assert!(FileSystem::delete_file(test_file).is_ok());
        assert!(!FileSystem::exists(test_file));
    }

    #[test]
    fn test_thread_pool() {
        let mut pool = ThreadPool::new(4).unwrap();
        assert_eq!(pool.size(), 4);
        assert_eq!(pool.active_threads(), 0);
    }
}
