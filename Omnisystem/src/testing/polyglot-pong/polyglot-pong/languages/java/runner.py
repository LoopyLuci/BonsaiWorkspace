#!/usr/bin/env python3
"""Polyglot Pong - Java Runner"""

import sys
import json
import subprocess
import tempfile
import os

JAVA_CODE = r"""
import java.util.*;

public class Pong {
    static class GameState {
        int ball_x, ball_y, ball_dx, ball_dy;
        int paddle1_y, paddle2_y;
        int score1, score2;

        GameState(int bx, int by, int bdx, int bdy, int p1, int p2, int s1, int s2) {
            ball_x = bx; ball_y = by; ball_dx = bdx; ball_dy = bdy;
            paddle1_y = p1; paddle2_y = p2; score1 = s1; score2 = s2;
        }
    }

    static GameState update(GameState s, boolean up1, boolean down1, boolean up2, boolean down2) {
        int p1 = s.paddle1_y;
        if (up1) p1 = Math.max(0, p1 - 1966);
        if (down1) p1 = Math.min(65536, p1 + 1966);

        int p2 = s.paddle2_y;
        if (up2) p2 = Math.max(0, p2 - 1966);
        if (down2) p2 = Math.min(65536, p2 + 1966);

        int x = s.ball_x + s.ball_dx;
        int y = s.ball_y + s.ball_dy;
        int dx = s.ball_dx, dy = s.ball_dy;

        if (x < 3277 && Math.abs(y - p1) < 6553) dx = -dx;
        else if (x > 62259 && Math.abs(y - p2) < 6553) dx = -dx;

        if (y < 0 || y > 65536) dy = -dy;

        int s1 = s.score1, s2 = s.score2;
        if (x < 0) { s2++; x = y = 32768; dx = 1310; dy = 655; }
        else if (x > 65536) { s1++; x = y = 32768; dx = 1310; dy = 655; }

        return new GameState(x, y, dx, dy, p1, p2, s1, s2);
    }

    public static void main(String[] args) throws Exception {
        long seed = args.length > 0 ? Long.parseLong(args[0]) : 42;
        int frames = args.length > 1 ? Integer.parseInt(args[1]) : 1000;

        GameState s = new GameState(32768, 32768, 1310, 655, 32768, 32768, 0, 0);
        StringBuilder out = new StringBuilder("[");

        boolean[][] inputs = {{false,false,false,false},{true,false,false,false},{false,false,true,false},{false,true,false,false},{false,false,false,true}};

        for (int i = 0; i < frames; i++) {
            boolean[] inp = inputs[i % inputs.length];
            s = update(s, inp[0], inp[1], inp[2], inp[3]);
            if (i > 0) out.append(",");
            out.append(String.format("{\"ball_x\":%d,\"ball_y\":%d,\"ball_dx\":%d,\"ball_dy\":%d,\"paddle1_y\":%d,\"paddle2_y\":%d,\"score1\":%d,\"score2\":%d}", s.ball_x, s.ball_y, s.ball_dx, s.ball_dy, s.paddle1_y, s.paddle2_y, s.score1, s.score2));
        }
        out.append("]");
        System.out.println(out);
    }
}
"""

if __name__ == "__main__":
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 42
    frames = int(sys.argv[2]) if len(sys.argv) > 2 else 1000

    try:
        with tempfile.TemporaryDirectory() as tmpdir:
            src = os.path.join(tmpdir, "Pong.java")
            with open(src, "w") as f:
                f.write(JAVA_CODE)

            subprocess.run(["javac", src], capture_output=True, timeout=30, check=True)
            result = subprocess.run(
                ["java", "-cp", tmpdir, "Pong", str(seed), str(frames)],
                capture_output=True, timeout=60, text=True
            )
            print(result.stdout)
    except Exception as e:
        sys.stderr.write(str(e))
        sys.exit(1)
