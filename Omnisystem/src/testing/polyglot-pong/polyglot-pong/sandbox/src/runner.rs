//! Pong code generation, compilation, and execution
//!
//! Handles the full pipeline: BPLIS code generation → compilation →
//! deterministic execution with trace capture.

use polyglot_pong_common::*;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Pong code runner for a specific language
pub struct PongRunner {
    pub language: Language,
    pub work_dir: PathBuf,
}

impl PongRunner {
    /// Create a new runner for the given language
    pub async fn new(language: Language) -> anyhow::Result<Self> {
        let work_dir = PathBuf::from(format!("/tmp/pong-{}", language.to_lowercase()));

        // Create work directory
        tokio::fs::create_dir_all(&work_dir).await?;

        info!("PongRunner created for language: {}", language);

        Ok(Self { language, work_dir })
    }

    /// Generate Pong implementation code via BPLIS
    pub async fn generate_code(&self, seed: u32) -> anyhow::Result<String> {
        debug!("Generating code for {} (seed: {})", self.language, seed);

        // In production: call BPLIS frontend to generate code
        // For MVP: return template implementation
        let code = self.template_code(seed);

        Ok(code)
    }

    /// Generate template Pong code (fallback if BPLIS unavailable)
    fn template_code(&self, _seed: u32) -> String {
        match self.language.as_str() {
            "Rust" => self.rust_template(),
            "Python" => self.python_template(),
            "JavaScript" => self.js_template(),
            "Go" => self.go_template(),
            "C" => self.c_template(),
            _ => self.generic_template(),
        }
    }

    fn rust_template(&self) -> String {
        r#"
use std::io::{self, Read};

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();

    let mut state = GameState::default();
    for &byte in &input {
        state = step(&state, byte);
        println!("{},{},{},{},{},{},{},{}",
            state.ball_x, state.ball_y, state.ball_dx, state.ball_dy,
            state.paddle_left_y, state.paddle_right_y,
            state.score_left, state.score_right);
    }
}

#[derive(Clone, Copy)]
struct GameState {
    ball_x: i32, ball_y: i32, ball_dx: i32, ball_dy: i32,
    paddle_left_y: i32, paddle_right_y: i32,
    score_left: i32, score_right: i32,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            ball_x: 512 << 16, ball_y: 384 << 16,
            ball_dx: 0, ball_dy: 0,
            paddle_left_y: 256 << 16, paddle_right_y: 256 << 16,
            score_left: 0, score_right: 0,
        }
    }
}

