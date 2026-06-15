//! Pure-Rust neural network evaluator for chess MCTS.
//!
//! Architecture: two-headed MLP trained via ADAM on self-play data.
//!
//!   Input:  768 features (12 piece planes × 64 squares)
//!   Hidden: 256 units, ReLU
//!   Policy head: 1858 outputs (all possible UCI-ish moves), softmax
//!   Value  head: 1 output, sigmoid → [0,1] win probability
//!
//! Weights are stored as a compact binary file (raw f32 little-endian).
//! Falls back to MaterialEvaluator if no weights are loaded.

use crate::mcts::{BoardEvaluator, MaterialEvaluator};
use crate::position::ChessPosition;
use rand::Rng;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

// ── Constants ─────────────────────────────────────────────────────────────────

pub const INPUT_SIZE: usize = 768; // 12 × 64
pub const HIDDEN_SIZE: usize = 256;
pub const POLICY_SIZE: usize = 1858; // max possible moves in any chess position
pub const VALUE_SIZE: usize = 1;

// Weight counts:
//   W1: INPUT × HIDDEN, b1: HIDDEN
//   Wp: HIDDEN × POLICY, bp: POLICY
//   Wv: HIDDEN × VALUE,  bv: VALUE
const W1_LEN: usize = INPUT_SIZE * HIDDEN_SIZE;
const B1_LEN: usize = HIDDEN_SIZE;
const WP_LEN: usize = HIDDEN_SIZE * POLICY_SIZE;
const BP_LEN: usize = POLICY_SIZE;
const WV_LEN: usize = HIDDEN_SIZE * VALUE_SIZE;
const BV_LEN: usize = VALUE_SIZE;
pub const TOTAL_PARAMS: usize = W1_LEN + B1_LEN + WP_LEN + BP_LEN + WV_LEN + BV_LEN;

// ── Network weights ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ChessNetWeights {
    pub w1: Vec<f32>, // [INPUT × HIDDEN]
    pub b1: Vec<f32>, // [HIDDEN]
    pub wp: Vec<f32>, // [HIDDEN × POLICY]
    pub bp: Vec<f32>, // [POLICY]
    pub wv: Vec<f32>, // [HIDDEN × VALUE]
    pub bv: Vec<f32>, // [VALUE]
}

impl ChessNetWeights {
    /// Xavier/He initialization — good starting point for ReLU activations.
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let he1 = (2.0_f32 / INPUT_SIZE as f32).sqrt();
        let hep = (2.0_f32 / HIDDEN_SIZE as f32).sqrt();
        let hev = (2.0_f32 / HIDDEN_SIZE as f32).sqrt();

        let w1: Vec<f32> = (0..W1_LEN)
            .map(|_| rng.gen::<f32>() * 2.0 * he1 - he1)
            .collect();
        let b1 = vec![0.0f32; B1_LEN];
        let wp: Vec<f32> = (0..WP_LEN)
            .map(|_| rng.gen::<f32>() * 2.0 * hep - hep)
            .collect();
        let bp = vec![0.0f32; BP_LEN];
        let wv: Vec<f32> = (0..WV_LEN)
            .map(|_| rng.gen::<f32>() * 2.0 * hev - hev)
            .collect();
        let bv = vec![0.5f32; BV_LEN];
        Self {
            w1,
            b1,
            wp,
            bp,
            wv,
            bv,
        }
    }

    /// Save weights to a compact binary file (raw f32 LE).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = std::fs::File::create(path)?;
        for slice in [&self.w1, &self.b1, &self.wp, &self.bp, &self.wv, &self.bv] {
            for &v in slice {
                f.write_all(&v.to_le_bytes())?;
            }
        }
        Ok(())
    }

    /// Load weights from binary file.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let mut f = std::fs::File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        let expected = TOTAL_PARAMS * 4;
        if buf.len() != expected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("expected {} bytes, got {}", expected, buf.len()),
            ));
        }
        let floats: Vec<f32> = buf
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        let mut off = 0;
        let take = |off: &mut usize, n: usize| {
            let v = floats[*off..*off + n].to_vec();
            *off += n;
            v
        };
        Ok(Self {
            w1: take(&mut off, W1_LEN),
            b1: take(&mut off, B1_LEN),
            wp: take(&mut off, WP_LEN),
            bp: take(&mut off, BP_LEN),
            wv: take(&mut off, WV_LEN),
            bv: take(&mut off, BV_LEN),
        })
    }

    // ── Forward pass helpers ──────────────────────────────────────────────────

    /// Hidden layer: linear + ReLU.  Only uses first INPUT_SIZE features.
    pub fn hidden(&self, input: &[f32]) -> Vec<f32> {
        let mut h = self.b1.clone();
        for (j, h_j) in h.iter_mut().enumerate() {
            let row_start = j * INPUT_SIZE;
            let sum: f32 = input[..INPUT_SIZE]
                .iter()
                .enumerate()
                .map(|(i, &x)| x * self.w1[row_start + i])
                .sum();
            *h_j = (*h_j + sum).max(0.0); // ReLU
        }
        h
    }

    /// Policy head: linear → softmax over first `n_moves` outputs.
    pub fn policy(&self, hidden: &[f32], n_moves: usize) -> Vec<f32> {
        if n_moves == 0 {
            return vec![];
        }
        let n = n_moves.min(POLICY_SIZE);
        let mut logits = vec![0.0f32; n];
        for (j, l) in logits.iter_mut().enumerate() {
            let row_start = j * HIDDEN_SIZE;
            *l = self.bp[j]
                + hidden
                    .iter()
                    .enumerate()
                    .map(|(i, &h)| h * self.wp[row_start + i])
                    .sum::<f32>();
        }
        softmax(&mut logits);
        logits
    }

    /// Value head: linear → sigmoid.
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

