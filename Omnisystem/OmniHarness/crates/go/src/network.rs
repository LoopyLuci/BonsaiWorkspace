//! Pure-Rust neural network evaluator for Go MCTS.
//!
//! Architecture: two-headed MLP for 19×19 Go.
//!
//!   Input:  6137 features (17 planes × 361 squares for 19×19)
//!   Hidden: 512 units, ReLU
//!   Policy head: 362 outputs (361 intersections + pass), softmax
//!   Value  head: 1 output, sigmoid → [0,1] win probability for current player
//!
//! Smaller boards (9×9, 13×13) pad the input with zeros to the 19×19 size.
//! Weights stored as raw f32 little-endian binary.

use crate::board::{BoardSize, GoBoard, Point, Stone};
use crate::mcts::{GoEvaluator, RandomGoEvaluator};
use rand::Rng;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

// ── Constants ─────────────────────────────────────────────────────────────────

pub const BOARD_SIZE: usize = 19;
pub const SQUARES: usize = BOARD_SIZE * BOARD_SIZE; // 361
pub const INPUT_SIZE: usize = 17 * SQUARES; // 6137
pub const HIDDEN_SIZE: usize = 512;
pub const POLICY_SIZE: usize = SQUARES + 1; // 362 (pass included)
pub const VALUE_SIZE: usize = 1;

const W1_LEN: usize = INPUT_SIZE * HIDDEN_SIZE;
const B1_LEN: usize = HIDDEN_SIZE;
const WP_LEN: usize = HIDDEN_SIZE * POLICY_SIZE;
const BP_LEN: usize = POLICY_SIZE;
const WV_LEN: usize = HIDDEN_SIZE * VALUE_SIZE;
const BV_LEN: usize = VALUE_SIZE;
pub const TOTAL_PARAMS: usize = W1_LEN + B1_LEN + WP_LEN + BP_LEN + WV_LEN + BV_LEN;

// ── Network weights ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct GoNetWeights {
    pub w1: Vec<f32>,
    pub b1: Vec<f32>,
    pub wp: Vec<f32>,
    pub bp: Vec<f32>,
    pub wv: Vec<f32>,
    pub bv: Vec<f32>,
}

impl GoNetWeights {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let he1 = (2.0_f32 / INPUT_SIZE as f32).sqrt();
        let hep = (2.0_f32 / HIDDEN_SIZE as f32).sqrt();
        let hev = (2.0_f32 / HIDDEN_SIZE as f32).sqrt();

        Self {
            w1: (0..W1_LEN)
                .map(|_| rng.gen::<f32>() * 2.0 * he1 - he1)
                .collect(),
            b1: vec![0.0f32; B1_LEN],
            wp: (0..WP_LEN)
                .map(|_| rng.gen::<f32>() * 2.0 * hep - hep)
                .collect(),
            bp: vec![0.0f32; BP_LEN],
            wv: (0..WV_LEN)
                .map(|_| rng.gen::<f32>() * 2.0 * hev - hev)
                .collect(),
            bv: vec![0.5f32; BV_LEN],
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let mut f = std::fs::File::create(path)?;
        for slice in [&self.w1, &self.b1, &self.wp, &self.bp, &self.wv, &self.bv] {
            for &v in slice {
                f.write_all(&v.to_le_bytes())?;
            }
        }
        Ok(())
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        let mut f = std::fs::File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        let expected = TOTAL_PARAMS * 4;
        if buf.len() != expected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("expected {expected} bytes, got {}", buf.len()),
            ));
        }
        let floats: Vec<f32> = buf
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        let mut off = 0usize;
        macro_rules! take {
            ($n:expr) => {{
                let v = floats[off..off + $n].to_vec();
                off += $n;
                v
            }};
        }
        let result = Ok(Self {
            w1: take!(W1_LEN),
            b1: take!(B1_LEN),
            wp: take!(WP_LEN),
            bp: take!(BP_LEN),
            wv: take!(WV_LEN),
            bv: take!(BV_LEN),
        });
        let _ = off; // off is fully consumed by the take! sequence above
        result
    }

    pub fn hidden(&self, input: &[f32]) -> Vec<f32> {
        let mut h = self.b1.clone();
        for (j, h_j) in h.iter_mut().enumerate() {
            let row = j * INPUT_SIZE;
            let sum: f32 = input
                .iter()
                .enumerate()
                .map(|(i, &x)| x * self.w1[row + i])
                .sum();
            *h_j = (*h_j + sum).max(0.0);
        }
        h
    }

    pub fn policy(&self, hidden: &[f32]) -> Vec<f32> {
        let mut logits = self.bp.clone();
        for (j, l) in logits.iter_mut().enumerate() {
            let row = j * HIDDEN_SIZE;
            *l += hidden
                .iter()
                .enumerate()
                .map(|(i, &h)| h * self.wp[row + i])
                .sum::<f32>();
        }
        softmax(&mut logits);
        logits
    }

    pub fn value(&self, hidden: &[f32]) -> f32 {
        let logit: f32 = self.bv[0]
            + hidden
                .iter()
                .enumerate()
                .map(|(i, &h)| h * self.wv[i])
                .sum::<f32>();
        sigmoid(logit)
    }
}

