//! Monte Carlo Tree Search engine for chess.
//!
//! Implements the AlphaZero-style MCTS with UCB exploration and
//! neural-network-guided policy/value evaluation.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::position::ChessPosition;

// ── Evaluator trait ───────────────────────────────────────────────────────────

/// Board evaluator: provides policy (move probabilities) and value (win probability).
pub trait BoardEvaluator: Send + Sync {
    /// Returns a probability vector over legal moves (must sum to 1).
    fn evaluate_policy(&self, pos: &ChessPosition) -> Vec<f32>;
    /// Returns win probability for the current player (0.0 to 1.0).
    fn evaluate_value(&self, pos: &ChessPosition) -> f32;
}

/// Uniform random policy + material heuristic value (fallback before neural net).
pub struct MaterialEvaluator;

impl BoardEvaluator for MaterialEvaluator {
    fn evaluate_policy(&self, pos: &ChessPosition) -> Vec<f32> {
        let n = pos.legal_moves_uci().len();
        if n == 0 { return vec![]; }
        vec![1.0 / n as f32; n]
    }
    fn evaluate_value(&self, pos: &ChessPosition) -> f32 {
        crate::position::material_evaluation(pos)
    }
}

// ── MCTS Node ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MctsNode {
    /// Times this node was visited.
    pub visit_count: u32,
    /// Sum of value backpropagated through this node.
    pub total_value: f32,
    /// Prior probability from the policy network.
    pub prior: f32,
    /// Children: UCI move → child node.
    pub children: HashMap<String, MctsNode>,
    /// Whether this is a terminal position.
    pub is_terminal: bool,
    /// Terminal value if is_terminal (from current player's perspective).
    pub terminal_value: Option<f32>,
}

impl Default for MctsNode {
    fn default() -> Self {
        Self {
            visit_count: 0,
            total_value: 0.0,
            prior: 0.0,
            children: HashMap::new(),
            is_terminal: false,
            terminal_value: None,
        }
    }
}

impl MctsNode {
    /// UCB score used during selection.
    /// Q(s,a) + c_puct × P(s,a) × sqrt(N(s)) / (1 + N(s,a))
    pub fn ucb_score(&self, parent_visits: u32, c_puct: f32) -> f32 {
        let q = if self.visit_count == 0 {
            0.5 // Optimistic prior for unvisited nodes
        } else {
            self.total_value / self.visit_count as f32
        };
        let u = c_puct * self.prior * (parent_visits as f32).sqrt()
            / (1.0 + self.visit_count as f32);
        q + u
    }

    /// Best child by visit count (final move selection).
    pub fn best_move_by_visits(&self) -> Option<&str> {
        self.children.iter()
            .max_by_key(|(_, n)| n.visit_count)
            .map(|(mv, _)| mv.as_str())
    }

    /// Best child by Q-value (used for evaluation games).
    pub fn best_move_by_value(&self) -> Option<&str> {
        self.children.iter()
            .filter(|(_, n)| n.visit_count > 0)
            .max_by(|(_, a), (_, b)| {
                let qa = a.total_value / a.visit_count as f32;
                let qb = b.total_value / b.visit_count as f32;
                qa.partial_cmp(&qb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(mv, _)| mv.as_str())
    }

    /// Policy distribution over children weighted by visit count^(1/temperature).
    pub fn move_probabilities(&self, temperature: f32) -> Vec<(String, f32)> {
        if self.children.is_empty() { return vec![]; }

        let total_visits: u32 = self.children.values().map(|c| c.visit_count).sum();
        if total_visits == 0 {
            let n = self.children.len() as f32;
            return self.children.keys().map(|k| (k.clone(), 1.0 / n)).collect();
        }

        let raw: Vec<(String, f32)> = if temperature == 0.0 {
            // Greedy: pick only the most-visited
            let max_v = self.children.values().map(|c| c.visit_count).max().unwrap_or(0);
            self.children.iter()
                .map(|(k, c)| (k.clone(), if c.visit_count == max_v { 1.0 } else { 0.0 }))
                .collect()
        } else {
            self.children.iter()
                .map(|(k, c)| {
                    let v = (c.visit_count as f32 / total_visits as f32).powf(1.0 / temperature);
                    (k.clone(), v)
                })
                .collect()
        };

        let sum: f32 = raw.iter().map(|(_, v)| v).sum();
        raw.into_iter().map(|(k, v)| (k, v / sum)).collect()
    }
}

// ── MCTS Engine ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MctsConfig {
    /// Number of simulations per move.
    pub num_simulations: u32,
    /// UCB exploration constant.
    pub c_puct: f32,
    /// Temperature for move selection (1.0 = proportional, 0.0 = greedy).
    pub temperature: f32,
    /// Dirichlet noise alpha (chess: 0.3, Go: 0.03).
    pub dirichlet_alpha: f32,
    /// Dirichlet noise weight at root (0.25).
    pub dirichlet_weight: f32,
    /// Maximum game length before declaring draw.
    pub max_moves: u32,
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self {
            num_simulations: 800,
            c_puct: 1.25,
            temperature: 1.0,
            dirichlet_alpha: 0.3,
            dirichlet_weight: 0.25,
            max_moves: 500,
        }
    }
}

