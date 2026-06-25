#!/usr/bin/env python3
"""Aether Pong – Actor-based Pong game runner."""
import threading, queue, time, sys

WIDTH, HEIGHT = 80, 24
PADDLE_H = 4

class Paddle:
    def __init__(self, side):
        self.side = side
        self.y = HEIGHT // 2 - PADDLE_H // 2

    def move(self, dy):
        self.y = max(0, min(HEIGHT - PADDLE_H, self.y + dy))

class Ball:
    def __init__(self):
        self.x = WIDTH // 2
        self.y = HEIGHT // 2
        self.dx = 2
        self.dy = 1

    def update(self, p1, p2):
        self.x += self.dx
        self.y += self.dy

        if self.y <= 0 or self.y >= HEIGHT - 1:
            self.dy = -self.dy

        if self.x <= 2 and self.y >= p1.y and self.y <= p1.y + PADDLE_H:
            self.dx = -self.dx
        if self.x >= WIDTH - 3 and self.y >= p2.y and self.y <= p2.y + PADDLE_H:
            self.dx = -self.dx

        if self.x <= 0:
            return 'right'
        if self.x >= WIDTH - 1:
            return 'left'
        return None

def run_pong():
    """Run Aether Pong game."""
    p1 = Paddle('left')
    p2 = Paddle('right')
    ball = Ball()
    score_left, score_right = 0, 0
    frame = 0

    print("Aether Pong Game")
    print("Controls: w/s (left paddle), o/l (right paddle), q (quit)")
    print("Auto-playing 100 frames...\n")

    while frame < 100:
        # Display state
        print(f"Frame {frame:03d} | Score: {score_left} - {score_right} | Ball: ({ball.x:2d},{ball.y:2d})")

        # Auto-play: simple deterministic AI
        if ball.y < p1.y + PADDLE_H // 2:
            p1.move(-1)
        elif ball.y > p1.y + PADDLE_H // 2:
            p1.move(1)

        if ball.y < p2.y + PADDLE_H // 2:
            p2.move(-1)
        elif ball.y > p2.y + PADDLE_H // 2:
            p2.move(1)

        # Update ball
        scorer = ball.update(p1, p2)
        if scorer == 'left':
            score_left += 1
            ball = Ball()
        elif scorer == 'right':
            score_right += 1
            ball = Ball()

        time.sleep(0.01)
        frame += 1

    print(f"\nFinal Score: {score_left} - {score_right}")

if __name__ == '__main__':
    run_pong()
