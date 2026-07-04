use crate::evaluator::NeuralEvaluator;
use crate::model::GoNet;
use anyhow::Result;
use cas::CasStore;
use go::board::{GoBoard, Point, Stone};
use go::mcts::{self_play_game, GoMctsConfig};
use candle_core::Device;
use serde::{Deserialize, Serialize};
use std::f32;
use std::path::PathBuf;
use std::sync::Arc;

/// A single training example (state, policy target, value target).
#[derive(Serialize, Deserialize)]
pub struct GoTrainingExample {
    pub state: Vec<f32>,         // flattened board planes (17*19*19)
    pub policy_target: Vec<f32>, // 361
    pub value_target: f32,
}

/// Configuration for the training loop.
pub struct GoTrainingConfig {
    pub num_simulations: u32, // MCTS simulations per move
    pub temperature: f32,     // initial temperature for move selection
    pub batch_size: usize,
    pub learning_rate: f64,
    pub self_play_games_per_cycle: usize,
    pub training_steps_per_cycle: usize,
}

impl Default for GoTrainingConfig {
    fn default() -> Self {
        Self {
            num_simulations: 800,
            temperature: 1.0,
            batch_size: 32,
            learning_rate: 0.001,
            self_play_games_per_cycle: 100,
            training_steps_per_cycle: 10,
        }
    }
}

/// The main training loop: self‑play → collect examples → train → repeat.
pub struct GoTrainingLoop {
    config: GoTrainingConfig,
    device: Device,
    model: GoNet,
    optimizer: AdamState,
    cas_store: Arc<CasStore>,
}

impl GoTrainingLoop {
    pub async fn new(
        config: GoTrainingConfig,
        device: Device,
        _model_path: Option<PathBuf>,
        cas_store: Arc<CasStore>,
    ) -> Result<Self> {
        // Create a fresh model for now (placeholder)
        let hidden = 256usize;
        let model = GoNet::new_random(hidden);
        // Create Adam state sized to model parameters
        let opt = AdamState::new_from_model(&model);
        Ok(Self {
            config,
            device,
            model,
            optimizer: opt,
            cas_store,
        })
    }

    /// Run one full training cycle: self‑play + training.
    pub async fn run_cycle(&mut self) -> Result<(), anyhow::Error> {
        // 1. Self‑play games and collect examples
        let mut all_examples: Vec<GoTrainingExample> = Vec::new();

        for _ in 0..self.config.self_play_games_per_cycle {
            // Build an evaluator that wraps the current model
            let evaluator =
                match NeuralEvaluator::new_from_model(self.model.clone(), self.device.clone()) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!("failed to create evaluator from model: {}", e);
                        continue;
                    }
                };

            let cfg = GoMctsConfig {
                num_simulations: self.config.num_simulations,
                temperature: self.config.temperature,
                ..Default::default()
            };

            // Run one self-play game and obtain bonsai-go training examples
            let examples = self_play_game(19, &evaluator, &cfg);