fn softmax(v: &mut [f32]) {
    let max = v.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0f32;
    for x in v.iter_mut() {
        *x = (*x - max).exp();
        sum += *x;
    }
    if sum > 0.0 {
        for x in v.iter_mut() {
            *x /= sum;
        }
    }
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Pad a board's `to_nn_input()` to the canonical 19×19 input size.
/// Boards smaller than 19×19 are placed in the top-left corner.
pub fn board_to_canonical_input(board: &GoBoard, color: Stone) -> Vec<f32> {
    let raw = board.to_nn_input(color);
    if board.size == BOARD_SIZE as u8 {
        return raw;
    }
    // Pad: 17 planes, each SQUARES long, filled from raw (smaller square)
    let src_sq = board.size as usize * board.size as usize;
    let src_per = 17 * src_sq;
    if raw.len() != src_per {
        return vec![0.0; INPUT_SIZE]; // safety fallback
    }
    let mut out = vec![0.0f32; INPUT_SIZE];
    for plane in 0..17 {
        let src_start = plane * src_sq;
        let dst_start = plane * SQUARES;
        for row in 0..board.size as usize {
            let src_row = src_start + row * board.size as usize;
            let dst_row = dst_start + row * BOARD_SIZE;
            let len = board.size as usize;
            out[dst_row..dst_row + len].copy_from_slice(&raw[src_row..src_row + len]);
        }
    }
    out
}

// ── ADAM optimizer (mirrors chess network implementation) ─────────────────────

pub struct AdamState {
    pub m: Vec<f32>,
    pub v: Vec<f32>,
    pub t: u32,
    pub lr: f32,
    pub beta1: f32,
    pub beta2: f32,
    pub eps: f32,
}

impl AdamState {
    pub fn new(n: usize) -> Self {
        Self {
            m: vec![0.0; n],
            v: vec![0.0; n],
            t: 0,
            lr: 1e-3,
            beta1: 0.9,
            beta2: 0.999,
            eps: 1e-8,
        }
    }

    pub fn step(&mut self, params: &mut [f32], grads: &[f32]) {
        self.t += 1;
        let b1t = self.beta1.powi(self.t as i32);
        let b2t = self.beta2.powi(self.t as i32);
        for i in 0..params.len() {
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * grads[i];
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * grads[i] * grads[i];
            let m_hat = self.m[i] / (1.0 - b1t);
            let v_hat = self.v[i] / (1.0 - b2t);
            params[i] -= self.lr * m_hat / (v_hat.sqrt() + self.eps);
        }
    }
}

// ── Training example ──────────────────────────────────────────────────────────

pub struct GoNetTrainExample {
    pub input: Vec<f32>,
    /// Policy target: probability over [0..361] intersections + [361] = pass.
    pub policy_target: Vec<f32>,
    pub value_target: f32,
}

// ── NetworkGoEvaluator ────────────────────────────────────────────────────────

/// Neural network-backed evaluator for Go MCTS.
/// Falls back to `RandomGoEvaluator` (uniform policy) if no weights loaded.
pub struct NetworkGoEvaluator {
    weights: Option<GoNetWeights>,
    pub weights_path: PathBuf,
}

impl NetworkGoEvaluator {
    pub fn new(weights_path: impl Into<PathBuf>) -> Self {
        let path: PathBuf = weights_path.into();
        let weights = GoNetWeights::load(&path).ok();
        Self {
            weights,
            weights_path: path,
        }
    }

    pub fn default_path() -> PathBuf {
        let base = dirs_or_home();
        base.join(".bonsai").join("models").join("go_net.bin")
    }

    pub fn load_default() -> Self {
        Self::new(Self::default_path())
    }

    pub fn is_loaded(&self) -> bool {
        self.weights.is_some()
    }

    pub fn init_random(&mut self) -> std::io::Result<()> {
        let w = GoNetWeights::random();
        w.save(&self.weights_path)?;
        self.weights = Some(w);
        Ok(())
    }

    pub fn reload(&mut self) -> bool {
        if let Ok(w) = GoNetWeights::load(&self.weights_path) {
            self.weights = Some(w);
            true
        } else {
            false
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        match &self.weights {
            Some(w) => w.save(&self.weights_path),
            None => Err(std::io::Error::other("no weights loaded")),
        }
    }

    pub fn weights(&self) -> Option<&GoNetWeights> {
        self.weights.as_ref()
    }
    pub fn weights_mut(&mut self) -> Option<&mut GoNetWeights> {
        self.weights.as_mut()
    }
}

fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

impl GoEvaluator for NetworkGoEvaluator {
    fn evaluate_policy(&self, board: &GoBoard, color: Stone) -> Vec<(Option<Point>, f32)> {
        let Some(w) = &self.weights else {
            return RandomGoEvaluator.evaluate_policy(board, color);
        };
        let input = board_to_canonical_input(board, color);
        let hidden = w.hidden(&input);
        let policy = w.policy(&hidden);

        // Build candidate list: all empty intersections + pass
        let mut moves: Vec<(Option<Point>, f32)> = Vec::new();
        for y in 0..board.size {
            for x in 0..board.size {
                let pt = Point::new(x, y);
                if !board.stones.contains_key(&pt) {
                    let idx = y as usize * BOARD_SIZE + x as usize;
                    moves.push((Some(pt), policy[idx.min(SQUARES - 1)]));
                }
            }
        }
        // Pass
        moves.push((None, policy[SQUARES]));

        // Re-normalize over legal candidates
        let total: f32 = moves.iter().map(|(_, p)| p).sum();
        if total > 0.0 {
            for (_, p) in moves.iter_mut() {
                *p /= total;
            }
        } else {
            let n = moves.len() as f32;
            for (_, p) in moves.iter_mut() {
                *p = 1.0 / n;
            }
        }
        moves
    }

    fn evaluate_value(&self, board: &GoBoard, color: Stone) -> f32 {
        let Some(w) = &self.weights else {
            return 0.5;
        };
        let input = board_to_canonical_input(board, color);
        let hidden = w.hidden(&input);
        w.value(&hidden)
    }
}

// ── Training helpers ──────────────────────────────────────────────────────────

/// Convert MCTS self-play examples to network training examples.
pub fn mcts_to_train_examples(
    examples: &[crate::mcts::TrainingExample],
    board_size: BoardSize,
) -> Vec<GoNetTrainExample> {
    examples
        .iter()
        .map(|ex| {
            // Reconstruct board from JSON
            let board: GoBoard =
                serde_json::from_str(&ex.board_json).unwrap_or_else(|_| GoBoard::new(board_size));
            // We don't know the color from the example directly — derive from move count
            let color = Stone::Black; // default; GameExample doesn't encode the color
            let input = board_to_canonical_input(&board, color);

            // Build policy target over POLICY_SIZE
            let mut policy_target = vec![1.0 / POLICY_SIZE as f32; POLICY_SIZE];
            for (gtp, prob) in &ex.move_probs {
                let idx = if gtp == "pass" {
                    SQUARES
                } else if let Some(pt) = Point::from_gtp(gtp, board_size) {
                    (pt.y as usize * BOARD_SIZE + pt.x as usize).min(SQUARES - 1)
                } else {
                    continue;
                };
                policy_target[idx] = *prob;
            }
            // Re-normalize
            let sum: f32 = policy_target.iter().sum();
            if sum > 0.0 {
                for p in policy_target.iter_mut() {
                    *p /= sum;
                }
            }

            let value_target = ex.game_result.unwrap_or(0.5);

            GoNetTrainExample {
                input,
                policy_target,
                value_target,
            }
        })
        .collect()
}

/// Train for one epoch on Go network training examples.
/// Returns (policy_loss, value_loss).
pub fn train_epoch(
    weights: &mut GoNetWeights,
    adam: &mut AdamState,
    examples: &[GoNetTrainExample],
    batch_size: usize,
) -> (f32, f32) {
    if examples.is_empty() {
        return (0.0, 0.0);
    }

    let mut total_pl = 0.0f32;
    let mut total_vl = 0.0f32;
    let mut batches = 0u32;

    for batch in examples.chunks(batch_size) {
        let mut g_w1 = vec![0.0f32; W1_LEN];
        let mut g_b1 = vec![0.0f32; B1_LEN];
        let mut g_wp = vec![0.0f32; WP_LEN];
        let mut g_bp = vec![0.0f32; BP_LEN];
        let mut g_wv = vec![0.0f32; WV_LEN];
        let mut g_bv = vec![0.0f32; BV_LEN];
        let n = batch.len() as f32;

        for ex in batch {
            let h = weights.hidden(&ex.input);
            let policy_out = weights.policy(&h);
            let value_out = weights.value(&h);

            // Policy gradient
            let mut d_policy = vec![0.0f32; POLICY_SIZE];
            let mut pl = 0.0f32;
            for i in 0..POLICY_SIZE {
                d_policy[i] = (policy_out[i] - ex.policy_target[i]) / n;
                pl -= ex.policy_target[i] * policy_out[i].max(1e-7).ln();
            }
            total_pl += pl;

            // Value gradient
            let vl = (value_out - ex.value_target).powi(2);
            total_vl += vl;
            let d_value = 2.0 * (value_out - ex.value_target) * value_out * (1.0 - value_out) / n;

            // Backprop value head
            for i in 0..HIDDEN_SIZE {
                g_wv[i] += d_value * h[i];
            }
            g_bv[0] += d_value;

            // Backprop policy head + accumulate d_hidden
            let mut d_hidden = vec![0.0f32; HIDDEN_SIZE];
            for j in 0..POLICY_SIZE {
                if d_policy[j].abs() < 1e-9 {
                    continue;
                }
                let row = j * HIDDEN_SIZE;
                for i in 0..HIDDEN_SIZE {
                    g_wp[row + i] += d_policy[j] * h[i];
                    d_hidden[i] += d_policy[j] * weights.wp[row + i];
                }
                g_bp[j] += d_policy[j];
            }
            for (i, dh) in d_hidden.iter_mut().enumerate() {
                *dh += d_value * weights.wv[i];
            }

            // ReLU gate
            for i in 0..HIDDEN_SIZE {
                if h[i] <= 0.0 {
                    d_hidden[i] = 0.0;
                }
            }

            // Backprop W1
            for j in 0..HIDDEN_SIZE {
                if d_hidden[j] == 0.0 {
                    continue;
                }
                let row = j * INPUT_SIZE;
                for (i, &x) in ex.input.iter().enumerate() {
                    g_w1[row + i] += d_hidden[j] * x / n;
                }
                g_b1[j] += d_hidden[j] / n;
            }
        }

        let mut all_params: Vec<f32> = [
            weights.w1.as_slice(),
            weights.b1.as_slice(),
            weights.wp.as_slice(),
            weights.bp.as_slice(),
            weights.wv.as_slice(),
            weights.bv.as_slice(),
        ]
        .concat();
        let all_grads: Vec<f32> = [
            g_w1.as_slice(),
            g_b1.as_slice(),
            g_wp.as_slice(),
            g_bp.as_slice(),
            g_wv.as_slice(),
            g_bv.as_slice(),
        ]
        .concat();

        adam.step(&mut all_params, &all_grads);

        let mut off = 0;
        macro_rules! copy_back {
            ($dst:expr) => {{
                let n = $dst.len();
                $dst.copy_from_slice(&all_params[off..off + n]);
                off += n;
            }};
        }
        copy_back!(weights.w1);
        copy_back!(weights.b1);
        copy_back!(weights.wp);
        copy_back!(weights.bp);
        copy_back!(weights.wv);
        copy_back!(weights.bv);
        let _ = off; // off is fully consumed by the copy_back! sequence above

        batches += 1;
    }

    (total_pl / batches as f32, total_vl / batches as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::GoBoard;

    #[test]
    fn forward_pass_gives_valid_outputs() {
        let w = GoNetWeights::random();
        let board = GoBoard::new(19);
        let input = board_to_canonical_input(&board, Stone::Black);
        assert_eq!(input.len(), INPUT_SIZE);

        let h = w.hidden(&input);
        assert_eq!(h.len(), HIDDEN_SIZE);
        assert!(
            h.iter().all(|&v| v >= 0.0),
            "ReLU: all hidden should be >= 0"
        );

        let policy = w.policy(&h);
        assert_eq!(policy.len(), POLICY_SIZE);
        let sum: f32 = policy.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-4,
            "policy should sum to 1, got {sum}"
        );

        let value = w.value(&h);
        assert!((0.0..=1.0).contains(&value), "value out of range: {value}");
    }

    #[test]
    fn small_board_padding_correct_size() {
        let board = GoBoard::new(9);
        let input = board_to_canonical_input(&board, Stone::White);
        assert_eq!(input.len(), INPUT_SIZE);
    }

    #[test]
    fn network_evaluator_fallback_without_weights() {
        let eval = NetworkGoEvaluator::new("/nonexistent/go_net.bin");
        assert!(!eval.is_loaded());
        let board = GoBoard::new(9);
        let moves = eval.evaluate_policy(&board, Stone::Black);
        assert!(!moves.is_empty());
        let value = eval.evaluate_value(&board, Stone::Black);
        assert!((0.0..=1.0).contains(&value));
    }

    #[test]
    fn weight_save_load_roundtrip() {
        let path = std::env::temp_dir().join("bonsai_go_net_test.bin");
        let w = GoNetWeights::random();
        w.save(&path).expect("save");
        let w2 = GoNetWeights::load(&path).expect("load");
        assert_eq!(w.w1[0].to_bits(), w2.w1[0].to_bits());
        std::fs::remove_file(&path).ok();
    }
}