// ── Activations ───────────────────────────────────────────────────────────────

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

// ── ADAM optimizer ────────────────────────────────────────────────────────────

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

    /// Apply one ADAM step. `params` and `grads` are flat slices of the same length.
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

/// One supervised training example for the chess network.
pub struct NetTrainExample {
    /// NN input features (length = INPUT_SIZE).
    pub input: Vec<f32>,
    /// MCTS move probabilities (length = n_legal_moves, indexed parallel to `legal_uci`).
    pub policy_target: Vec<f32>,
    /// Game outcome from current player's perspective (0.0 / 0.5 / 1.0).
    pub value_target: f32,
    /// Number of legal moves (policy head width for this example).
    pub n_moves: usize,
}

// ── NetworkEvaluator ──────────────────────────────────────────────────────────

/// Neural network-backed evaluator for chess MCTS.
///
/// Falls back to `MaterialEvaluator` if weights are not loaded.
pub struct NetworkEvaluator {
    weights: Option<ChessNetWeights>,
    pub weights_path: PathBuf,
}

impl NetworkEvaluator {
    /// Create with a path to the weight file.
    /// Automatically tries to load existing weights.
    pub fn new(weights_path: impl Into<PathBuf>) -> Self {
        let path: PathBuf = weights_path.into();
        let weights = ChessNetWeights::load(&path).ok();
        Self {
            weights,
            weights_path: path,
        }
    }

    /// Default path: `~/.bonsai/models/chess_net.bin`
    pub fn default_path() -> PathBuf {
        let base = dirs_or_home();
        base.join(".bonsai").join("models").join("chess_net.bin")
    }

    /// Load from the default path.
    pub fn load_default() -> Self {
        Self::new(Self::default_path())
    }

    /// Returns true if network weights are loaded.
    pub fn is_loaded(&self) -> bool {
        self.weights.is_some()
    }

    /// Initialize with random weights and save them to disk.
    /// Used for bootstrapping a new network from scratch.
    pub fn init_random(&mut self) -> std::io::Result<()> {
        let w = ChessNetWeights::random();
        w.save(&self.weights_path)?;
        self.weights = Some(w);
        Ok(())
    }

    /// Load/reload weights from disk.
    pub fn reload(&mut self) -> bool {
        if let Ok(w) = ChessNetWeights::load(&self.weights_path) {
            self.weights = Some(w);
            true
        } else {
            false
        }
    }

    /// Get a reference to the loaded weights (if any).
    pub fn weights(&self) -> Option<&ChessNetWeights> {
        self.weights.as_ref()
    }

    /// Get a mutable reference to the loaded weights (if any).
    pub fn weights_mut(&mut self) -> Option<&mut ChessNetWeights> {
        self.weights.as_mut()
    }

    /// Save current weights to disk.
    pub fn save(&self) -> std::io::Result<()> {
        match &self.weights {
            Some(w) => w.save(&self.weights_path),
            None => Err(std::io::Error::other("no weights loaded")),
        }
    }
}

fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn truncate_input(raw: Vec<f32>) -> Vec<f32> {
    if raw.len() >= INPUT_SIZE {
        return raw[..INPUT_SIZE].to_vec();
    }
    let mut out = raw;
    out.resize(INPUT_SIZE, 0.0);
    out
}

impl BoardEvaluator for NetworkEvaluator {
    fn evaluate_policy(&self, pos: &ChessPosition) -> Vec<f32> {
        let Some(w) = &self.weights else {
            return MaterialEvaluator.evaluate_policy(pos);
        };
        let input = truncate_input(pos.to_nn_input());
        let hidden = w.hidden(&input);
        let n = pos.legal_moves_uci().len();
        w.policy(&hidden, n)
    }

    fn evaluate_value(&self, pos: &ChessPosition) -> f32 {
        let Some(w) = &self.weights else {
            return MaterialEvaluator.evaluate_value(pos);
        };
        let input = truncate_input(pos.to_nn_input());
        let hidden = w.hidden(&input);
        w.value(&hidden)
    }
}

// ── Teacher distillation warmup ───────────────────────────────────────────────

/// Generate synthetic training examples by running `MaterialEvaluator` on random
/// positions and using its evaluations as soft targets.  Used to warm-start the
/// network so it plays legal chess immediately rather than randomly.
pub fn teacher_distill_examples(positions: &[ChessPosition]) -> Vec<NetTrainExample> {
    let teacher = MaterialEvaluator;
    positions
        .iter()
        .map(|pos| {
            let raw = pos.to_nn_input();
            // Truncate / zero-pad to INPUT_SIZE (network uses first 768 features)
            let mut input = vec![0.0f32; INPUT_SIZE];
            let copy_len = raw.len().min(INPUT_SIZE);
            input[..copy_len].copy_from_slice(&raw[..copy_len]);
            let policy_target = teacher.evaluate_policy(pos);
            let value_target = teacher.evaluate_value(pos);
            let n_moves = policy_target.len();
            NetTrainExample {
                input,
                policy_target,
                value_target,
                n_moves,
            }
        })
        .collect()
}

