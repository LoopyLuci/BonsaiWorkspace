#!/usr/bin/env python3
"""
Polyglot Pong - Rust Runner
Executes Pong implementation (compiled from Rust) and outputs JSON trace
"""

import sys
import json
import subprocess
import tempfile
import os

RUST_CODE = r"""
#[derive(Clone, Copy, Debug, serde::Serialize)]
struct GameState {{
    ball_x: i32,
    ball_y: i32,
    ball_dx: i32,
    ball_dy: i32,
    paddle1_y: i32,
    paddle2_y: i32,
    score1: u8,
    score2: u8,
}}

fn new_game() -> GameState {{
    GameState {{
        ball_x: 32768,
        ball_y: 32768,
        ball_dx: 1310,
        ball_dy: 655,
        paddle1_y: 32768,
        paddle2_y: 32768,
        score1: 0,
        score2: 0,
    }}
}}

fn update(mut state: GameState, up1: bool, down1: bool, up2: bool, down2: bool) -> GameState {{
    // Paddle movement
    if up1 {{
        state.paddle1_y = (state.paddle1_y - 1966).max(0);
    }}
    if down1 {{
        state.paddle1_y = (state.paddle1_y + 1966).min(65536);
    }}
    if up2 {{
        state.paddle2_y = (state.paddle2_y - 1966).max(0);
    }}
    if down2 {{
        state.paddle2_y = (state.paddle2_y + 1966).min(65536);
    }}

    // Ball movement
    state.ball_x = state.ball_x.wrapping_add(state.ball_dx);
    state.ball_y = state.ball_y.wrapping_add(state.ball_dy);

    // Paddle collisions
    if state.ball_x < 3277 && (state.ball_y - state.paddle1_y).abs() < 6553 {{
        state.ball_dx = -state.ball_dx;
    }} else if state.ball_x > 62259 && (state.ball_y - state.paddle2_y).abs() < 6553 {{
        state.ball_dx = -state.ball_dx;
    }}

    // Wall collisions
    if state.ball_y < 0 || state.ball_y > 65536 {{
        state.ball_dy = -state.ball_dy;
    }}

    // Scoring
    if state.ball_x < 0 {{
        state.score2 += 1;
        state.ball_x = 32768;
        state.ball_y = 32768;
        state.ball_dx = 1310;
        state.ball_dy = 655;
    }} else if state.ball_x > 65536 {{
        state.score1 += 1;
        state.ball_x = 32768;
        state.ball_y = 32768;
        state.ball_dx = 1310;
        state.ball_dy = 655;
    }}

    state
}}

fn main() {{
    let seed: u64 = std::env::args().nth(1).unwrap_or("42".into()).parse().unwrap();
    let frames: usize = std::env::args().nth(2).unwrap_or("1000".into()).parse().unwrap();

    let mut state = new_game();
    let mut trace = vec![];

    let inputs = [
        (false, false, false, false),
        (true, false, false, false),
        (false, false, true, false),
        (false, true, false, false),
        (false, false, false, true),
    ];

    for i in 0..frames {{
        let (up1, down1, up2, down2) = inputs[i % inputs.len()];
        state = update(state, up1, down1, up2, down2);
        trace.push(state);
    }}

    println!("{{}}", serde_json::to_string(&trace).unwrap());
}}
"""

def run_pong(seed, frames):
    """Compile and run Rust Pong"""
    with tempfile.TemporaryDirectory() as tmpdir:
        src_file = os.path.join(tmpdir, "pong.rs")
        bin_file = os.path.join(tmpdir, "pong")

        # Write Rust source
        with open(src_file, "w") as f:
            f.write(RUST_CODE)

        # Compile
        result = subprocess.run(
            ["rustc", "-O", src_file, "-o", bin_file, "--edition", "2021"],
            capture_output=True,
            timeout=30
        )

        if result.returncode != 0:
            raise RuntimeError(f"Compilation failed: {result.stderr.decode()}")

        # Run
        result = subprocess.run(
            [bin_file, str(seed), str(frames)],
            capture_output=True,
            timeout=60,
            text=True
        )

        if result.returncode != 0:
            raise RuntimeError(f"Execution failed: {result.stderr}")

        return json.loads(result.stdout)

if __name__ == "__main__":
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 42
    frames = int(sys.argv[2]) if len(sys.argv) > 2 else 1000

    try:
        trace = run_pong(seed, frames)
        print(json.dumps(trace))
    except Exception as e:
        sys.stderr.write(str(e))
        sys.exit(1)
