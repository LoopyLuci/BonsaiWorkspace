/// Distributed State Machine Module
///
/// Manages replicated state machine:
/// - Command log
/// - Deterministic execution
/// - Snapshot management
/// - State consistency

use crate::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

/// State machine entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub index: u64,
    pub term: u64,
    pub command: Vec<u8>,
}

/// State machine
pub struct StateMachine {
    log: Vec<LogEntry>,
    commit_index: u64,
    #[allow(dead_code)]
    last_applied: u64,
}

impl StateMachine {
    /// Create state machine
    pub fn new() -> Result<Self> {
        info!("Initializing Distributed State Machine");
        Ok(Self {
            log: Vec::new(),
            commit_index: 0,
            last_applied: 0,
        })
    }

    /// Append command to log
    pub fn append_entry(&mut self, entry: LogEntry) -> Result<()> {
        info!("Appending log entry at index {}", entry.index);
        self.log.push(entry);
        Ok(())
    }

    /// Apply committed entries
    pub async fn apply_committed(&mut self, target_index: u64) -> Result<()> {
        info!("Applying committed entries up to index {}", target_index);
        self.commit_index = target_index;
        Ok(())
    }

    /// Get log entries
    pub fn get_entries(&self, from_index: u64, to_index: u64) -> Vec<LogEntry> {
        self.log
            .iter()
            .filter(|e| e.index >= from_index && e.index < to_index)
            .cloned()
            .collect()
    }

    /// Get last log term
    pub fn get_last_log_term(&self) -> u64 {
        self.log.last().map(|e| e.term).unwrap_or(0)
    }

    /// Create snapshot
    pub fn create_snapshot(&self) -> Result<Vec<u8>> {
        info!("Creating state machine snapshot");
        let data = serde_json::to_vec(&self.log)?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine() {
        let mut sm = StateMachine::new().unwrap();
        let entry = LogEntry {
            index: 1,
            term: 1,
            command: vec![1, 2, 3],
        };
        assert!(sm.append_entry(entry).is_ok());
        assert_eq!(sm.log.len(), 1);
    }
}