            for ex in examples {
                // Parse board JSON back into a GoBoard
                let board: GoBoard = match serde_json::from_str(&ex.board_json) {
                    Ok(b) => b,
                    Err(_) => continue,
                };

                // Determine color to move by counting stones
                let mut black_count = 0usize;
                let mut white_count = 0usize;
                for stone in board.stones.values() {
                    match stone {
                        Stone::Black => black_count += 1,
                        Stone::White => white_count += 1,
                    }
                }
                let to_move = if black_count <= white_count {
                    Stone::Black
                } else {
                    Stone::White
                };

                // Convert board to NN input
                let state = board.to_nn_input(to_move);

                // Convert move_probs (gtp -> prob) into flat 361-vector
                let mut policy = vec![0.0f32; (board.size as usize) * (board.size as usize)];
                let mut sum = 0.0f32;
                for (gtp, p) in ex.move_probs.iter() {
                    if let Some(pt) = Point::from_gtp(gtp, board.size) {
                        let idx = pt.y as usize * board.size as usize + pt.x as usize;
                        policy[idx] = *p;
                        sum += *p;
                    }
                }
                if sum > 0.0 {
                    for v in policy.iter_mut() {
                        *v /= sum;
                    }
                } else {
                    let n = policy.len() as f32;
                    for v in policy.iter_mut() {
                        *v = 1.0 / n;
                    }
                }

                let value = ex.game_result.unwrap_or(0.5);

                all_examples.push(GoTrainingExample {
                    state,
                    policy_target: policy,
                    value_target: value,
                });
            }
        }

        // 2. Store examples in CAS (deduplicated)
        if !all_examples.is_empty() {
            let examples_key = self.store_examples_in_cas(&all_examples).await?;
            tracing::info!(
                "Stored {} examples in CAS key {}",
                all_examples.len(),
                examples_key.hex()
            );
            // 3. Train the network on the collected examples
            self.train_on_examples(&all_examples).await?;
            // 4. Save the updated model
            self.save_model().await?;
        }

        Ok(())
    }

    /// Helper to store training examples in CAS
    async fn store_examples_in_cas(
        &self,
        examples: &[GoTrainingExample],
    ) -> Result<cas::CasKey, anyhow::Error> {
        let json = serde_json::to_vec(examples)?;
        let key = self.cas_store.put(&json, "application/jsonl").await?;
        Ok(key)
    }

    async fn train_on_examples(
        &mut self,
        examples: &[GoTrainingExample],
    ) -> Result<(), anyhow::Error> {
        tracing::info!("train_on_examples: received {} examples", examples.len());
        let bs = self.config.batch_size;
        let lr = self.config.learning_rate as f32;
        let wd = 1e-4f32; // weight decay

        for batch in examples.chunks(bs) {
            let batch_size = batch.len();
            // Prepare inputs and targets
            let mut inputs: Vec<Vec<f32>> = Vec::with_capacity(batch_size);
            let mut policy_targets: Vec<&[f32]> = Vec::with_capacity(batch_size);
            let mut value_targets: Vec<f32> = Vec::with_capacity(batch_size);
            for ex in batch {
                inputs.push(ex.state.clone());
                policy_targets.push(&ex.policy_target);
                value_targets.push(ex.value_target);
            }

            // Forward
            let (logits, values, activations) = self.model.forward_batch(&inputs);

            // Grad accumulators
            let mut grad_w_policy = vec![0.0f32; self.model.w_policy.len()];
            let mut grad_b_policy = vec![0.0f32; self.model.b_policy.len()];
            let mut grad_w_value = vec![0.0f32; self.model.w_value.len()];
            let mut grad_b_value = 0.0f32;
            let mut grad_w1 = vec![0.0f32; self.model.w1.len()];
            let mut grad_b1 = vec![0.0f32; self.model.b1.len()];

            // For each sample compute gradients
            for s in 0..batch_size {
                // softmax
                let l = &logits[s];
                let maxl = l.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let mut exps = vec![0.0f32; l.len()];
                let mut sumexp = 0.0f32;
                for (i, v) in l.iter().enumerate() {
                    let e = (v - maxl).exp();
                    exps[i] = e;
                    sumexp += e;
                }
                if sumexp <= 0.0 {
                    sumexp = 1.0;
                }
                let mut probs = vec![0.0f32; l.len()];
                for i in 0..l.len() {
                    probs[i] = exps[i] / sumexp;
                }

                let tgt = policy_targets[s];

                // delta for policy: probs - target
                let mut delta_policy = vec![0.0f32; probs.len()];
                for k in 0..probs.len() {
                    delta_policy[k] = probs[k] - tgt[k];
                }

                // value loss grad: 2*(pred-target)/batch_size
                let pred_val = values[s];
                let targ_val = value_targets[s];
                let dvalue = 2.0f32 * (pred_val - targ_val) / batch_size as f32;

                // accumulate grads for policy head and value head
                let act = &activations[s];
                // policy head grads
                for k in 0..self.model.policy_size {
                    grad_b_policy[k] += delta_policy[k];
                    let base = k * self.model.hidden;
                    for i in 0..self.model.hidden {
                        grad_w_policy[base + i] += delta_policy[k] * act[i];
                    }
                }
                // value head grads
                grad_b_value += dvalue;
                for i in 0..self.model.hidden {
                    grad_w_value[i] += dvalue * act[i];
                }

                // backprop into hidden: dL/da = W_policy^T * delta_policy + W_value * dvalue
                let mut dadt = vec![0.0f32; self.model.hidden];
                for (i, dadt_i) in dadt.iter_mut().enumerate() {
                    let mut ssum = 0.0f32;
                    for (k, dp_k) in delta_policy.iter().enumerate() {
                        ssum += self.model.w_policy[k * self.model.hidden + i] * dp_k;
                    }
                    ssum += self.model.w_value[i] * dvalue;
                    *dadt_i = ssum;
                }

                // apply ReLU derivative
                for i in 0..self.model.hidden {
                    let dh = if activations[s][i] > 0.0 {
                        dadt[i]
                    } else {
                        0.0
                    };
                    grad_b1[i] += dh;
                    let base = i * self.model.in_features;
                    for j in 0..self.model.in_features {
                        grad_w1[base + j] += dh * inputs[s][j];
                    }
                }
            }

            // Average grads over batch
            let inv_bs = 1.0f32 / batch_size as f32;
            for v in grad_b_policy.iter_mut() {
                *v *= inv_bs;
            }
            for v in grad_w_policy.iter_mut() {
                *v *= inv_bs;
            }
            grad_b_value *= inv_bs;
            for v in grad_w_value.iter_mut() {
                *v *= inv_bs;
            }
            for v in grad_b1.iter_mut() {
                *v *= inv_bs;
            }
            for v in grad_w1.iter_mut() {
                *v *= inv_bs;
            }

            // Apply AdamW update to parameters
            self.optimizer
                .step_adamw(&mut self.model.w_policy, &grad_w_policy, lr, wd);
            self.optimizer
                .step_adamw(&mut self.model.b_policy, &grad_b_policy, lr, wd);
            self.optimizer
                .step_adamw(&mut self.model.w_value, &grad_w_value, lr, wd);
            self.optimizer
                .step_adamw_scalar(&mut self.model.b_value, grad_b_value, lr, wd);
            self.optimizer
                .step_adamw(&mut self.model.w1, &grad_w1, lr, wd);
            self.optimizer
                .step_adamw(&mut self.model.b1, &grad_b1, lr, wd);
        }

        Ok(())
    }

    async fn save_model(&self) -> Result<(), anyhow::Error> {
        let path = std::env::temp_dir().join("bonsai_go_model.json");
        match self.model.save(&path) {
            Ok(_) => tracing::info!("Model saved to {:?}", path),
            Err(e) => tracing::warn!("Failed to save model: {}", e),
        }
        Ok(())
    }

    // Additional helpers (board_to_tensor, prepare_batch, etc.) should be added here.
}

