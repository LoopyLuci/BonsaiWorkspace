//! Canonical Pong Specification (Deterministic, Fixed-Point)
//!
//! All game logic is expressed in 16.16 fixed-point integers.
//! This ensures bit-identical execution across all 750+ languages.

use super::GameState;
use serde::{Serialize, Deserialize};

/// Canonical Pong specification - language-agnostic, deterministic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalSpec {
    pub initial_state: GameState,
    pub physics_rules: PhysicsRules,
    pub render_config: RenderConfig,
    pub frame_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsRules {
    // Collision boundaries (16.16 fixed-point)
    pub paddle_width: i32,      // 1310 = 0.02 * 65536
    pub paddle_height: i32,     // 6553 = 0.1 * 65536
    pub ball_radius: i32,       // 655 = 0.01 * 65536
    pub canvas_width: i32,      // 65536 = 1.0 normalized
    pub canvas_height: i32,     // 65536 = 1.0 normalized

    // Physics parameters
    pub gravity: i32,           // 0 (Pong has no gravity)
    pub paddle_speed: i32,      // 1966 = 0.03 * 65536
    pub max_ball_speed: i32,    // 3932 = 0.06 * 65536
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderConfig {
    pub canvas_width_pixels: u32,
    pub canvas_height_pixels: u32,
    pub text_color: String,
    pub background_color: String,
}

impl CanonicalSpec {
    /// Create the standard Pong specification (deterministic).
    pub fn standard() -> Self {
        CanonicalSpec {
            initial_state: GameState {
                ball_x: 32768,      // 0.5 * 65536 (center)
                ball_y: 32768,      // 0.5 * 65536 (center)
                ball_dx: 1310,      // 0.02 * 65536 (small initial velocity)
                ball_dy: 655,       // 0.01 * 65536
                paddle1_y: 32768,   // 0.5 * 65536 (center)
                paddle2_y: 32768,
                score1: 0,
                score2: 0,
            },
            physics_rules: PhysicsRules {
                paddle_width: 1310,     // 0.02 * 65536
                paddle_height: 6553,    // 0.1 * 65536
                ball_radius: 655,       // 0.01 * 65536
                canvas_width: 65536,
                canvas_height: 65536,
                gravity: 0,
                paddle_speed: 1966,     // 0.03 * 65536
                max_ball_speed: 3932,   // 0.06 * 65536
            },
            render_config: RenderConfig {
                canvas_width_pixels: 640,
                canvas_height_pixels: 480,
                text_color: "WHITE".to_string(),
                background_color: "BLACK".to_string(),
            },
            frame_count: 3600,  // 60 FPS × 60 seconds
        }
    }

    /// Compute next game state (deterministic, no floating-point).
    pub fn step(&self, state: &GameState, input: u8) -> GameState {
        let mut next = state.clone();

        // Update ball position (integer arithmetic only)
        next.ball_x += next.ball_dx;
        next.ball_y += next.ball_dy;

        // Paddle collision (left)
        if next.ball_x < self.physics_rules.paddle_width + 1000
            && (next.ball_y - next.paddle1_y).abs() < self.physics_rules.paddle_height
        {
            next.ball_dx = -next.ball_dx.abs(); // Bounce right
        }

        // Paddle collision (right)
        if next.ball_x > self.physics_rules.canvas_width - self.physics_rules.paddle_width - 1000
            && (next.ball_y - next.paddle2_y).abs() < self.physics_rules.paddle_height
        {
            next.ball_dx = next.ball_dx.abs(); // Bounce left
        }

        // Wall collisions
        if next.ball_y < 0 || next.ball_y > self.physics_rules.canvas_height {
            next.ball_dy = -next.ball_dy;
        }

        // Scoring
        if next.ball_x < 0 {
            next.score2 += 1;
            next = self.reset_ball(next);
        }
        if next.ball_x > self.physics_rules.canvas_width {
            next.score1 += 1;
            next = self.reset_ball(next);
        }

        // Paddle movement (deterministic from input)
        if (input & 0x01) != 0 {
            next.paddle1_y = (next.paddle1_y - self.physics_rules.paddle_speed).max(0);
        }
        if (input & 0x02) != 0 {
            next.paddle1_y = (next.paddle1_y + self.physics_rules.paddle_speed)
                .min(self.physics_rules.canvas_height);
        }

        next
    }

    fn reset_ball(&self, mut state: GameState) -> GameState {
        state.ball_x = self.initial_state.ball_x;
        state.ball_y = self.initial_state.ball_y;
        state.ball_dx = self.initial_state.ball_dx;
        state.ball_dy = self.initial_state.ball_dy;
        state
    }

    /// Generate deterministic input sequence from seed.
    pub fn input_sequence(&self, seed: u64, frame_count: u32) -> Vec<u8> {
        let mut inputs = Vec::with_capacity(frame_count as usize);
        let mut rng_state = seed;

        for _ in 0..frame_count {
            // Deterministic LCG (Linear Congruential Generator)
            rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
            let input = ((rng_state >> 32) & 0x03) as u8;
            inputs.push(input);
        }

        inputs
    }

    /// Execute the full game trace deterministically.
    pub fn execute(&self, seed: u64) -> Vec<GameState> {
        let inputs = self.input_sequence(seed, self.frame_count);
        let mut trace = vec![self.initial_state];

        for input in inputs {
            let next = self.step(trace.last().unwrap(), input);
            trace.push(next);
        }

        trace
    }

    /// Hash the specification (for verification).
    pub fn hash(&self) -> [u8; 32] {
        let bytes = serde_json::to_vec(self).unwrap();
        blake3::hash(&bytes).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_execution() {
        let spec = CanonicalSpec::standard();
        let trace1 = spec.execute(42);
        let trace2 = spec.execute(42);

        assert_eq!(trace1, trace2, "Same seed should produce identical traces");
    }

    #[test]
    fn test_different_seeds_differ() {
        let spec = CanonicalSpec::standard();
        let trace1 = spec.execute(42);
        let trace2 = spec.execute(43);

        assert_ne!(trace1, trace2, "Different seeds should produce different traces");
    }

    #[test]
    fn test_ball_never_escapes_canvas() {
        let spec = CanonicalSpec::standard();
        let trace = spec.execute(42);

        for state in trace {
            assert!(state.ball_x >= 0 && state.ball_x <= 65536, "Ball X out of bounds");
            assert!(state.ball_y >= 0 && state.ball_y <= 65536, "Ball Y out of bounds");
        }
    }

    #[test]
    fn test_scores_increase_monotonically() {
        let spec = CanonicalSpec::standard();
        let trace = spec.execute(42);

        let mut last_score1 = 0;
        let mut last_score2 = 0;

        for state in trace {
            assert!(state.score1 >= last_score1, "Score1 decreased");
            assert!(state.score2 >= last_score2, "Score2 decreased");
            last_score1 = state.score1;
            last_score2 = state.score2;
        }
    }
}
