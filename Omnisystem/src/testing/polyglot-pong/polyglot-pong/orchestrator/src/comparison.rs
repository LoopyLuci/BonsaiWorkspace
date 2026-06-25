//! Trace comparison and fidelity computation
//!
//! Compares execution traces from different language implementations
//! to determine conversion fidelity and identify divergences.

use polyglot_pong_common::*;

/// Trace comparison engine
pub struct TraceComparator;

impl TraceComparator {
    /// Create a new trace comparator
    pub fn new() -> Self {
        Self
    }

    /// Compare two execution traces, returning fidelity score (0.0-1.0)
    pub fn compare_traces(
        &self,
        src_trace: &[GameState],
        tgt_trace: &[GameState],
    ) -> f64 {
        if src_trace.is_empty() || tgt_trace.is_empty() {
            return 0.0;
        }

        let min_len = src_trace.len().min(tgt_trace.len());
        let mut matches = 0;

        for i in 0..min_len {
            if self.states_equal(&src_trace[i], &tgt_trace[i]) {
                matches += 1;
            }
        }

        matches as f64 / src_trace.len() as f64
    }

    /// Check if two game states are equal (within tolerance)
    fn states_equal(&self, a: &GameState, b: &GameState) -> bool {
        a.ball_x == b.ball_x
            && a.ball_y == b.ball_y
            && a.ball_dx == b.ball_dx
            && a.ball_dy == b.ball_dy
            && a.paddle_left_y == b.paddle_left_y
            && a.paddle_right_y == b.paddle_right_y
            && a.score_left == b.score_left
            && a.score_right == b.score_right
    }

    /// Find first divergence point between traces
    pub fn find_divergence(
        &self,
        src_trace: &[GameState],
        tgt_trace: &[GameState],
    ) -> Option<(usize, GameState, GameState)> {
        let min_len = src_trace.len().min(tgt_trace.len());

        for i in 0..min_len {
            if !self.states_equal(&src_trace[i], &tgt_trace[i]) {
                return Some((i, src_trace[i], tgt_trace[i]));
            }
        }

        // If lengths differ, that's a divergence
        if src_trace.len() != tgt_trace.len() {
            let i = min_len;
            let src_state = src_trace.last().copied().unwrap_or_default();
            let tgt_state = tgt_trace.last().copied().unwrap_or_default();
            return Some((i, src_state, tgt_state));
        }

        None
    }

    /// Compute fidelity matrix from multiple test results
    pub fn fidelity_matrix(
        &self,
        results: &[TestResult],
        languages: &[Language],
    ) -> Vec<Vec<f64>> {
        let n = languages.len();
        let mut matrix = vec![vec![0.0; n]; n];

        for result in results {
            if let Some(src_idx) = languages.iter().position(|l| l == &result.src_lang) {
                if let Some(tgt_idx) = languages.iter().position(|l| l == &result.tgt_lang) {
                    matrix[src_idx][tgt_idx] = result.fidelity;
                }
            }
        }

        matrix
    }
}

impl Default for TraceComparator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_traces() {
        let comparator = TraceComparator::new();
        let state = GameState::default();
        let trace1 = vec![state, state, state];
        let trace2 = vec![state, state, state];

        let fidelity = comparator.compare_traces(&trace1, &trace2);
        assert_eq!(fidelity, 1.0);
    }

    #[test]
    fn test_empty_traces() {
        let comparator = TraceComparator::new();
        let fidelity = comparator.compare_traces(&[], &[]);
        assert_eq!(fidelity, 0.0);
    }

    #[test]
    fn test_find_divergence() {
        let comparator = TraceComparator::new();
        let mut state1 = GameState::default();
        let mut state2 = GameState::default();
        state1.score_left = 0;
        state2.score_left = 1;

        let trace1 = vec![GameState::default(), state1];
        let trace2 = vec![GameState::default(), state2];

        let divergence = comparator.find_divergence(&trace1, &trace2);
        assert!(divergence.is_some());
        let (idx, _, _) = divergence.unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn test_fidelity_matrix() {
        let comparator = TraceComparator::new();
        let langs = vec!["Rust".into(), "Python".into()];
        let mut result1 = TestResult::default();
        result1.src_lang = "Rust".into();
        result1.tgt_lang = "Python".into();
        result1.fidelity = 0.95;

        let matrix = comparator.fidelity_matrix(&[result1], &langs);
        assert_eq!(matrix.len(), 2);
        assert_eq!(matrix[0][1], 0.95);
    }
}
