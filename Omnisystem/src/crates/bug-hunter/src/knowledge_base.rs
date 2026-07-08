/// Knowledge Base – Centralized repository of all learned bugs, errors, failures, and solutions
/// Continuously updated by Bug Hunter and Survival System
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: String,
    pub category: IssueCategory,
    pub pattern: String,
    pub description: String,
    pub impact: String,
    pub solution: String,
    pub confidence_score: f32,  // 0.0 - 1.0
    pub occurrences: u32,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub fixed_count: u32,
    pub prevention: String,
    pub related_patterns: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IssueCategory {
    // Bug Hunter categories
    Stub,
    Placeholder,
    UnhandledError,

    // Security categories
    SQLInjection,
    CommandInjection,
    BufferOverflow,
    MemoryLeak,
    RaceCondition,
    TimingAttack,

    // Reliability categories
    Crash,
    Panic,
    Timeout,
    ResourceExhaustion,

    // Performance categories
    SlowOperation,
    HighMemory,
    DeadlockRisk,

    // Logic categories
    OffByOne,
    NullDereference,
    InvalidState,
    EdgeCase,
}

impl std::fmt::Display for IssueCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueCategory::Stub => write!(f, "Stub/Placeholder"),
            IssueCategory::Placeholder => write!(f, "Placeholder Code"),
            IssueCategory::UnhandledError => write!(f, "Unhandled Error"),
            IssueCategory::SQLInjection => write!(f, "SQL Injection"),
            IssueCategory::CommandInjection => write!(f, "Command Injection"),
            IssueCategory::BufferOverflow => write!(f, "Buffer Overflow"),
            IssueCategory::MemoryLeak => write!(f, "Memory Leak"),
            IssueCategory::RaceCondition => write!(f, "Race Condition"),
            IssueCategory::TimingAttack => write!(f, "Timing Attack"),
            IssueCategory::Crash => write!(f, "Crash"),
            IssueCategory::Panic => write!(f, "Panic"),
            IssueCategory::Timeout => write!(f, "Timeout"),
            IssueCategory::ResourceExhaustion => write!(f, "Resource Exhaustion"),
            IssueCategory::SlowOperation => write!(f, "Slow Operation"),
            IssueCategory::HighMemory => write!(f, "High Memory Usage"),
            IssueCategory::DeadlockRisk => write!(f, "Deadlock Risk"),
            IssueCategory::OffByOne => write!(f, "Off-By-One Error"),
            IssueCategory::NullDereference => write!(f, "Null Dereference"),
            IssueCategory::InvalidState => write!(f, "Invalid State"),
            IssueCategory::EdgeCase => write!(f, "Edge Case"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeIncident {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub source: IncidentSource,
    pub issue_category: IssueCategory,
    pub description: String,
    pub location: String,  // File:line
    pub impact: String,
    pub resolution: Option<String>,
    pub resolution_time_ms: Option<u64>,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncidentSource {
    BugHunter,
    SurvivalSystem,
    ManualReport,
    AutomaticDetection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionPattern {
    pub issue_pattern: String,
    pub solution_steps: Vec<String>,
    pub code_example: String,
    pub success_rate: f32,
    pub time_to_fix_minutes: u32,
}

pub struct KnowledgeBase {
    // Core storage
    entries: Arc<RwLock<HashMap<String, KnowledgeEntry>>>,
    incidents: Arc<RwLock<Vec<KnowledgeIncident>>>,
    solutions: Arc<RwLock<HashMap<String, SolutionPattern>>>,

    // Indexes for fast lookup
    category_index: Arc<RwLock<HashMap<IssueCategory, Vec<String>>>>,
    pattern_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl KnowledgeBase {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            incidents: Arc::new(RwLock::new(Vec::new())),
            solutions: Arc::new(RwLock::new(HashMap::new())),
            category_index: Arc::new(RwLock::new(HashMap::new())),
            pattern_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a finding from Bug Hunter
    pub async fn record_bug_hunter_finding(
        &self,
        category: IssueCategory,
        pattern: String,
        description: String,
        solution: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let entry_id = format!("bh-{}-{}", category as u32, uuid::Uuid::new_v4());

        let mut entries = self.entries.write().await;
        let mut category_idx = self.category_index.write().await;

        let entry = KnowledgeEntry {
            id: entry_id.clone(),
            category,
            pattern: pattern.clone(),
            description,
            impact: "Code quality issue".to_string(),
            solution,
            confidence_score: 0.95,  // Bug Hunter has high confidence
            occurrences: 1,
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            fixed_count: 0,
            prevention: "Run Bug Hunter in CI/CD".to_string(),
            related_patterns: Vec::new(),
            metadata: HashMap::new(),
        };

        entries.insert(entry_id.clone(), entry);

        category_idx.entry(category)
            .or_insert_with(Vec::new)
            .push(entry_id.clone());

        Ok(entry_id)
    }

    /// Record an incident from Survival System
    pub async fn record_survival_incident(
        &self,
        issue_category: IssueCategory,
        description: String,
        location: String,
        resolution: Option<String>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let incident_id = format!("ss-{}-{}", issue_category as u32, uuid::Uuid::new_v4());

        let mut incidents = self.incidents.write().await;

        let incident = KnowledgeIncident {
            id: incident_id.clone(),
            timestamp: Utc::now(),
            source: IncidentSource::SurvivalSystem,
            issue_category,
            description,
            location,
            impact: "Runtime issue detected".to_string(),
            resolution,
            resolution_time_ms: None,
            success: true,
        };

        incidents.push(incident);

        // Also update or create entry in knowledge base
        self.update_knowledge_entry(issue_category, incident_id.clone()).await?;

        Ok(incident_id)
    }

    /// Update knowledge entry confidence and statistics
    async fn update_knowledge_entry(
        &self,
        category: IssueCategory,
        entry_id: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut entries = self.entries.write().await;

        // Find matching entry by category
        let entry = entries.values_mut()
            .find(|e| e.category == category)
            .map(|e| e.clone());

        if let Some(mut entry) = entry {
            entry.occurrences += 1;
            entry.last_seen = Utc::now();
            // Increase confidence with more occurrences (up to 0.99)
            entry.confidence_score = (entry.confidence_score + 0.01).min(0.99);
            entries.insert(entry.id.clone(), entry);
        }

        Ok(())
    }

    /// Store a solution for a problem
    pub async fn store_solution(
        &self,
        issue_pattern: String,
        solution_steps: Vec<String>,
        code_example: String,
        time_to_fix_minutes: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut solutions = self.solutions.write().await;

        let solution = SolutionPattern {
            issue_pattern: issue_pattern.clone(),
            solution_steps,
            code_example,
            success_rate: 1.0,
            time_to_fix_minutes,
        };

        solutions.insert(issue_pattern.clone(), solution);

        // Update pattern index
        let mut pattern_idx = self.pattern_index.write().await;
        pattern_idx.entry(issue_pattern)
            .or_insert_with(Vec::new)
            .push("solution-added".to_string());

        Ok(())
    }

    /// Query knowledge base for similar patterns
    pub async fn find_similar_patterns(
        &self,
        pattern: &str,
        min_confidence: f32,
    ) -> Vec<KnowledgeEntry> {
        let entries = self.entries.read().await;
        entries.values()
            .filter(|e| {
                e.pattern.contains(pattern) && e.confidence_score >= min_confidence
            })
            .cloned()
            .collect()
    }

    /// Get all entries by category
    pub async fn get_by_category(
        &self,
        category: IssueCategory,
    ) -> Vec<KnowledgeEntry> {
        let entries = self.entries.read().await;
        entries.values()
            .filter(|e| e.category == category)
            .cloned()
            .collect()
    }

    /// Get solution for a problem
    pub async fn get_solution(&self, pattern: &str) -> Option<SolutionPattern> {
        let solutions = self.solutions.read().await;
        solutions.get(pattern).cloned()
    }

    /// Generate statistics on knowledge base
    pub async fn get_statistics(&self) -> KnowledgeStatistics {
        let entries = self.entries.read().await;
        let incidents = self.incidents.read().await;
        let solutions = self.solutions.read().await;

        let total_occurrences: u32 = entries.values().map(|e| e.occurrences).sum();
        let total_fixed: u32 = entries.values().map(|e| e.fixed_count).sum();
        let avg_confidence: f32 = if !entries.is_empty() {
            entries.values().map(|e| e.confidence_score).sum::<f32>() / entries.len() as f32
        } else {
            0.0
        };

        KnowledgeStatistics {
            total_entries: entries.len(),
            total_incidents: incidents.len(),
            total_solutions: solutions.len(),
            total_occurrences,
            total_fixed,
            average_confidence: avg_confidence,
            most_common_category: self.get_most_common_category(&entries),
        }
    }

    fn get_most_common_category(
        &self,
        entries: &HashMap<String, KnowledgeEntry>,
    ) -> IssueCategory {
        let mut category_counts: HashMap<IssueCategory, u32> = HashMap::new();
        for entry in entries.values() {
            *category_counts.entry(entry.category).or_insert(0) += entry.occurrences;
        }

        category_counts.iter()
            .max_by_key(|&(_, count)| count)
            .map(|(&cat, _)| cat)
            .unwrap_or(IssueCategory::Stub)
    }

    /// Export knowledge base to JSON
    pub async fn export_to_json(&self) -> Result<String, serde_json::Error> {
        let entries = self.entries.read().await;
        let incidents = self.incidents.read().await;
        let solutions = self.solutions.read().await;

        #[derive(Serialize)]
        struct Export {
            entries: Vec<KnowledgeEntry>,
            incidents: Vec<KnowledgeIncident>,
            solutions: Vec<SolutionPattern>,
            timestamp: DateTime<Utc>,
        }

        let export = Export {
            entries: entries.values().cloned().collect(),
            incidents: incidents.clone(),
            solutions: solutions.values().cloned().collect(),
            timestamp: Utc::now(),
        };

        serde_json::to_string_pretty(&export)
    }

    /// Clear old entries (retention policy)
    pub async fn cleanup_old_entries(&self, days_old: u64) -> u32 {
        let mut entries = self.entries.write().await;
        let cutoff = Utc::now() - chrono::Duration::days(days_old as i64);

        let initial_count = entries.len();
        entries.retain(|_, e| e.last_seen > cutoff);

        (initial_count - entries.len()) as u32
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KnowledgeStatistics {
    pub total_entries: usize,
    pub total_incidents: usize,
    pub total_solutions: usize,
    pub total_occurrences: u32,
    pub total_fixed: u32,
    pub average_confidence: f32,
    pub most_common_category: IssueCategory,
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_bug_hunter_finding() {
        let kb = KnowledgeBase::new();
        let result = kb.record_bug_hunter_finding(
            IssueCategory::Stub,
            "unimplemented!()".to_string(),
            "Found stub".to_string(),
            "Implement the function".to_string(),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_survival_incident() {
        let kb = KnowledgeBase::new();
        let result = kb.record_survival_incident(
            IssueCategory::Crash,
            "Application crashed".to_string(),
            "src/lib.rs:42".to_string(),
            Some("Restarted".to_string()),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_statistics() {
        let kb = KnowledgeBase::new();
        kb.record_bug_hunter_finding(
            IssueCategory::Stub,
            "unimplemented!()".to_string(),
            "Found stub".to_string(),
            "Implement".to_string(),
        ).await.unwrap();

        let stats = kb.get_statistics().await;
        assert_eq!(stats.total_entries, 1);
    }
}
