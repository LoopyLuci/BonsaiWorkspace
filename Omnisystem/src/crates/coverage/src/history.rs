use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Duration, Utc};
use std::collections::VecDeque;

/// Historical coverage point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageHistoryPoint {
    pub timestamp: DateTime<Utc>,
    pub crate_name: String,
    pub coverage_percent: f64,
    pub lines_covered: usize,
    pub lines_total: usize,
}

/// Tracks coverage history and trends
pub struct CoverageHistory {
    points: Arc<RwLock<VecDeque<CoverageHistoryPoint>>>,
    max_points: usize,
}

impl CoverageHistory {
    pub fn new(max_points: usize) -> Self {
        Self {
            points: Arc::new(RwLock::new(VecDeque::with_capacity(max_points))),
            max_points,
        }
    }

    /// Record a coverage point
    pub fn record(
        &self,
        crate_name: &str,
        coverage_percent: f64,
        lines_covered: usize,
        lines_total: usize,
    ) {
        let point = CoverageHistoryPoint {
            timestamp: Utc::now(),
            crate_name: crate_name.to_string(),
            coverage_percent,
            lines_covered,
            lines_total,
        };

        let mut points = self.points.write();
        points.push_back(point);

        // Maintain max capacity
        while points.len() > self.max_points {
            points.pop_front();
        }
    }

    /// Get trend for a crate (last N days)
    pub fn get_crate_trend(&self, crate_name: &str, days: i64) -> Vec<CoverageHistoryPoint> {
        let points = self.points.read();
        let cutoff = Utc::now() - Duration::days(days);

        points
            .iter()
            .filter(|p| p.crate_name == crate_name && p.timestamp > cutoff)
            .cloned()
            .collect()
    }

    /// Calculate trend direction (improving, stable, declining)
    pub fn get_trend_direction(&self, crate_name: &str, days: i64) -> TrendDirection {
        let trend = self.get_crate_trend(crate_name, days);

        if trend.len() < 2 {
            return TrendDirection::Insufficient;
        }

        let first = trend.first().unwrap().coverage_percent;
        let last = trend.last().unwrap().coverage_percent;
        let change = last - first;

        match change {
            c if c > 2.0 => TrendDirection::Improving,
            c if c < -2.0 => TrendDirection::Declining,
            _ => TrendDirection::Stable,
        }
    }

    /// Get all history points
    pub fn get_all_points(&self) -> Vec<CoverageHistoryPoint> {
        self.points.read().iter().cloned().collect()
    }

    /// Get latest coverage for each crate
    pub fn get_latest_coverage(&self) -> std::collections::HashMap<String, f64> {
        let points = self.points.read();
        let mut latest = std::collections::HashMap::new();

        for point in points.iter().rev() {
            latest.entry(point.crate_name.clone())
                .or_insert(point.coverage_percent);
        }

        latest
    }

    /// Clear history
    pub fn clear(&self) {
        self.points.write().clear();
    }
}

impl Default for CoverageHistory {
    fn default() -> Self {
        Self::new(10000) // Store up to 10000 points (e.g., ~11 days at 15-min intervals)
    }
}

/// Trend direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrendDirection {
    Improving,
    Stable,
    Declining,
    Insufficient,
}

impl std::fmt::Display for TrendDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrendDirection::Improving => write!(f, "📈 Improving"),
            TrendDirection::Stable => write!(f, "→ Stable"),
            TrendDirection::Declining => write!(f, "📉 Declining"),
            TrendDirection::Insufficient => write!(f, "? Insufficient data"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_history() {
        let history = CoverageHistory::new(100);
        history.record("test_crate", 80.0, 80, 100);
        history.record("test_crate", 82.0, 82, 100);

        let trend = history.get_crate_trend("test_crate", 1);
        assert_eq!(trend.len(), 2);
    }

    #[test]
    fn test_trend_direction() {
        let history = CoverageHistory::new(100);
        history.record("crate1", 75.0, 75, 100);
        history.record("crate1", 80.0, 80, 100);

        let direction = history.get_trend_direction("crate1", 1);
        assert_eq!(direction, TrendDirection::Improving);
    }
}
