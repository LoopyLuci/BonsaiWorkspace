use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Small fully-connected network used for quick experiments and training scaffolding.
/// Input: 17 * 19 * 19 planes (AlphaZero-style). Hidden: configurable.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GoNet {
    pub in_features: usize,
    pub hidden: usize,
    pub policy_size: usize,
    // weights stored row-major: w1[i * in_features + j]
    pub w1: Vec<f32>,
    pub b1: Vec<f32>,
    // policy head: w_policy[k * hidden + i]
    pub w_policy: Vec<f32>,
    pub b_policy: Vec<f32>,
    // value head: w_value[i]
    pub w_value: Vec<f32>,
    pub b_value: f32,
}

impl GoNet {
    pub fn new_random(hidden: usize) -> Self {
        let board = 19usize;
        let in_features = 17 * board * board;
        let policy_size = board * board;
        let mut rng = StdRng::from_entropy();

        let mut w1 = vec![0.0f32; hidden * in_features];
        let b1 = vec![0.0f32; hidden];
        let mut w_policy = vec![0.0f32; policy_size * hidden];
        let b_policy = vec![0.0f32; policy_size];
        let mut w_value = vec![0.0f32; hidden];

        // He init for ReLU
        let std1 = (2.0f32 / in_features as f32).sqrt();
        let d1 = rand_distr::Normal::<f32>::new(0.0, std1).unwrap();
        for v in w1.iter_mut() {
            *v = d1.sample(&mut rng);
        }

        let stdp = (2.0f32 / hidden as f32).sqrt();
        let dp = rand_distr::Normal::<f32>::new(0.0, stdp).unwrap();
        for v in w_policy.iter_mut() {
            *v = dp.sample(&mut rng);
        }
        for v in w_value.iter_mut() {
            *v = dp.sample(&mut rng);
        }

        Self {
            in_features,
            hidden,
            policy_size,
            w1,
            b1,
            w_policy,
            b_policy,
            w_value,
            b_value: 0.0,
        }
    }

    /// Forward for a batch of inputs (each input is flat Vec<f32> of length `in_features`).
    /// Returns (logits [batch x policy_size], values [batch]) and activations [batch x hidden]
    pub fn forward_batch(&self, inputs: &[Vec<f32>]) -> (Vec<Vec<f32>>, Vec<f32>, Vec<Vec<f32>>) {
        let batch = inputs.len();
        let mut activations = vec![vec![0.0f32; self.hidden]; batch];

        for (s, x) in inputs.iter().enumerate() {
            let h = &mut activations[s];
            for (i, h_i) in h.iter_mut().enumerate() {
                let mut sum = self.b1[i];
                let base = i * self.in_features;
                for (j, x_j) in x.iter().enumerate() {
                    sum += self.w1[base + j] * x_j;
                }
                *h_i = if sum > 0.0 { sum } else { 0.0 }; // ReLU
            }
        }

        let mut logits = vec![vec![0.0f32; self.policy_size]; batch];
        let mut values = vec![0.0f32; batch];

        for s in 0..batch {
            let h = &activations[s];
            // policy head
            for (k, l) in logits[s].iter_mut().enumerate() {
                let mut sum = self.b_policy[k];
                let base = k * self.hidden;
                for (i, h_i) in h.iter().enumerate() {
                    sum += self.w_policy[base + i] * h_i;
                }
                *l = sum;
            }
            // value head
            let mut vsum = self.b_value;
            for (i, act_i) in activations[s].iter().enumerate() {
                vsum += self.w_value[i] * act_i;
            }
            values[s] = vsum;
        }

        (logits, values, activations)
    }

    pub fn forward_single(&self, input: &[f32]) -> (Vec<f32>, f32, Vec<f32>) {
        let (l, v, act) = self.forward_batch(std::slice::from_ref(&input.to_vec()));
        (
            l.into_iter().next().unwrap(),
            v.into_iter().next().unwrap(),
            act.into_iter().next().unwrap(),
        )
    }

    /// Save model parameters to disk as JSON (simple checkpoint).
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let j = serde_json::to_vec(self).map_err(std::io::Error::other)?;
        std::fs::write(p, &j)
    }
}
