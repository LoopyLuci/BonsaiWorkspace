use crate::model::GoNet;
use anyhow::Result;
use go::board::{GoBoard, Point, Stone};
use go::mcts::GoEvaluator as BonsaiGoEvaluator;
use candle_core::{Device, Tensor};
use std::sync::Arc;

/// Neural evaluator bridge — currently a scaffold that falls back to uniform policy.
pub struct NeuralEvaluator {
    pub model: Option<Arc<GoNet>>,
    pub device: Device,
}

impl NeuralEvaluator {
    pub fn new(_model_path: &str, device: Device) -> Result<Self> {
        // In a full implementation, load model vars here. For now, keep model None.
        Ok(Self {
            model: None,
            device,
        })
    }

    /// Construct an evaluator from an existing model instance.
    pub fn new_from_model(model: crate::model::GoNet, device: Device) -> Result<Self> {
        Ok(Self {
            model: Some(Arc::new(model)),
            device,
        })
    }

    fn board_to_tensor(&self, board: &GoBoard) -> Result<Tensor> {
        // Build (1, 17, 19*19) tensor from the board's flat NN input (Black to move).
        let flat = board.to_nn_input(go::board::Stone::Black);
        let t = Tensor::from_vec(flat, (1, 17, 19 * 19), &self.device)?;
        Ok(t)
    }
}

impl BonsaiGoEvaluator for NeuralEvaluator {
    fn evaluate_policy(&self, board: &GoBoard, _color: Stone) -> Vec<(Option<Point>, f32)> {
        // If model is available, run inference. Fallback: use uniform policy.
        if let Some(model) = &self.model {
            // Produce the candle tensor for potential GPU-based pre-processing / logging.
            // For the Vec<f32>-based GoNet path we continue to use to_nn_input directly;
            // board_to_tensor records the tensor in debug builds for shape validation.
            #[cfg(debug_assertions)]
            if let Ok(_t) = self.board_to_tensor(board) {
                // Tensor shape validated: (1, 17, 19*19). No further action in this path.
                let _ = _t;
            }
            // Convert board -> flat input
            let input = board.to_nn_input(_color);
            let (logits, _value, _act) = model.forward_single(&input);
            // softmax
            let maxl = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let exps: Vec<f32> = logits.iter().map(|v| (v - maxl).exp()).collect();
            let sum: f32 = exps.iter().sum();
            let probs: Vec<f32> = if sum > 0.0 {
                exps.into_iter().map(|v| v / sum).collect()
            } else {
                vec![1.0 / logits.len() as f32; logits.len()]
            };

            let mut out: Vec<(Option<Point>, f32)> = Vec::new();
            let mut total = 0.0f32;
            for pt in board.empty_points() {
                let idx = pt.y as usize * board.size as usize + pt.x as usize;
                let p = probs.get(idx).cloned().unwrap_or(0.0);
                total += p;
                out.push((Some(pt), p));
            }
            // normalize
            if total > 0.0 {
                for (_p, prob) in out.iter_mut() {
                    *prob /= total;
                }
            } else {
                let n = out.len() as f32;
                for (_p, prob) in out.iter_mut() {
                    *prob = 1.0 / n;
                }
            }
            // append pass with tiny prob 0.0
            out.push((None, 0.0));
            return out;
        }

        let pts: Vec<Option<Point>> = board.empty_points().into_iter().map(Some).collect();
        let mut out: Vec<(Option<Point>, f32)> = pts.into_iter().map(|p| (p, 1.0)).collect();
        out.push((None, 1.0));
        let n = out.len() as f32;
        out.into_iter().map(|(p, _)| (p, 1.0 / n)).collect()
    }

    fn evaluate_value(&self, _board: &GoBoard, _color: Stone) -> f32 {
        if let Some(model) = &self.model {
            let input = _board.to_nn_input(_color);
            let (_logits, value, _act) = model.forward_single(&input);
            // Sigmoid-like squash to 0..1 via tanh/2 + 0.5
            let v = value.tanh() * 0.5 + 0.5;
            return v;
        }
        0.5
    }
}