impl MctsConfig {
    /// Fast config for training self-play (fewer sims for speed).
    pub fn training() -> Self {
        Self { num_simulations: 200, temperature: 1.0, ..Default::default() }
    }
    /// Strong config for evaluation matches.
    pub fn strong() -> Self {
        Self { num_simulations: 3200, temperature: 0.0, ..Default::default() }
    }
    /// Quick config for real-time user games.
    pub fn interactive() -> Self {
        Self { num_simulations: 400, temperature: 0.0, ..Default::default() }
    }
}

/// MCTS search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Best move in UCI notation.
    pub best_move: String,
    /// Win probability for the current player (0.0–1.0).
    pub value: f32,
    /// Move probability distribution.
    pub move_probs: Vec<(String, f32)>,
    /// Simulations run.
    pub simulations: u32,
}

/// Run MCTS from the given position and return the best move.
pub fn search(
    position: &ChessPosition,
    evaluator: &dyn BoardEvaluator,
    config: &MctsConfig,
) -> SearchResult {
    if position.is_terminal() {
        let val = position.result().unwrap_or(0.5);
        return SearchResult {
            best_move: String::new(),
            value: val,
            move_probs: vec![],
            simulations: 0,
        };
    }

    let legal = position.legal_moves_uci();
    if legal.is_empty() {
        return SearchResult {
            best_move: String::new(),
            value: 0.5,
            move_probs: vec![],
            simulations: 0,
        };
    }

    let mut root = MctsNode::default();

    // Initialize root children with policy priors + Dirichlet noise
    let policy = evaluator.evaluate_policy(position);
    let noise = dirichlet_noise(legal.len(), config.dirichlet_alpha);

    for (i, mv) in legal.iter().enumerate() {
        let prior_raw = policy.get(i).copied().unwrap_or(1.0 / legal.len() as f32);
        let prior = if config.dirichlet_weight > 0.0 {
            (1.0 - config.dirichlet_weight) * prior_raw
                + config.dirichlet_weight * noise[i]
        } else {
            prior_raw
        };
        root.children.insert(mv.clone(), MctsNode { prior, ..Default::default() });
    }

    // Run simulations
    for _ in 0..config.num_simulations {
        run_simulation(&mut root, position, evaluator, config, 0);
    }

    let move_probs = root.move_probabilities(config.temperature);
    let best_move = move_probs.iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(m, _)| m.clone())
        .unwrap_or_default();
    let value = if root.visit_count > 0 {
        root.total_value / root.visit_count as f32
    } else {
        0.5
    };

    SearchResult { best_move, value, move_probs, simulations: config.num_simulations }
}

