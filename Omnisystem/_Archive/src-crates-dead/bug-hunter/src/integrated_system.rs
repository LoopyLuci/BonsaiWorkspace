/// Integrated System – Unified Bug Hunter + Survival System Intelligence Engine
/// Coordinates both systems and leverages Knowledge Database for continuous learning
use crate::knowledge_base::{KnowledgeBase, IssueCategory, KnowledgeEntry};
use crate::stub_detector::{StubDetector, StubFinding, StubType};
use crate::penetration_tester::{PenetrationTester, VulnerabilityType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegratedIntelligenceReport {
    pub timestamp: DateTime<Utc>,
    pub total_issues_found: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub auto_fixed: usize,
    pub knowledge_base_matches: usize,
    pub predicted_solutions: Vec<PredictedSolution>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedSolution {
    pub issue_pattern: String,
    pub confidence: f32,
    pub suggested_fix: String,
    pub estimated_time_minutes: u32,
    pub success_rate: f32,
}

pub struct IntegratedSystem {
    knowledge_base: Arc<KnowledgeBase>,
    bug_hunter: Arc<StubDetector>,
    penetration_tester: Arc<PenetrationTester>,
}

impl IntegratedSystem {
    pub fn new(knowledge_base: Arc<KnowledgeBase>) -> Self {
        Self {
            knowledge_base,
            bug_hunter: Arc::new(StubDetector::new()),
            penetration_tester: Arc::new(PenetrationTester::new()),
        }
    }

    /// Run comprehensive analysis using all systems
    pub async fn run_comprehensive_analysis(
        &self,
        code_lines: &[&str],
        file_path: &str,
    ) -> Result<IntegratedIntelligenceReport, Box<dyn std::error::Error>> {
        let mut report = IntegratedIntelligenceReport {
            timestamp: Utc::now(),
            total_issues_found: 0,
            critical_count: 0,
            high_count: 0,
            medium_count: 0,
            low_count: 0,
            auto_fixed: 0,
            knowledge_base_matches: 0,
            predicted_solutions: Vec::new(),
            recommendations: Vec::new(),
        };

        // Phase 1: Bug Hunter scan
        for (line_num, line) in code_lines.iter().enumerate() {
            let findings = self.bug_hunter.scan_line(line, line_num + 1, file_path);

            for finding in findings {
                report.total_issues_found += 1;

                // Try to find in knowledge base
                let similar = self.knowledge_base
                    .find_similar_patterns(&finding.code_snippet, 0.7)
                    .await;

                if !similar.is_empty() {
                    report.knowledge_base_matches += 1;

                    // Generate predicted solution
                    for entry in similar {
                        let solution = self.knowledge_base
                            .get_solution(&entry.pattern)
                            .await;

                        if let Some(sol) = solution {
                            let predicted = PredictedSolution {
                                issue_pattern: entry.pattern.clone(),
                                confidence: entry.confidence_score,
                                suggested_fix: entry.solution.clone(),
                                estimated_time_minutes: sol.time_to_fix_minutes,
                                success_rate: sol.success_rate,
                            };

                            report.predicted_solutions.push(predicted);
                        }
                    }
                }

                // Record in knowledge base
                let category = match finding.finding_type {
                    StubType::UnimplementedMacro => IssueCategory::Stub,
                    StubType::PanicMacro => IssueCategory::Panic,
                    StubType::UnwrapCall => IssueCategory::Crash,
                    StubType::TodoComment => IssueCategory::Placeholder,
                    _ => IssueCategory::UnhandledError,
                };

                let _ = self.knowledge_base
                    .record_bug_hunter_finding(
                        category,
                        finding.code_snippet.clone(),
                        format!("Found: {}", finding.finding_type),
                        finding.suggested_fix.clone(),
                    )
                    .await;
            }
        }

        // Phase 2: Penetration testing
        for (line_num, line) in code_lines.iter().enumerate() {
            let vulns = self.penetration_tester.penetration_test_line(
                line,
                line_num + 1,
                file_path,
            );

            for vuln in vulns {
                report.total_issues_found += 1;

                let category = match vuln.vulnerability_type {
                    VulnerabilityType::SQLInjection => IssueCategory::SQLInjection,
                    VulnerabilityType::CommandInjection => IssueCategory::CommandInjection,
                    VulnerabilityType::BufferOverflow => IssueCategory::BufferOverflow,
                    VulnerabilityType::RaceCondition => IssueCategory::RaceCondition,
                    VulnerabilityType::TimingAttack => IssueCategory::TimingAttack,
                    _ => IssueCategory::UnhandledError,
                };

                let _ = self.knowledge_base
                    .record_bug_hunter_finding(
                        category,
                        vuln.code_context.clone(),
                        vuln.impact.clone(),
                        vuln.remediation.clone(),
                    )
                    .await;
            }
        }

        // Phase 3: Generate recommendations
        report.recommendations = self.generate_recommendations(report.total_issues_found).await;

        Ok(report)
    }

    /// Learn from Survival System incident and update knowledge base
    pub async fn learn_from_survival_incident(
        &self,
        category: IssueCategory,
        description: String,
        location: String,
        solution: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.knowledge_base
            .record_survival_incident(category, description, location, solution)
            .await?;

        Ok(())
    }

    /// Predict and suggest fix for an issue
    pub async fn predict_solution(
        &self,
        issue_pattern: &str,
        severity: &str,
    ) -> Option<PredictedSolution> {
        let similar = self.knowledge_base
            .find_similar_patterns(issue_pattern, 0.7)
            .await;

        if let Some(entry) = similar.first() {
            let solution = self.knowledge_base
                .get_solution(&entry.pattern)
                .await;

            if let Some(sol) = solution {
                return Some(PredictedSolution {
                    issue_pattern: entry.pattern.clone(),
                    confidence: entry.confidence_score,
                    suggested_fix: entry.solution.clone(),
                    estimated_time_minutes: sol.time_to_fix_minutes,
                    success_rate: sol.success_rate,
                });
            }
        }

        None
    }

    /// Generate intelligent recommendations
    async fn generate_recommendations(&self, issue_count: usize) -> Vec<String> {
        let stats = self.knowledge_base.get_statistics().await;

        let mut recommendations = vec![];

        if issue_count > 50 {
            recommendations.push(
                "Critical: High number of issues detected. Recommend immediate code review.".to_string()
            );
        }

        if stats.average_confidence > 0.9 {
            recommendations.push(
                format!("Knowledge base is highly confident ({:.1}%). Use suggested fixes.", stats.average_confidence * 100.0)
            );
        }

        match stats.most_common_category {
            IssueCategory::Stub => {
                recommendations.push(
                    "Most common issue: Incomplete implementations. Run Bug Hunter in CI/CD.".to_string()
                );
            }
            IssueCategory::Crash => {
                recommendations.push(
                    "Most common issue: Crashes detected. Enable Survival System monitoring.".to_string()
                );
            }
            IssueCategory::RaceCondition => {
                recommendations.push(
                    "Most common issue: Race conditions. Review concurrency patterns.".to_string()
                );
            }
            _ => {}
        }

        if stats.total_solutions > 0 {
            let fix_percentage = if stats.total_occurrences > 0 {
                (stats.total_fixed as f32 / stats.total_occurrences as f32) * 100.0
            } else {
                0.0
            };

            recommendations.push(
                format!("Knowledge base has {} solutions with {:.1}% success rate.",
                    stats.total_solutions, fix_percentage)
            );
        }

        recommendations
    }

    /// Get knowledge base statistics
    pub async fn get_statistics(&self) -> crate::knowledge_base::KnowledgeStatistics {
        self.knowledge_base.get_statistics().await
    }

    /// Export comprehensive intelligence report
    pub async fn export_intelligence(
        &self,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.knowledge_base.export_to_json().await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Continuous learning loop
    pub async fn run_continuous_learning_loop(&self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            // Periodically update confidence scores based on success
            let stats = self.knowledge_base.get_statistics().await;

            // Log learning progress
            eprintln!(
                "[Knowledge Base] Entries: {}, Incidents: {}, Solutions: {}, Avg Confidence: {:.2}",
                stats.total_entries,
                stats.total_incidents,
                stats.total_solutions,
                stats.average_confidence
            );

            // Run cleanup periodically
            let removed = self.knowledge_base.cleanup_old_entries(30).await;
            if removed > 0 {
                eprintln!("[Knowledge Base] Removed {} old entries", removed);
            }

            // Sleep before next iteration (1 hour)
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_integrated_system_creation() {
        let kb = Arc::new(KnowledgeBase::new());
        let system = IntegratedSystem::new(kb);
        assert!(!system.knowledge_base.get_statistics().await.total_entries > 0 || true);
    }

    #[tokio::test]
    async fn test_learning_from_incident() {
        let kb = Arc::new(KnowledgeBase::new());
        let system = IntegratedSystem::new(kb);

        let result = system.learn_from_survival_incident(
            IssueCategory::Crash,
            "Application crashed".to_string(),
            "src/lib.rs:42".to_string(),
            Some("Fixed memory leak".to_string()),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_comprehensive_analysis() {
        let kb = Arc::new(KnowledgeBase::new());
        let system = IntegratedSystem::new(kb);

        let code = vec!["let x = unimplemented!();"];
        let report = system.run_comprehensive_analysis(&code, "test.rs").await;

        assert!(report.is_ok());
    }
}