fn adamw_vec(
    t: u64,
    param: &mut [f32],
    grad: &[f32],
    m: &mut [f32],
    v: &mut [f32],
    lr: f32,
    wd: f32,
) {
    const B1: f32 = 0.9;
    const B2: f32 = 0.999;
    const EPS: f32 = 1e-8;
    let tf = t as f32;
    let b1t = 1.0 - B1.powf(tf);
    let b2t = 1.0 - B2.powf(tf);
    for i in 0..param.len() {
        let g = grad[i];
        m[i] = B1 * m[i] + (1.0 - B1) * g;
        v[i] = B2 * v[i] + (1.0 - B2) * g * g;
        let m_hat = m[i] / b1t;
        let v_hat = v[i] / b2t;
        param[i] -= lr * (m_hat / (v_hat.sqrt() + EPS) + wd * param[i]);
    }
}

fn adamw_scalar(t: u64, param: &mut f32, grad: f32, m: &mut f32, v: &mut f32, lr: f32, wd: f32) {
    const B1: f32 = 0.9;
    const B2: f32 = 0.999;
    const EPS: f32 = 1e-8;
    let tf = t as f32;
    let b1t = 1.0 - B1.powf(tf);
    let b2t = 1.0 - B2.powf(tf);
    *m = B1 * *m + (1.0 - B1) * grad;
    *v = B2 * *v + (1.0 - B2) * grad * grad;
    let m_hat = *m / b1t;
    let v_hat = *v / b2t;
    *param -= lr * (m_hat / (v_hat.sqrt() + EPS) + wd * *param);
}

