#!/usr/bin/env python3
"""
Polyglot Pong - JavaScript Runner
Executes Pong implementation in Node.js and outputs JSON trace
"""

import sys
import json
import subprocess

JS_CODE = """
function newGame() {
    return {
        ball_x: 32768,
        ball_y: 32768,
        ball_dx: 1310,
        ball_dy: 655,
        paddle1_y: 32768,
        paddle2_y: 32768,
        score1: 0,
        score2: 0,
    };
}

function update(state, up1, down1, up2, down2) {
    let s = {...state};

    // Paddle movement
    if (up1) s.paddle1_y = Math.max(0, s.paddle1_y - 1966);
    if (down1) s.paddle1_y = Math.min(65536, s.paddle1_y + 1966);
    if (up2) s.paddle2_y = Math.max(0, s.paddle2_y - 1966);
    if (down2) s.paddle2_y = Math.min(65536, s.paddle2_y + 1966);

    // Ball movement
    s.ball_x += s.ball_dx;
    s.ball_y += s.ball_dy;

    // Paddle collisions
    if (s.ball_x < 3277 && Math.abs(s.ball_y - s.paddle1_y) < 6553) {
        s.ball_dx = -s.ball_dx;
    } else if (s.ball_x > 62259 && Math.abs(s.ball_y - s.paddle2_y) < 6553) {
        s.ball_dx = -s.ball_dx;
    }

    // Wall collisions
    if (s.ball_y < 0 || s.ball_y > 65536) {
        s.ball_dy = -s.ball_dy;
    }

    // Scoring
    if (s.ball_x < 0) {
        s.score2++;
        s.ball_x = 32768;
        s.ball_y = 32768;
        s.ball_dx = 1310;
        s.ball_dy = 655;
    } else if (s.ball_x > 65536) {
        s.score1++;
        s.ball_x = 32768;
        s.ball_y = 32768;
        s.ball_dx = 1310;
        s.ball_dy = 655;
    }

    return s;
}

const seed = parseInt(process.argv[2]) || 42;
const frames = parseInt(process.argv[3]) || 1000;

let state = newGame();
const trace = [];

const inputs = [
    [false, false, false, false],
    [true, false, false, false],
    [false, false, true, false],
    [false, true, false, false],
    [false, false, false, true],
];

for (let i = 0; i < frames; i++) {
    const [up1, down1, up2, down2] = inputs[i % inputs.length];
    state = update(state, up1, down1, up2, down2);
    trace.push(state);
}

console.log(JSON.stringify(trace));
"""

if __name__ == "__main__":
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 42
    frames = int(sys.argv[2]) if len(sys.argv) > 2 else 1000

    try:
        result = subprocess.run(
            ["node", "-e", JS_CODE, str(seed), str(frames)],
            capture_output=True,
            timeout=60,
            text=True
        )

        if result.returncode != 0:
            raise RuntimeError(f"Execution failed: {result.stderr}")

        print(result.stdout)
    except Exception as e:
        sys.stderr.write(str(e))
        sys.exit(1)
