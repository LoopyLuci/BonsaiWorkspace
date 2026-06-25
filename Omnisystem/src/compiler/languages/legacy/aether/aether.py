#!/usr/bin/env python3
"""Aether – Actor-based runtime system."""
import threading, queue, time, sys

class Actor:
    """Base actor that processes messages sequentially."""
    def __init__(self, name, behavior):
        self.name = name
        self.behavior = behavior
        self.mailbox = queue.Queue()
        self.running = True
        self.thread = threading.Thread(target=self.run, daemon=True)
        self.thread.start()

    def send(self, msg):
        """Send message to this actor's mailbox."""
        self.mailbox.put(msg)

    def run(self):
        """Main message processing loop."""
        while self.running:
            try:
                msg = self.mailbox.get(timeout=0.1)
                if msg == ('die',):
                    break
                self.behavior(self, msg)
            except queue.Empty:
                continue

    def stop(self):
        """Stop this actor."""
        self.running = False

class ActorSystem:
    """System for managing actors."""
    def __init__(self):
        self.actors = {}
        self.lock = threading.Lock()

    def spawn(self, name, behavior):
        """Create and register a new actor."""
        with self.lock:
            actor = Actor(name, behavior)
            self.actors[name] = actor
            return actor

    def send(self, to, msg):
        """Send a message to an actor by name."""
        if to in self.actors:
            self.actors[to].send(msg)

    def shutdown(self):
        """Stop all actors."""
        for actor in self.actors.values():
            actor.send(('die',))
            actor.stop()

# Pong game state and actors
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

        # Bounce off walls
        if self.y <= 0 or self.y >= HEIGHT - 1:
            self.dy = -self.dy

        # Bounce off paddles
        if self.x <= 2 and self.y >= p1.y and self.y <= p1.y + PADDLE_H:
            self.dx = -self.dx
        if self.x >= WIDTH - 3 and self.y >= p2.y and self.y <= p2.y + PADDLE_H:
            self.dx = -self.dx

        # Check scoring
        if self.x <= 0:
            return 'right'
        if self.x >= WIDTH - 1:
            return 'left'
        return None

def run_aether_pong():
    """Run Pong using actor model (simulated locally for simplicity)."""
    p1 = Paddle('left')
    p2 = Paddle('right')
    ball = Ball()
    score_left, score_right = 0, 0
    frame = 0

    while True:
        # Display
        print(f"\rFrame {frame:04d} | Score: {score_left} - {score_right} | Ball: ({ball.x:2d},{ball.y:2d})", end='', flush=True)

        # Simulate input
        try:
            ch = input() if frame % 10 == 0 else ''
        except EOFError:
            break

        if ch == 'q':
            break
        elif ch == 'w':
            p1.move(-1)
        elif ch == 's':
            p1.move(1)
        elif ch == 'o':
            p2.move(-1)
        elif ch == 'l':
            p2.move(1)

        # Update ball
        scorer = ball.update(p1, p2)
        if scorer == 'left':
            score_left += 1
            ball = Ball()
        elif scorer == 'right':
            score_right += 1
            ball = Ball()

        time.sleep(0.05)
        frame += 1

    print("\nGame over!")

if __name__ == '__main__':
    run_aether_pong()