/// Recursive simulation step. Returns value from the current player's perspective.
fn run_simulation(
    node: &mut MctsNode,
    position: &ChessPosition,
    evaluator: &dyn BoardEvaluator,
    config: &MctsConfig,
    depth: u32,
) -> f32 {
    if node.is_terminal {
        return node.terminal_value.unwrap_or(0.5);
    }

    if depth >= config.max_moves {
        return 0.5; // Draw by length
    }

    // Leaf node: evaluate and expand
    if node.visit_count == 0 || node.children.is_empty() {
        let value = evaluator.evaluate_value(position);
        node.visit_count += 1;
        node.total_value += value;

        // Expand if not already expanded
        if node.children.is_empty() && !position.is_terminal() {
            let legal = position.legal_moves_uci();
            let policy = evaluator.evaluate_policy(position);
            for (i, mv) in legal.iter().enumerate() {
                let prior = policy.get(i).copied().unwrap_or(1.0 / legal.len() as f32);
                node.children.insert(mv.clone(), MctsNode { prior, ..Default::default() });
            }
        }

        return 1.0 - value; // Opponent gets the complementary value
    }

    // Selection: pick best child by UCB
    let parent_visits = node.visit_count;
    let best_mv = node.children.iter()
        .max_by(|(_, a), (_, b)| {
            a.ucb_score(parent_visits, config.c_puct)
                .partial_cmp(&b.ucb_score(parent_visits, config.c_puct))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(mv, _)| mv.clone())
        .unwrap();

    let mut child_pos = position.clone();
    let child_value = if child_pos.make_move_uci(&best_mv).is_ok() {
        let child = node.children.get_mut(&best_mv).unwrap();

        if child_pos.is_terminal() {
            child.is_terminal = true;
            let r = child_pos.result().unwrap_or(0.5);
            // Flip for opponent perspective (position is after the move)
            let v = if child_pos.side_to_move() == "white" { r } else { 1.0 - r };
            child.terminal_value = Some(v);
            child.visit_count += 1;
            child.total_value += v;
            1.0 - v
        } else {
            let v = run_simulation(child, &child_pos, evaluator, config, depth + 1);
            v
        }
    } else {
        0.5
    };

    // Backpropagate
    node.visit_count += 1;
    node.total_value += 1.0 - child_value; // Our perspective is opposite child's
    1.0 - child_value
}

/// Generate Dirichlet noise for root exploration.
fn dirichlet_noise(n: usize, alpha: f32) -> Vec<f32> {
    if n == 0 { return vec![]; }
    // Approximate Dirichlet using Gamma samples
    let mut rng_vals: Vec<f32> = (0..n)
        .map(|_| {
            // Simple Gamma approximation using Box-Muller-style
            let u: f32 = rand::random::<f32>().max(1e-10);
            (-u.ln() * alpha).max(1e-10)
        })
        .collect();
    let sum: f32 = rng_vals.iter().sum();
    for v in &mut rng_vals { *v /= sum; }
    rng_vals
}

// ── Training example ──────────────────────────────────────────────────────────

/// A single training example from a self-play game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    /// FEN of the position.
    pub fen: String,
    /// Move probabilities from MCTS (UCI → probability).
    pub move_probs: Vec<(String, f32)>,
    /// The move that was actually played.
    pub selected_move: String,
    /// Game result from this player's perspective (filled after game ends).
    pub game_result: Option<f32>,
}

/// Play a complete self-play game and return training examples.
pub fn self_play_game(
    evaluator: &dyn BoardEvaluator,
    config: &MctsConfig,
    opening_moves: &[String],
) -> Vec<TrainingExample> {
    let mut pos = ChessPosition::initial();
    let mut examples = Vec::new();

    // Apply opening book moves
    for mv in opening_moves {
        if pos.make_move_uci(mv).is_err() { break; }
    }

    let mut move_count = 0;
    while !pos.is_terminal() && move_count < config.max_moves {
        // Use high temperature for first 30 moves (exploration)
        let temp = if move_count < 30 { 1.0 } else { 0.1 };
        let mut search_config = config.clone();
        search_config.temperature = temp;

        let result = search(&pos, evaluator, &search_config);
        if result.best_move.is_empty() { break; }

        examples.push(TrainingExample {
            fen: pos.to_fen(),
            move_probs: result.move_probs.clone(),
            selected_move: result.best_move.clone(),
            game_result: None,
        });

        if pos.make_move_uci(&result.best_move).is_err() { break; }
        move_count += 1;
    }

    // Fill game results
    let final_result = pos.result().unwrap_or(0.5);
    let n = examples.len();
    for (i, ex) in examples.iter_mut().enumerate() {
        // Alternate perspective: even indices = white's perspective
        let perspective = if i % 2 == 0 { final_result } else { 1.0 - final_result };
        ex.game_result = Some(perspective);
    }

    examples
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::ChessPosition;

    #[test]
    fn mcts_returns_legal_move() {
        let pos = ChessPosition::initial();
        let eval = MaterialEvaluator;
        let config = MctsConfig { num_simulations: 50, ..Default::default() };
        let result = search(&pos, &eval, &config);
        assert!(!result.best_move.is_empty());
        let legal = pos.legal_moves_uci();
        assert!(legal.contains(&result.best_move), "best_move {} not in legal moves", result.best_move);
    }

    #[test]
    fn mcts_terminal_position() {
        // Scholar's mate — white just won
        let mut pos = ChessPosition::initial();
        pos.make_move_uci("e2e4").unwrap();
        pos.make_move_uci("e7e5").unwrap();
        pos.make_move_uci("f1c4").unwrap();
        pos.make_move_uci("b8c6").unwrap();
        pos.make_move_uci("d1h5").unwrap();
        pos.make_move_uci("a7a6").unwrap();
        pos.make_move_uci("h5f7").unwrap();
        assert!(pos.is_terminal());
        let eval = MaterialEvaluator;
        let config = MctsConfig::default();
        let result = search(&pos, &eval, &config);
        assert_eq!(result.simulations, 0); // No search on terminal position
    }
}
