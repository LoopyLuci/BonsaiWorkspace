use crate::board::{GoBoard, Point, Stone};
use crate::mcts::GoEvaluator;
use crate::mcts::RandomGoEvaluator;

/// Neural evaluator scaffold for Go. Replace the internals with a real
/// model-backed evaluator (candle/tch/FI) when ready.
#[derive(Debug, Clone, Default)]
pub struct NeuralGoEvaluator {
    pub model_path: Option<String>,
}

impl NeuralGoEvaluator {
    pub fn with_model(path: impl Into<String>) -> Self {
        Self {
            model_path: Some(path.into()),
        }
    }
}

impl GoEvaluator for NeuralGoEvaluator {
    fn evaluate_policy(&self, board: &GoBoard, color: Stone) -> Vec<(Option<Point>, f32)> {
        // TODO: Run real model inference here. For now, fall back to uniform/random policy.
        RandomGoEvaluator.evaluate_policy(board, color)
    }

    fn evaluate_value(&self, _board: &GoBoard, _color: Stone) -> f32 {
        // TODO: Run model value head. Return neutral 0.5 until integrated.
        0.5
    }
}