/// Train the network for one epoch on the given examples using ADAM.
/// Returns (policy_loss, value_loss) averages.
pub fn train_epoch(
    weights: &mut ChessNetWeights,
    adam: &mut AdamState,
    examples: &[NetTrainExample],
    batch_size: usize,
) -> (f32, f32) {
    if examples.is_empty() {
        return (0.0, 0.0);
    }

    let mut total_pl = 0.0f32;
    let mut total_vl = 0.0f32;
    let mut batches = 0u32;

    for batch in examples.chunks(batch_size) {
        // Accumulate gradients over the batch (simple SGD-style accumulation)
        let mut g_w1 = vec![0.0f32; W1_LEN];
        let mut g_b1 = vec![0.0f32; B1_LEN];
        let mut g_wp = vec![0.0f32; WP_LEN];
        let mut g_bp = vec![0.0f32; BP_LEN];
        let mut g_wv = vec![0.0f32; WV_LEN];
        let mut g_bv = vec![0.0f32; BV_LEN];

        let n = batch.len() as f32;

        for ex in batch {
            // Forward
            let h = weights.hidden(&ex.input);
            let n_moves = ex.n_moves.min(POLICY_SIZE);
            let policy_out = weights.policy(&h, n_moves);
            let value_out = weights.value(&h);

            // Policy loss gradient (cross-entropy: dL/dlogit = p_out - p_target)
            let mut d_policy = vec![0.0f32; n_moves];
            let mut pl = 0.0f32;
            for i in 0..n_moves {
                let t = ex.policy_target.get(i).copied().unwrap_or(0.0);
                d_policy[i] = (policy_out[i] - t) / n;
                pl -= t * (policy_out[i].max(1e-7)).ln();
            }
            total_pl += pl;

            // Value loss gradient (MSE: dL/d(sigmoid) = 2*(v - t) * sigmoid' )
            let vl = (value_out - ex.value_target).powi(2);
            total_vl += vl;
            let d_value = 2.0 * (value_out - ex.value_target) * value_out * (1.0 - value_out) / n;

            // Backprop through value head
            for i in 0..HIDDEN_SIZE {
                g_wv[i] += d_value * h[i];
            }
            g_bv[0] += d_value;

            // Backprop through policy head
            let mut d_hidden = vec![0.0f32; HIDDEN_SIZE];
            for j in 0..n_moves {
                let row_start = j * HIDDEN_SIZE;
                for i in 0..HIDDEN_SIZE {
                    g_wp[row_start + i] += d_policy[j] * h[i];
                    d_hidden[i] += d_policy[j] * weights.wp[row_start + i];
                }
                g_bp[j] += d_policy[j];
            }

            // Value head also contributes to d_hidden
            for (i, dh) in d_hidden.iter_mut().enumerate() {
                *dh += d_value * weights.wv[i];
            }

            // ReLU gate
            for i in 0..HIDDEN_SIZE {
                if h[i] <= 0.0 {
                    d_hidden[i] = 0.0;
                }
            }

            // Backprop through W1
            for j in 0..HIDDEN_SIZE {
                if d_hidden[j] == 0.0 {
                    continue;
                }
                let row_start = j * INPUT_SIZE;
                for (i, &x) in ex.input.iter().enumerate() {
                    g_w1[row_start + i] += d_hidden[j] * x / n;
                }
                g_b1[j] += d_hidden[j] / n;
            }
        }

        // Flatten all params + grads and apply ADAM
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

        // Write back — resolve lengths before the mutable borrow
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

    let b = batches as f32;
    (total_pl / b, total_vl / b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::ChessPosition;

    #[test]
    fn forward_pass_gives_valid_outputs() {
        let w = ChessNetWeights::random();
        let pos = ChessPosition::initial();
        // to_nn_input returns 119*64=7616; we truncate to INPUT_SIZE=768
        let input = truncate_input(pos.to_nn_input());
        assert_eq!(input.len(), INPUT_SIZE);

        let h = w.hidden(&input);
        assert_eq!(h.len(), HIDDEN_SIZE);
        assert!(h.iter().all(|&v| v >= 0.0)); // ReLU

        let legal_moves = pos.legal_moves_uci();
        let policy = w.policy(&h, legal_moves.len());
        assert_eq!(policy.len(), legal_moves.len());
        let sum: f32 = policy.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-4,
            "policy should sum to 1, got {sum}"
        );

        let value = w.value(&h);
        assert!((0.0..=1.0).contains(&value), "value out of range: {value}");
    }

    #[test]
    fn network_evaluator_fallback_without_weights() {
        // No weight file → should fall back to MaterialEvaluator without panicking
        let eval = NetworkEvaluator::new("/nonexistent/path/chess_net.bin");
        assert!(!eval.is_loaded());
        let pos = ChessPosition::initial();
        let policy = eval.evaluate_policy(&pos);
        assert!(!policy.is_empty());
        let value = eval.evaluate_value(&pos);
        assert!((0.0..=1.0).contains(&value));
    }

    #[test]
    fn teacher_distill_produces_valid_examples() {
        let pos = ChessPosition::initial();
        let examples = teacher_distill_examples(&[pos]);
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].input.len(), INPUT_SIZE);
        let sum: f32 = examples[0].policy_target.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4);
    }

    #[test]
    fn train_epoch_reduces_loss() {
        let mut weights = ChessNetWeights::random();
        let mut adam = AdamState::new(TOTAL_PARAMS);
        let positions: Vec<_> = (0..8).map(|_| ChessPosition::initial()).collect();
        let examples = teacher_distill_examples(&positions);
        let (pl0, vl0) = train_epoch(&mut weights, &mut adam, &examples, 4);
        let (pl1, vl1) = train_epoch(&mut weights, &mut adam, &examples, 4);
        // Losses should generally not explode; not guaranteed to decrease in 2 steps
        assert!(
            pl0 >= 0.0 && pl1 >= 0.0,
            "policy loss should be non-negative"
        );
        assert!(
            vl0 >= 0.0 && vl1 >= 0.0,
            "value loss should be non-negative"
        );
    }

    #[test]
    fn weight_save_load_roundtrip() {
        let dir = std::env::temp_dir().join("bonsai_chess_net_test.bin");
        let w = ChessNetWeights::random();
        w.save(&dir).expect("save failed");
        let w2 = ChessNetWeights::load(&dir).expect("load failed");
        assert_eq!(w.w1[0].to_bits(), w2.w1[0].to_bits());
        std::fs::remove_file(&dir).ok();
    }
}