fn step(state: &GameState, _input: u8) -> GameState {
    let mut next = *state;
    next.ball_x = next.ball_x + next.ball_dx;
    next.ball_y = next.ball_y + next.ball_dy;
    next
}
"#.to_string()
    }

    fn python_template(&self) -> String {
        r#"
import sys

class GameState:
    def __init__(self):
        self.ball_x = 512 << 16
        self.ball_y = 384 << 16
        self.ball_dx = 0
        self.ball_dy = 0
        self.paddle_left_y = 256 << 16
        self.paddle_right_y = 256 << 16
        self.score_left = 0
        self.score_right = 0

def step(state, input_byte):
    state.ball_x = state.ball_x + state.ball_dx
    state.ball_y = state.ball_y + state.ball_dy
    return state

state = GameState()
for byte in sys.stdin.buffer.read():
    state = step(state, byte)
    print(f"{state.ball_x},{state.ball_y},{state.ball_dx},{state.ball_dy}," +
          f"{state.paddle_left_y},{state.paddle_right_y}," +
          f"{state.score_left},{state.score_right}")
"#.to_string()
    }

    fn js_template(&self) -> String {
        r#"
const readline = require('readline');

class GameState {
    constructor() {
        this.ball_x = 512 << 16;
        this.ball_y = 384 << 16;
        this.ball_dx = 0;
        this.ball_dy = 0;
        this.paddle_left_y = 256 << 16;
        this.paddle_right_y = 256 << 16;
        this.score_left = 0;
        this.score_right = 0;
    }
}

function step(state, input) {
    state.ball_x = state.ball_x + state.ball_dx;
    state.ball_y = state.ball_y + state.ball_dy;
    return state;
}

let state = new GameState();
const rl = readline.createInterface({ input: process.stdin });
rl.on('line', (line) => {
    for (let byte of line) {
        state = step(state, byte);
        console.log(`${state.ball_x},${state.ball_y},${state.ball_dx},${state.ball_dy},` +
                    `${state.paddle_left_y},${state.paddle_right_y},` +
                    `${state.score_left},${state.score_right}`);
    }
});
"#.to_string()
    }

    fn go_template(&self) -> String {
        r#"
package main

import (
    "bufio"
    "fmt"
    "os"
)

type GameState struct {
    BallX, BallY int32
    BallDX, BallDY int32
    PaddleLeftY, PaddleRightY int32
    ScoreLeft, ScoreRight int32
}

func step(state GameState, input byte) GameState {
    state.BallX = state.BallX + state.BallDX
    state.BallY = state.BallY + state.BallDY
    return state
}

func main() {
    scanner := bufio.NewScanner(os.Stdin)
    state := GameState{
        BallX: 512 << 16, BallY: 384 << 16,
        PaddleLeftY: 256 << 16, PaddleRightY: 256 << 16,
    }

    for scanner.Scan() {
        for _, b := range scanner.Bytes() {
            state = step(state, b)
            fmt.Printf("%d,%d,%d,%d,%d,%d,%d,%d\n",
                state.BallX, state.BallY, state.BallDX, state.BallDY,
                state.PaddleLeftY, state.PaddleRightY,
                state.ScoreLeft, state.ScoreRight)
        }
    }
}
"#.to_string()
    }

    fn c_template(&self) -> String {
        r#"
#include <stdio.h>
#include <stdint.h>

struct GameState {
    int32_t ball_x, ball_y, ball_dx, ball_dy;
    int32_t paddle_left_y, paddle_right_y;
    int32_t score_left, score_right;
};

struct GameState step(struct GameState state, uint8_t input) {
    state.ball_x = state.ball_x + state.ball_dx;
    state.ball_y = state.ball_y + state.ball_dy;
    return state;
}

int main() {
    struct GameState state = {
        512 << 16, 384 << 16, 0, 0,
        256 << 16, 256 << 16, 0, 0
    };

    int c;
    while ((c = getchar()) != EOF) {
        state = step(state, (uint8_t)c);
        printf("%d,%d,%d,%d,%d,%d,%d,%d\n",
            state.ball_x, state.ball_y, state.ball_dx, state.ball_dy,
            state.paddle_left_y, state.paddle_right_y,
            state.score_left, state.score_right);
    }
    return 0;
}
"#.to_string()
    }

    fn generic_template(&self) -> String {
        "// Template for language not yet implemented\n".to_string()
    }

    /// Compile the generated code
    pub async fn compile(&self, _code: &str) -> anyhow::Result<CompiledPong> {
        debug!("Compiling Pong for language: {}", self.language);

        // In production: invoke actual compiler
        // For MVP: return mock compiled binary path
        let binary_path = self.work_dir.join("pong_binary");

        Ok(CompiledPong {
            language: self.language.clone(),
            binary_path,
        })
    }

    /// Measure energy consumption (RAPL on Linux)
    pub async fn measure_energy(&self) -> anyhow::Result<EnergyMetrics> {
        // In production: use polyglot_pong_energy crate
        // For MVP: return default
        Ok(EnergyMetrics {
            package_joules: 0.0,
            core_joules: 0.0,
            dram_joules: 0.0,
        })
    }
}

/// Compiled Pong binary
pub struct CompiledPong {
    pub language: Language,
    pub binary_path: PathBuf,
}

impl CompiledPong {
    /// Execute the compiled binary with deterministic input
    pub async fn execute(&self, input_seq: &[u8]) -> anyhow::Result<Vec<GameState>> {
        info!("Executing Pong binary for language: {}", self.language);

        // In production: actually run binary and capture output
        // For MVP: generate mock trace
        let trace = self.generate_mock_trace();

        Ok(trace)
    }

    fn generate_mock_trace(&self) -> Vec<GameState> {
        let mut trace = Vec::new();
        let mut state = GameState::default();

        for _ in 0..100 {
            trace.push(state);
            state.ball_x += state.ball_dx;
            state.ball_y += state.ball_dy;
        }

        trace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runner_creation() {
        let runner = PongRunner::new("Rust".into()).await;
        assert!(runner.is_ok());
    }

    #[tokio::test]
    async fn test_code_generation() {
        let runner = PongRunner::new("Python".into()).await.unwrap();
        let code = runner.generate_code(42).await.unwrap();
        assert!(!code.is_empty());
        assert!(code.contains("def") || code.contains("GameState"));
    }

    #[tokio::test]
    async fn test_compilation() {
        let runner = PongRunner::new("Go".into()).await.unwrap();
        let code = runner.generate_code(0).await.unwrap();
        let compiled = runner.compile(&code).await.unwrap();
        assert_eq!(compiled.language, "Go");
    }

    #[tokio::test]
    async fn test_execution() {
        let runner = PongRunner::new("Rust".into()).await.unwrap();
        let code = runner.generate_code(0).await.unwrap();
        let compiled = runner.compile(&code).await.unwrap();
        let trace = compiled.execute(&[1, 2, 3]).await.unwrap();
        assert!(!trace.is_empty());
    }
}
