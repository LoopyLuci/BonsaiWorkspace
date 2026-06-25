#!/usr/bin/env python3
"""
Polyglot Pong - Go Runner
Executes Pong implementation in Go and outputs JSON trace
"""

import sys
import json
import subprocess
import tempfile
import os

GO_CODE = r"""
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"strconv"
)

type GameState struct {
	BallX    int `json:"ball_x"`
	BallY    int `json:"ball_y"`
	BallDX   int `json:"ball_dx"`
	BallDY   int `json:"ball_dy"`
	Paddle1Y int `json:"paddle1_y"`
	Paddle2Y int `json:"paddle2_y"`
	Score1   int `json:"score1"`
	Score2   int `json:"score2"`
}

func newGame() GameState {
	return GameState{
		BallX:    32768,
		BallY:    32768,
		BallDX:   1310,
		BallDY:   655,
		Paddle1Y: 32768,
		Paddle2Y: 32768,
		Score1:   0,
		Score2:   0,
	}
}

func update(state GameState, up1, down1, up2, down2 bool) GameState {
	p1 := state.Paddle1Y
	if up1 {
		p1 -= 1966
		if p1 < 0 {
			p1 = 0
		}
	}
	if down1 {
		p1 += 1966
		if p1 > 65536 {
			p1 = 65536
		}
	}

	p2 := state.Paddle2Y
	if up2 {
		p2 -= 1966
		if p2 < 0 {
			p2 = 0
		}
	}
	if down2 {
		p2 += 1966
		if p2 > 65536 {
			p2 = 65536
		}
	}

	x := state.BallX + state.BallDX
	y := state.BallY + state.BallDY
	dx := state.BallDX
	dy := state.BallDY

	if x < 3277 && absInt(y-p1) < 6553 {
		dx = -dx
	} else if x > 62259 && absInt(y-p2) < 6553 {
		dx = -dx
	}

	if y < 0 || y > 65536 {
		dy = -dy
	}

	s1 := state.Score1
	s2 := state.Score2
	if x < 0 {
		s2++
		x, y = 32768, 32768
		dx, dy = 1310, 655
	} else if x > 65536 {
		s1++
		x, y = 32768, 32768
		dx, dy = 1310, 655
	}

	return GameState{
		BallX:    x,
		BallY:    y,
		BallDX:   dx,
		BallDY:   dy,
		Paddle1Y: p1,
		Paddle2Y: p2,
		Score1:   s1,
		Score2:   s2,
	}
}

func absInt(x int) int {
	if x < 0 {
		return -x
	}
	return x
}

func main() {
	seed := int64(42)
	frames := 1000

	if len(os.Args) > 1 {
		s, _ := strconv.ParseInt(os.Args[1], 10, 64)
		seed = s
	}
	if len(os.Args) > 2 {
		f, _ := strconv.Atoi(os.Args[2])
		frames = f
	}

	state := newGame()
	var trace []GameState

	inputs := [][4]bool{
		{false, false, false, false},
		{true, false, false, false},
		{false, false, true, false},
		{false, true, false, false},
		{false, false, false, true},
	}

	for i := 0; i < frames; i++ {
		inp := inputs[i%len(inputs)]
		state = update(state, inp[0], inp[1], inp[2], inp[3])
		trace = append(trace, state)
	}

	data, _ := json.Marshal(trace)
	fmt.Println(string(data))
}
"""

if __name__ == "__main__":
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 42
    frames = int(sys.argv[2]) if len(sys.argv) > 2 else 1000

    try:
        with tempfile.TemporaryDirectory() as tmpdir:
            src_file = os.path.join(tmpdir, "pong.go")
            bin_file = os.path.join(tmpdir, "pong")

            with open(src_file, "w") as f:
                f.write(GO_CODE)

            # Compile
            result = subprocess.run(
                ["go", "build", "-o", bin_file, src_file],
                capture_output=True,
                timeout=30,
                cwd=tmpdir
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

            print(result.stdout)
    except Exception as e:
        sys.stderr.write(str(e))
        sys.exit(1)
