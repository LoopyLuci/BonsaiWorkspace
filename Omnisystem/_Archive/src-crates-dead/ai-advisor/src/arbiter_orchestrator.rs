/// Conflict resolution and decision arbitration between advisors
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisorRecommendation {
    pub advisor_id: String,
    pub recommendation: String,
    pub confidence: f32,
    pub weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    None,
    Minor,
    Moderate,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictAnalysis {
    pub conflict_type: ConflictType,
    pub disagreement_level: f32,
    pub advisors_involved: Vec<String>,
    pub analysis_timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrationDecision {
    pub decision_id: String,
    pub selected_recommendation: String,
    pub reason: String,
    pub final_confidence: f32,
    pub conflict_analysis: ConflictAnalysis,
    pub user_preference_applied: bool,
    pub timestamp: i64,
}

pub struct Arbiter {
    recommendations: Arc<RwLock<HashMap<String, Vec<AdvisorRecommendation>>>>,
    decisions: Arc<RwLock<Vec<ArbitrationDecision>>>,
    user_preferences: Arc<RwLock<HashMap<String, String>>>,
    conflict_history: Arc<RwLock<Vec<ConflictAnalysis>>>,
}

impl Arbiter {
    pub fn new() -> Self {
        Self {
            recommendations: Arc::new(RwLock::new(HashMap::new())),
            decisions: Arc::new(RwLock::new(Vec::new())),
            user_preferences: Arc::new(RwLock::new(HashMap::new())),
            conflict_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn submit_recommendation(
        &self,
        context_id: String,
        recommendation: AdvisorRecommendation,
    ) -> Result<()> {
        let mut recs = self.recommendations.write().await;
        recs.entry(context_id)
            .or_insert_with(Vec::new)
            .push(recommendation);

        tracing::info!("Submitted recommendation for context");
        Ok(())
    }

    pub async fn detect_conflicts(&self, context_id: &str) -> Result<ConflictAnalysis> {
        let recs = self.recommendations.read().await;
        let recommendations = recs.get(context_id).cloned().unwrap_or_default();

        let conflict_type = if recommendations.len() <= 1 {
            ConflictType::None
        } else {
            let confidence_variance = self.calculate_variance(&recommendations);
            match confidence_variance {
                v if v < 0.05 => ConflictType::None,
                v if v < 0.15 => ConflictType::Minor,
                v if v < 0.30 => ConflictType::Moderate,
                _ => ConflictType::Critical,
            }
        };

        let advisors: Vec<String> = recommendations.iter().map(|r| r.advisor_id.clone()).collect();
        let disagreement_level = self.calculate_variance(&recommendations);

        let analysis = ConflictAnalysis {
            conflict_type,
            disagreement_level,
            advisors_involved: advisors,
            analysis_timestamp: chrono::Utc::now().timestamp(),
        };

        let mut history = self.conflict_history.write().await;
        history.push(analysis.clone());

        tracing::info!("Detected conflict type: {:?}", conflict_type);
        Ok(analysis)
    }

    pub async fn arbitrate(&self, context_id: &str) -> Result<ArbitrationDecision> {
        let recs = self.recommendations.read().await;
        let recommendations = recs.get(context_id).cloned().unwrap_or_default();

        let conflict = self.detect_conflicts(context_id).await?;
        drop(recs);

        let (selected, reason, confidence) = if recommendations.is_empty() {
            ("No recommendation available".to_string(), "No advisors provided input".to_string(), 0.0)
        } else if recommendations.len() == 1 {
            let rec = &recommendations[0];
            (rec.recommendation.clone(), "Single advisor recommendation".to_string(), rec.confidence)
        } else {
            let weighted = self.calculate_weighted_consensus(&recommendations);
            (weighted.0, weighted.1, weighted.2)
        };

        let user_pref = self.user_preferences.read().await;
        let user_preference_applied = user_pref.contains_key(context_id);
        drop(user_pref);

        let decision = ArbitrationDecision {
            decision_id: uuid::Uuid::new_v4().to_string(),
            selected_recommendation: selected,
            reason,
            final_confidence: confidence,
            conflict_analysis: conflict,
            user_preference_applied,
            timestamp: chrono::Utc::now().timestamp(),
        };

        let mut decisions = self.decisions.write().await;
        decisions.push(decision.clone());

        tracing::info!("Arbitration decision made with confidence: {}", confidence);
        Ok(decision)
    }

    pub async fn set_user_preference(&self, context_id: String, preference: String) -> Result<()> {
        let mut prefs = self.user_preferences.write().await;
        prefs.insert(context_id, preference);

        tracing::info!("User preference recorded");
        Ok(())
    }

    pub async fn get_user_preference(&self, context_id: &str) -> Result<Option<String>> {
        let prefs = self.user_preferences.read().await;
        Ok(prefs.get(context_id).cloned())
    }

    pub async fn get_decision_history(&self) -> Result<Vec<ArbitrationDecision>> {
        let decisions = self.decisions.read().await;
        Ok(decisions.clone())
    }

    pub async fn get_conflict_history(&self) -> Result<Vec<ConflictAnalysis>> {
        let history = self.conflict_history.read().await;
        Ok(history.clone())
    }

    pub async fn get_feedback(&self, decision_id: &str) -> Result<Option<String>> {
        let decisions = self.decisions.read().await;
        let decision = decisions.iter().find(|d| d.decision_id == decision_id);
        Ok(decision.map(|d| format!("Decision: {} with confidence: {}", d.selected_recommendation, d.final_confidence)))
    }

    pub async fn record_feedback(&self, decision_id: String, feedback: String) -> Result<()> {
        let mut decisions = self.decisions.write().await;
        if let Some(decision) = decisions.iter_mut().find(|d| d.decision_id == decision_id) {
            tracing::info!("Recorded feedback for decision: {}", feedback);
        }
        Ok(())
    }

    pub async fn get_stats(&self) -> Result<HashMap<String, String>> {
        let decisions = self.decisions.read().await;
        let conflicts = self.conflict_history.read().await;

        let mut stats = HashMap::new();
        stats.insert("total_decisions".to_string(), decisions.len().to_string());
        stats.insert("total_conflicts".to_string(), conflicts.len().to_string());

        let critical_count = conflicts.iter().filter(|c| c.conflict_type == ConflictType::Critical).count();
        stats.insert("critical_conflicts".to_string(), critical_count.to_string());

        let avg_confidence = if decisions.is_empty() {
            0.0
        } else {
            decisions.iter().map(|d| d.final_confidence).sum::<f32>() / decisions.len() as f32
        };
        stats.insert("avg_confidence".to_string(), format!("{:.2}", avg_confidence));

        Ok(stats)
    }

    fn calculate_variance(&self, recommendations: &[AdvisorRecommendation]) -> f32 {
        if recommendations.len() <= 1 {
            return 0.0;
        }

        let mean: f32 = recommendations.iter().map(|r| r.confidence).sum::<f32>() / recommendations.len() as f32;
        let variance: f32 = recommendations
            .iter()
            .map(|r| (r.confidence - mean).powi(2))
            .sum::<f32>() / recommendations.len() as f32;

        variance.sqrt()
    }

    fn calculate_weighted_consensus(&self, recommendations: &[AdvisorRecommendation]) -> (String, String, f32) {
        if recommendations.is_empty() {
            return ("No recommendation".to_string(), "Empty recommendations".to_string(), 0.0);
        }

        let best = recommendations.iter()
            .max_by(|a, b| {
                let a_score = a.confidence * a.weight;
                let b_score = b.confidence * b.weight;
                a_score.partial_cmp(&b_score).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        (
            best.recommendation.clone(),
            "Weighted consensus from multiple advisors".to_string(),
            best.confidence * best.weight,
        )
    }
}

impl Default for Arbiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_arbiter_creation() {
        let arbiter = Arbiter::new();
        let stats = arbiter.get_stats().await.unwrap();
        assert_eq!(stats.get("total_decisions").map(|s| s.as_str()), Some("0"));
    }

    #[tokio::test]
    async fn test_submit_recommendation() {
        let arbiter = Arbiter::new();
        let rec = AdvisorRecommendation {
            advisor_id: "advisor-1".to_string(),
            recommendation: "Option A".to_string(),
            confidence: 0.9,
            weight: 1.0,
        };

        let result = arbiter.submit_recommendation("ctx-1".to_string(), rec).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_detect_conflicts() {
        let arbiter = Arbiter::new();
        let rec1 = AdvisorRecommendation {
            advisor_id: "advisor-1".to_string(),
            recommendation: "Option A".to_string(),
            confidence: 0.9,
            weight: 1.0,
        };

        arbiter.submit_recommendation("ctx-1".to_string(), rec1).await.unwrap();

        let conflict = arbiter.detect_conflicts("ctx-1").await.unwrap();
        assert_eq!(conflict.conflict_type, ConflictType::None);
    }

    #[tokio::test]
    async fn test_arbitrate() {
        let arbiter = Arbiter::new();
        let rec = AdvisorRecommendation {
            advisor_id: "advisor-1".to_string(),
            recommendation: "Option A".to_string(),
            confidence: 0.85,
            weight: 1.0,
        };

        arbiter.submit_recommendation("ctx-1".to_string(), rec).await.unwrap();
        let decision = arbiter.arbitrate("ctx-1").await.unwrap();

        assert!(!decision.selected_recommendation.is_empty());
        assert!(decision.final_confidence > 0.0);
    }
}
