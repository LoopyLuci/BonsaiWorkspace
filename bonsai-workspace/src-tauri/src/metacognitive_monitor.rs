//! Metacognitive Monitor — tracks reasoning quality, calibrates confidence,
//! detects overconfidence, and recommends strategies.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ── ReasoningStrategy ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReasoningStrategy {
    Deduction,
    Induction,
    Abduction,
    Analogy,
    Counterfactual,
    Hybrid,
}

impl ReasoningStrategy {
    pub fn name(&self) -> &str {
        match self {
            ReasoningStrategy::Deduction => "deduction",
            ReasoningStrategy::Induction => "induction",
            ReasoningStrategy::Abduction => "abduction",
            ReasoningStrategy::Analogy => "analogy",
            ReasoningStrategy::Counterfactual => "counterfactual",
            ReasoningStrategy::Hybrid => "hybrid",
        }
    }
}

// ── Outcome ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Outcome {
    Correct,
    PartiallyCorrect { overlap: f32 },
    Incorrect,
    UserCorrected { corrected_answer: String },
    Contradicted { by_belief_id: String },
    Unknown,
}

impl Outcome {
    pub fn is_correct(&self) -> bool {
        match self {
            Outcome::Correct => true,
            Outcome::PartiallyCorrect { overlap } => *overlap > 0.5,
            _ => false,
        }
    }
}

// ── ReasoningRecord ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningRecord {
    pub id: String,
    pub strategy: ReasoningStrategy,
    pub query: String,
    pub conclusion: String,
    pub predicted_confidence: f32,
    pub actual_outcome: Outcome,
    pub latency_ms: u64,
    pub timestamp: i64,
}

impl ReasoningRecord {
    pub fn is_correct(&self) -> bool { self.actual_outcome.is_correct() }
}

// ── CalibrationCurve ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CalibrationBin {
    predicted_sum: f32,
    correct_count: u32,
    total_count: u32,
}

