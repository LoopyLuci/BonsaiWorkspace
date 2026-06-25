//! Language Runner - Executes Pong implementations in arbitrary languages
//!
//! Each language has a runner script that:
//! 1. Takes a seed and frame count
//! 2. Executes Pong with deterministic inputs
//! 3. Outputs a JSON trace of GameState objects

use anyhow::{anyhow, Result};
use polyglot_pong_common::GameState;
use std::path::PathBuf;
use std::time::Instant;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct LanguageRunner {
    language: String,
    runner_path: PathBuf,
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub language: String,
    pub seed: u64,
    pub frames: usize,
    pub trace: Vec<GameState>,
    pub exec_time_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

impl LanguageRunner {
    pub fn new(language: String, runner_path: PathBuf) -> Self {
        Self {
            language,
            runner_path,
        }
    }

    /// Execute the language runner and capture trace
    pub async fn execute(&self, seed: u64, frames: usize) -> Result<ExecutionResult> {
        let start = Instant::now();

        // Run the language-specific runner script
        let output = Command::new("python3")
            .arg(self.runner_path.to_str().unwrap())
            .arg(seed.to_string())
            .arg(frames.to_string())
            .output()
            .await;

        let elapsed_ms = start.elapsed().as_millis() as u64;

        match output {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Ok(ExecutionResult {
                        language: self.language.clone(),
                        seed,
                        frames,
                        trace: Vec::new(),
                        exec_time_ms: elapsed_ms,
                        success: false,
                        error: Some(format!("Runner failed: {}", stderr)),
                    });
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                match serde_json::from_str::<Vec<GameState>>(&stdout) {
                    Ok(trace) => Ok(ExecutionResult {
                        language: self.language.clone(),
                        seed,
                        frames,
                        trace,
                        exec_time_ms: elapsed_ms,
                        success: true,
                        error: None,
                    }),
                    Err(e) => Ok(ExecutionResult {
                        language: self.language.clone(),
                        seed,
                        frames,
                        trace: Vec::new(),
                        exec_time_ms: elapsed_ms,
                        success: false,
                        error: Some(format!("Parse error: {}", e)),
                    }),
                }
            }
            Err(e) => Ok(ExecutionResult {
                language: self.language.clone(),
                seed,
                frames,
                trace: Vec::new(),
                exec_time_ms: elapsed_ms,
                success: false,
                error: Some(format!("Execution error: {}", e)),
            }),
        }
    }
}

/// Canonical reference implementation in pure Rust
pub fn canonical_trace(seed: u64, frames: usize) -> Vec<GameState> {
    let mut state = GameState {
        ball_x: 32768,
        ball_y: 32768,
        ball_dx: 1310,
        ball_dy: 655,
        paddle1_y: 32768,
        paddle2_y: 32768,
        score1: 0,
        score2: 0,
    };

    let input_pattern = [
        (false, false, false, false),
        (true, false, false, false),
        (false, false, true, false),
        (false, true, false, false),
        (false, false, false, true),
    ];

    let mut trace = Vec::with_capacity(frames);

    for i in 0..frames {
        let (up1, down1, up2, down2) = input_pattern[i % input_pattern.len()];

        // Ball movement
        state.ball_x = state.ball_x.wrapping_add(state.ball_dx);
        state.ball_y = state.ball_y.wrapping_add(state.ball_dy);

        // Paddle collisions
        if state.ball_x < 3277 && (state.ball_y - state.paddle1_y).abs() < 6553 {
            state.ball_dx = -state.ball_dx;
        } else if state.ball_x > 62259 && (state.ball_y - state.paddle2_y).abs() < 6553 {
            state.ball_dx = -state.ball_dx;
        }

        // Wall collisions
        if state.ball_y < 0 || state.ball_y > 65536 {
            state.ball_dy = -state.ball_dy;
        }

        // Scoring
        if state.ball_x < 0 {
            state.score2 += 1;
            state.ball_x = 32768;
            state.ball_y = 32768;
            state.ball_dx = 1310;
            state.ball_dy = 655;
        } else if state.ball_x > 65536 {
            state.score1 += 1;
            state.ball_x = 32768;
            state.ball_y = 32768;
            state.ball_dx = 1310;
            state.ball_dy = 655;
        }

        // Paddle movement
        if up1 {
            state.paddle1_y = (state.paddle1_y - 1966).max(0);
        }
        if down1 {
            state.paddle1_y = (state.paddle1_y + 1966).min(65536);
        }
        if up2 {
            state.paddle2_y = (state.paddle2_y - 1966).max(0);
        }
        if down2 {
            state.paddle2_y = (state.paddle2_y + 1966).min(65536);
        }

        trace.push(state);
    }

    trace
}

/// Compute fidelity between two traces
pub fn compute_fidelity(trace1: &[GameState], trace2: &[GameState]) -> f32 {
    if trace1.is_empty() || trace2.is_empty() {
        return 0.0;
    }

    let min_len = trace1.len().min(trace2.len());
    let mut matches = 0;

    for i in 0..min_len {
        if trace1[i] == trace2[i] {
            matches += 1;
        }
    }

    matches as f32 / min_len as f32
}
