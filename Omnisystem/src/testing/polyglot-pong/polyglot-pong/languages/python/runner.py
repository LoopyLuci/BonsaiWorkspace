#!/usr/bin/env python3
"""
Polyglot Pong - Python Runner
Executes Pong implementation in Python and outputs JSON trace
"""

import sys
import json

def new_game():
    return {
        "ball_x": 32768,
        "ball_y": 32768,
        "ball_dx": 1310,
        "ball_dy": 655,
        "paddle1_y": 32768,
        "paddle2_y": 32768,
        "score1": 0,
        "score2": 0,
    }

def update(state, up1, down1, up2, down2):
    """Update game state for one frame with fixed-point arithmetic"""
    # Paddle movement
    p1 = state["paddle1_y"]
    if up1:
        p1 = max(0, p1 - 1966)
    if down1:
        p1 = min(65536, p1 + 1966)

    p2 = state["paddle2_y"]
    if up2:
        p2 = max(0, p2 - 1966)
    if down2:
        p2 = min(65536, p2 + 1966)

    # Ball movement
    x = (state["ball_x"] + state["ball_dx"]) & 0xFFFFFFFF
    y = (state["ball_y"] + state["ball_dy"]) & 0xFFFFFFFF
    dx = state["ball_dx"]
    dy = state["ball_dy"]

    # Paddle collisions
    if x < 3277 and abs(y - p1) < 6553:
        dx = -dx
    elif x > 62259 and abs(y - p2) < 6553:
        dx = -dx

    # Wall collisions
    if y < 0 or y > 65536:
        dy = -dy

    # Scoring
    s1, s2 = state["score1"], state["score2"]
    reset = False
    if x < 0:
        s2 += 1
        reset = True
    elif x > 65536:
        s1 += 1
        reset = True

    if reset:
        x, y = 32768, 32768
        dx, dy = 1310, 655

    return {
        "ball_x": x,
        "ball_y": y,
        "ball_dx": dx,
        "ball_dy": dy,
        "paddle1_y": p1,
        "paddle2_y": p2,
        "score1": s1,
        "score2": s2,
    }

def run_pong(seed, frames):
    """Run Pong game with deterministic inputs and return trace"""
    state = new_game()
    trace = []

    # Deterministic input pattern
    inputs = [
        (False, False, False, False),
        (True, False, False, False),
        (False, False, True, False),
        (False, True, False, False),
        (False, False, False, True),
    ]

    for i in range(frames):
        up1, down1, up2, down2 = inputs[i % len(inputs)]
        state = update(state, up1, down1, up2, down2)
        trace.append(state)

    return trace

if __name__ == "__main__":
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 42
    frames = int(sys.argv[2]) if len(sys.argv) > 2 else 1000

    trace = run_pong(seed, frames)
    print(json.dumps(trace))