impl CalibrationBin {
    fn accuracy(&self) -> f32 {
        if self.total_count == 0 { 0.5 } else { self.correct_count as f32 / self.total_count as f32 }
    }
    fn avg_confidence(&self) -> f32 {
        if self.total_count == 0 { 0.5 } else { self.predicted_sum / self.total_count as f32 }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalibrationCurve {
    /// 10 bins: [0.0–0.1), [0.1–0.2), … [0.9–1.0]
    bins: Vec<CalibrationBin>,
}

impl CalibrationCurve {
    pub fn new() -> Self {
        Self { bins: (0..10).map(|_| CalibrationBin::default()).collect() }
    }

    pub fn add_point(&mut self, predicted: f32, correct: bool) {
        let idx = ((predicted * 10.0) as usize).min(9);
        self.bins[idx].predicted_sum += predicted;
        self.bins[idx].total_count += 1;
        if correct { self.bins[idx].correct_count += 1; }
    }

    /// Return calibrated confidence: blend raw with empirical accuracy from that bin.
    pub fn calibrate(&self, raw_confidence: f32) -> f32 {
        let idx = ((raw_confidence * 10.0) as usize).min(9);
        if idx < self.bins.len() && self.bins[idx].total_count >= 5 {
            let empirical = self.bins[idx].accuracy();
            raw_confidence * 0.4 + empirical * 0.6
        } else {
            raw_confidence
        }
    }

    /// Expected Calibration Error (ECE) — lower is better.
    pub fn ece(&self) -> f32 {
        let total: u32 = self.bins.iter().map(|b| b.total_count).sum();
        if total == 0 { return 0.0; }
        self.bins.iter()
            .map(|b| {
                let weight = b.total_count as f32 / total as f32;
                let diff = (b.avg_confidence() - b.accuracy()).abs();
                weight * diff
            })
            .sum()
    }

    pub fn as_points(&self) -> Vec<CalibrationPoint> {
        self.bins.iter().enumerate().map(|(i, b)| CalibrationPoint {
            bucket_center: (i as f32 + 0.5) / 10.0,
            avg_confidence: b.avg_confidence(),
            accuracy: b.accuracy(),
            count: b.total_count,
        }).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationPoint {
    pub bucket_center: f32,
    pub avg_confidence: f32,
    pub accuracy: f32,
    pub count: u32,
}

// ── StrategyPerformance ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyPerformance {
    pub total_attempts: u32,
    pub correct_attempts: u32,
    pub total_confidence: f32,
    pub rolling_accuracy: f32,   // Exponential moving average
    pub average_confidence: f32,
}

impl StrategyPerformance {
    const ALPHA: f32 = 0.1; // EMA smoothing factor

    pub fn update(&mut self, correct: bool, confidence: f32) {
        self.total_attempts += 1;
        if correct { self.correct_attempts += 1; }
        self.total_confidence += confidence;
        let accuracy = if correct { 1.0 } else { 0.0 };
        self.rolling_accuracy = (1.0 - Self::ALPHA) * self.rolling_accuracy + Self::ALPHA * accuracy;
        self.average_confidence = self.total_confidence / self.total_attempts as f32;
    }

    pub fn calibration_error(&self) -> f32 {
        (self.average_confidence - self.rolling_accuracy).abs()
    }
}

// ── MetacognitiveMonitor ──────────────────────────────────────────────────────

pub struct MetacognitiveMonitor {
    pub history: Vec<ReasoningRecord>,
    pub calibration: CalibrationCurve,
    pub strategy_performance: HashMap<ReasoningStrategy, StrategyPerformance>,
}

impl MetacognitiveMonitor {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            calibration: CalibrationCurve::new(),
            strategy_performance: HashMap::new(),
        }
    }

    /// Record the result of a reasoning attempt.
    pub fn record(&mut self, record: ReasoningRecord) {
        let correct = record.is_correct();
        self.calibration.add_point(record.predicted_confidence, correct);
        self.strategy_performance
            .entry(record.strategy.clone())
            .or_default()
            .update(correct, record.predicted_confidence);
        self.history.push(record);
    }

    /// Return calibrated confidence for a given raw value.
    pub fn calibrated_confidence(&self, _strategy: &ReasoningStrategy, raw: f32) -> f32 {
        self.calibration.calibrate(raw)
    }

    /// True if this strategy is predicting > 15% more than it actually achieves.
    pub fn is_overconfident(&self, strategy: &ReasoningStrategy) -> bool {
        self.strategy_performance.get(strategy)
            .map(|p| p.average_confidence - p.rolling_accuracy > 0.15)
            .unwrap_or(false)
    }

    /// Recommend the strategy with highest rolling accuracy (min 5 attempts).
    pub fn recommend_strategy(&self) -> Option<ReasoningStrategy> {
        self.strategy_performance.iter()
            .filter(|(_, p)| p.total_attempts >= 5)
            .max_by(|a, b| a.1.rolling_accuracy.partial_cmp(&b.1.rolling_accuracy)
                .unwrap_or(std::cmp::Ordering::Equal))
            .map(|(s, _)| s.clone())
    }

    /// Generate a full metacognitive report.
    pub fn reflect(&self) -> MetacognitiveReport {
        let strategy_reports = self.strategy_performance.iter().map(|(s, p)| {
            let overconfident = self.is_overconfident(s);
            StrategyReport {
                strategy: s.name().to_string(),
                total_attempts: p.total_attempts,
                rolling_accuracy: p.rolling_accuracy,
                avg_confidence: p.average_confidence,
                calibration_error: p.calibration_error(),
                recommendation: if overconfident {
                    format!("Reduce {} confidence by {:.0}%",
                        s.name(), (p.average_confidence - p.rolling_accuracy) * 100.0)
                } else if p.total_attempts < 5 {
                    format!("{} needs more samples", s.name())
                } else {
                    format!("{} is well-calibrated", s.name())
                },
            }
        }).collect();

        MetacognitiveReport {
            total_reasoning_attempts: self.history.len() as u32,
            overall_accuracy: self.overall_accuracy(),
            calibration_ece: self.calibration.ece(),
            calibration_curve: self.calibration.as_points(),
            strategy_reports,
            recommended_strategy: self.recommend_strategy().map(|s| s.name().to_string()),
        }
    }

    pub fn overall_accuracy(&self) -> f32 {
        let total = self.history.len();
        if total == 0 { return 0.0; }
        let correct = self.history.iter().filter(|r| r.is_correct()).count();
        correct as f32 / total as f32
    }

    pub fn recent_history(&self, n: usize) -> &[ReasoningRecord] {
        let start = self.history.len().saturating_sub(n);
        &self.history[start..]
    }
}

impl Default for MetacognitiveMonitor {
    fn default() -> Self { Self::new() }
}

// ── Report types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyReport {
    pub strategy: String,
    pub total_attempts: u32,
    pub rolling_accuracy: f32,
    pub avg_confidence: f32,
    pub calibration_error: f32,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetacognitiveReport {
    pub total_reasoning_attempts: u32,
    pub overall_accuracy: f32,
    pub calibration_ece: f32,
    pub calibration_curve: Vec<CalibrationPoint>,
    pub strategy_reports: Vec<StrategyReport>,
    pub recommended_strategy: Option<String>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn record(strategy: ReasoningStrategy, conf: f32, correct: bool) -> ReasoningRecord {
        ReasoningRecord {
            id: uuid::Uuid::new_v4().to_string(),
            strategy,
            query: "test".into(),
            conclusion: "answer".into(),
            predicted_confidence: conf,
            actual_outcome: if correct { Outcome::Correct } else { Outcome::Incorrect },
            latency_ms: 50,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    #[test]
    fn overconfidence_detected() {
        let mut m = MetacognitiveMonitor::new();
        for _ in 0..10 {
            m.record(record(ReasoningStrategy::Deduction, 0.9, false));
        }
        assert!(m.is_overconfident(&ReasoningStrategy::Deduction));
    }

    #[test]
    fn recommend_strategy_picks_best() {
        let mut m = MetacognitiveMonitor::new();
        for _ in 0..5 { m.record(record(ReasoningStrategy::Deduction, 0.8, true)); }
        for _ in 0..5 { m.record(record(ReasoningStrategy::Induction, 0.8, false)); }
        let rec = m.recommend_strategy();
        assert_eq!(rec, Some(ReasoningStrategy::Deduction));
    }

    #[test]
    fn calibration_ece_zero_with_no_data() {
        let m = MetacognitiveMonitor::new();
        assert_eq!(m.calibration.ece(), 0.0);
    }

    #[test]
    fn report_generates_without_panic() {
        let mut m = MetacognitiveMonitor::new();
        m.record(record(ReasoningStrategy::Analogy, 0.7, true));
        let report = m.reflect();
        assert_eq!(report.total_reasoning_attempts, 1);
    }
}
