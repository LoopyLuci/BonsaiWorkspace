//! Real-time compilation monitoring

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileStats {
    pub total_compilations: u64,
    pub successful_compilations: u64,
    pub failed_compilations: u64,
    pub total_time_seconds: u64,
    pub average_time_seconds: f64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
}

impl CompileStats {
    /// Create empty stats
    pub fn new() -> Self {
        Self {
            total_compilations: 0,
            successful_compilations: 0,
            failed_compilations: 0,
            total_time_seconds: 0,
            average_time_seconds: 0.0,
            cache_hits: 0,
            cache_misses: 0,
            cache_hit_rate: 0.0,
        }
    }

    /// Update stats
    pub fn record_compilation(&mut self, duration_secs: u64, success: bool, cache_hit: bool) {
        self.total_compilations += 1;

        if success {
            self.successful_compilations += 1;
        } else {
            self.failed_compilations += 1;
        }

        self.total_time_seconds += duration_secs;
        self.average_time_seconds = self.total_time_seconds as f64 / self.total_compilations as f64;

        if cache_hit {
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }

        let total_cache_ops = self.cache_hits + self.cache_misses;
        self.cache_hit_rate = if total_cache_ops > 0 {
            self.cache_hits as f64 / total_cache_ops as f64
        } else {
            0.0
        };
    }
}

/// Real-time compilation monitor
pub struct CompileMonitor {
    stats: parking_lot::Mutex<CompileStats>,
    current_compile: parking_lot::Mutex<Option<CurrentCompile>>,
}

#[derive(Debug, Clone)]
struct CurrentCompile {
    start_time: std::time::Instant,
    project: String,
}

impl CompileMonitor {
    /// Create new monitor
    pub fn new() -> Self {
        Self {
            stats: parking_lot::Mutex::new(CompileStats::new()),
            current_compile: parking_lot::Mutex::new(None),
        }
    }

    /// Start compilation
    pub fn start_compile(&self, project: String) {
        *self.current_compile.lock() = Some(CurrentCompile {
            start_time: std::time::Instant::now(),
            project,
        });

        log::info!("Starting compilation");
    }

    /// Complete compilation
    pub fn complete_compile(&self, success: bool, cache_hit: bool) {
        if let Some(compile) = self.current_compile.lock().take() {
            let duration = compile.start_time.elapsed().as_secs();
            let mut stats = self.stats.lock();
            stats.record_compilation(duration, success, cache_hit);

            log::info!(
                "Compilation complete: {} ({}s, cache: {})",
                if success { "✓" } else { "✗" },
                duration,
                if cache_hit { "HIT" } else { "MISS" }
            );
        }
    }

    /// Get current stats
    pub fn stats(&self) -> CompileStats {
        self.stats.lock().clone()
    }

    /// Print stats
    pub fn print_stats(&self) {
        let stats = self.stats.lock();
        println!("\n=== Compilation Statistics ===");
        println!("Total compilations:     {}", stats.total_compilations);
        println!("Successful:             {}", stats.successful_compilations);
        println!("Failed:                 {}", stats.failed_compilations);
        println!("Total time:             {}s", stats.total_time_seconds);
        println!("Average time:           {:.2}s", stats.average_time_seconds);
        println!("Cache hits:             {}", stats.cache_hits);
        println!("Cache misses:           {}", stats.cache_misses);
        println!("Cache hit rate:         {:.1}%", stats.cache_hit_rate * 100.0);
    }
}


impl Default for CompileMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() {
        let monitor = CompileMonitor::new();
        let stats = monitor.stats();
        assert_eq!(stats.total_compilations, 0);
    }

    #[test]
    fn test_compilation_recording() {
        let monitor = CompileMonitor::new();
        monitor.start_compile("test".to_string());
        std::thread::sleep(std::time::Duration::from_millis(10));
        monitor.complete_compile(true, false);

        let stats = monitor.stats();
        assert_eq!(stats.total_compilations, 1);
        assert_eq!(stats.successful_compilations, 1);
    }
}