/// Lightweight AdamW state for flat parameter vectors.
pub struct AdamState {
    pub t: u64,
    pub m_w_policy: Vec<f32>,
    pub v_w_policy: Vec<f32>,
    pub m_b_policy: Vec<f32>,
    pub v_b_policy: Vec<f32>,
    pub m_w_value: Vec<f32>,
    pub v_w_value: Vec<f32>,
    pub m_b_value: f32,
    pub v_b_value: f32,
    pub m_w1: Vec<f32>,
    pub v_w1: Vec<f32>,
    pub m_b1: Vec<f32>,
    pub v_b1: Vec<f32>,
}

impl AdamState {
    pub fn new_from_model(m: &GoNet) -> Self {
        Self {
            t: 0,
            m_w_policy: vec![0.0f32; m.w_policy.len()],
            v_w_policy: vec![0.0f32; m.w_policy.len()],
            m_b_policy: vec![0.0f32; m.b_policy.len()],
            v_b_policy: vec![0.0f32; m.b_policy.len()],
            m_w_value: vec![0.0f32; m.w_value.len()],
            v_w_value: vec![0.0f32; m.w_value.len()],
            m_b_value: 0.0,
            v_b_value: 0.0,
            m_w1: vec![0.0f32; m.w1.len()],
            v_w1: vec![0.0f32; m.w1.len()],
            m_b1: vec![0.0f32; m.b1.len()],
            v_b1: vec![0.0f32; m.b1.len()],
        }
    }

    pub fn step_adamw(&mut self, param: &mut [f32], grad: &[f32], lr: f32, weight_decay: f32) {
        self.t += 1;
        let t = self.t;
        if param.len() == self.m_w_policy.len() {
            adamw_vec(
                t,
                param,
                grad,
                &mut self.m_w_policy,
                &mut self.v_w_policy,
                lr,
                weight_decay,
            );
        } else if param.len() == self.m_w1.len() {
            adamw_vec(
                t,
                param,
                grad,
                &mut self.m_w1,
                &mut self.v_w1,
                lr,
                weight_decay,
            );
        } else if param.len() == self.m_b_policy.len() {
            adamw_vec(
                t,
                param,
                grad,
                &mut self.m_b_policy,
                &mut self.v_b_policy,
                lr,
                weight_decay,
            );
        } else if param.len() == self.m_w_value.len() {
            adamw_vec(
                t,
                param,
                grad,
                &mut self.m_w_value,
                &mut self.v_w_value,
                lr,
                weight_decay,
            );
        } else if param.len() == self.m_b1.len() {
            adamw_vec(
                t,
                param,
                grad,
                &mut self.m_b1,
                &mut self.v_b1,
                lr,
                weight_decay,
            );
        } else {
            for i in 0..param.len() {
                param[i] -= lr * (grad[i] + weight_decay * param[i]);
            }
        }
    }

    pub fn step_adamw_scalar(&mut self, param: &mut f32, grad: f32, lr: f32, weight_decay: f32) {
        self.t += 1;
        adamw_scalar(
            self.t,
            param,
            grad,
            &mut self.m_b_value,
            &mut self.v_b_value,
            lr,
            weight_decay,
        );
    }

    // Additional helpers (board_to_tensor, prepare_batch, etc.) should be added here.
}
