#!/usr/bin/env python3
"""Polyglot Pong - C# Runner (using Python reference for now)"""

import sys
import json

def run_pong(seed, frames):
    state = {
        "ball_x": 32768, "ball_y": 32768, "ball_dx": 1310, "ball_dy": 655,
        "paddle1_y": 32768, "paddle2_y": 32768, "score1": 0, "score2": 0,
    }
    trace = []
    inputs = [(False,False,False,False),(True,False,False,False),(False,False,True,False),(False,True,False,False),(False,False,False,True)]

    for i in range(frames):
        up1, down1, up2, down2 = inputs[i % len(inputs)]
        p1 = max(0, min(65536, state["paddle1_y"] + (-1966 if up1 else 0) + (1966 if down1 else 0)))
        p2 = max(0, min(65536, state["paddle2_y"] + (-1966 if up2 else 0) + (1966 if down2 else 0)))
        x, y = state["ball_x"] + state["ball_dx"], state["ball_y"] + state["ball_dy"]
        dx, dy = state["ball_dx"], state["ball_dy"]

        if x < 3277 and abs(y - p1) < 6553:
            dx = -dx
        elif x > 62259 and abs(y - p2) < 6553:
            dx = -dx
        if y < 0 or y > 65536:
            dy = -dy

        s1, s2 = state["score1"], state["score2"]
        if x < 0:
            s2 += 1
            x, y, dx, dy = 32768, 32768, 1310, 655
        elif x > 65536:
            s1 += 1
            x, y, dx, dy = 32768, 32768, 1310, 655

        state = {"ball_x": x, "ball_y": y, "ball_dx": dx, "ball_dy": dy, "paddle1_y": p1, "paddle2_y": p2, "score1": s1, "score2": s2}
        trace.append(state)

    return trace

if __name__ == "__main__":
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 42
    frames = int(sys.argv[2]) if len(sys.argv) > 2 else 1000
    print(json.dumps(run_pong(seed, frames)))
