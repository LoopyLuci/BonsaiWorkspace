//! MCTS engine for Go — mirrors bonsai-chess MCTS structure.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::board::{GoBoard, Stone, Point};

pub trait GoEvaluator: Send + Sync {
    fn evaluate_policy(&self, board: &GoBoard, color: Stone) -> Vec<(Option<Point>, f32)>;
    fn evaluate_value(&self, board: &GoBoard, color: Stone) -> f32;
}

/// Uniform random policy + simplified territory value (fallback).
pub struct RandomGoEvaluator;

impl GoEvaluator for RandomGoEvaluator {
    fn evaluate_policy(&self, board: &GoBoard, _color: Stone) -> Vec<(Option<Point>, f32)> {
        let mut moves: Vec<Option<Point>> = board.empty_points().into_iter().map(Some).collect();
        moves.push(None); // pass
        let n = moves.len() as f32;
        moves.into_iter().map(|m| (m, 1.0 / n)).collect()
    }

    fn evaluate_value(&self, board: &GoBoard, color: Stone) -> f32 {
        let score = board.final_score(7.5);
        match color {
            Stone::Black => (score / 361.0 + 0.5).clamp(0.0, 1.0),
            Stone::White => (-score / 361.0 + 0.5).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoMctsNode {
    pub visit_count: u32,
    pub total_value: f32,
    pub prior: f32,
    pub children: HashMap<String, GoMctsNode>, // key: GTP coord or "pass"
    pub is_terminal: bool,
    pub terminal_value: Option<f32>,
}

impl Default for GoMctsNode {
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

impl GoMctsNode {
    fn ucb_score(&self, parent_visits: u32, c_puct: f32) -> f32 {
        let q = if self.visit_count == 0 { 0.5 }
                else { self.total_value / self.visit_count as f32 };
        q + c_puct * self.prior * (parent_visits as f32).sqrt()
            / (1.0 + self.visit_count as f32)
    }
}

#[derive(Debug, Clone)]
pub struct GoMctsConfig {
    pub num_simulations: u32,
    pub c_puct: f32,
    pub temperature: f32,
    pub max_moves: u32,
    pub komi: f32,
}

impl Default for GoMctsConfig {
    fn default() -> Self {
        Self { num_simulations: 400, c_puct: 1.25, temperature: 1.0, max_moves: 500, komi: 7.5 }
    }
}

impl GoMctsConfig {
    pub fn interactive() -> Self { Self { num_simulations: 200, temperature: 0.0, ..Default::default() } }
    pub fn training()    -> Self { Self { num_simulations: 100, temperature: 1.0, ..Default::default() } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoSearchResult {
    /// GTP coord of best move, or "pass".
    pub best_move: String,
    pub best_point: Option<Point>,
    pub value: f32,
    pub simulations: u32,
    /// Move probability distribution at the root (GTP coord or "pass" -> prob)
    pub move_probs: Vec<(String, f32)>,
}

pub fn go_search(
    board: &GoBoard,
    color: Stone,
    evaluator: &dyn GoEvaluator,
    config: &GoMctsConfig,
) -> GoSearchResult {
    if board.is_terminal() {
        let score = board.final_score(config.komi);
        let val = match color {
            Stone::Black => if score > 0.0 { 1.0 } else { 0.0 },
            Stone::White => if score < 0.0 { 1.0 } else { 0.0 },
        };
        return GoSearchResult { best_move: "pass".into(), best_point: None, value: val, simulations: 0 };
    }

    let mut root = GoMctsNode::default();
    let policy = evaluator.evaluate_policy(board, color);
    for (pt, prior) in &policy {
        let key = pt.map(|p| p.to_gtp(board.size)).unwrap_or_else(|| "pass".into());
        root.children.insert(key, GoMctsNode { prior: *prior, ..Default::default() });
    }

    for _ in 0..config.num_simulations {
        go_simulate(&mut root, board, color, evaluator, config, 0);
    }

    // Build move probability distribution from child visit counts
    let total_visits: u32 = root.children.values().map(|c| c.visit_count).sum();
    let move_probs: Vec<(String, f32)> = if total_visits == 0 {
        // Fall back to evaluator policy if no visits recorded
        evaluator.evaluate_policy(board, color).into_iter()
            .map(|(pt, p)| (pt.map(|p| p.to_gtp(board.size)).unwrap_or_else(|| "pass".into()), p))
            .collect()
    } else {
        root.children.iter()
            .map(|(k, c)| (k.clone(), c.visit_count as f32 / total_visits as f32))
            .collect()
    };

    // Pick best by visit count
    let best = root.children.iter()
        .max_by_key(|(_, n)| n.visit_count)
        .map(|(k, _)| k.clone())
        .unwrap_or_else(|| "pass".into());

    let best_point = if best == "pass" { None } else { Point::from_gtp(&best, board.size) };
    let value = if root.visit_count > 0 { root.total_value / root.visit_count as f32 } else { 0.5 };

    GoSearchResult { best_move: best, best_point, value, simulations: config.num_simulations, move_probs }
}

// ── Training example / self-play helper ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    /// Serialized board state (JSON of `GoBoard`).
    pub board_json: String,
    /// Move distribution over candidate moves (GTP coord, prob).
    pub move_probs: Vec<(String, f32)>,
    /// Selected (played) move.
    pub selected_move: String,
    /// Game result from this player's perspective (0.0..1.0)
    pub game_result: Option<f32>,
}

/// Play a self-play game using the provided evaluator and config.
pub fn self_play_game(size: u8, evaluator: &dyn GoEvaluator, config: &GoMctsConfig) -> Vec<TrainingExample> {
    let mut board = GoBoard::new(size);
    let mut examples: Vec<TrainingExample> = Vec::new();
    let mut move_count: u32 = 0;
    let mut color = Stone::Black;

    while !board.is_terminal() && move_count < config.max_moves {
        let result = go_search(&board, color, evaluator, config);
        if result.best_move.is_empty() { break; }

        let board_json = serde_json::to_string(&board).unwrap_or_default();
        let ex = TrainingExample {
            board_json,
            move_probs: result.move_probs.clone(),
            selected_move: result.best_move.clone(),
            game_result: None,
        };
        examples.push(ex);

        // Apply the chosen move
        if result.best_move == "pass" {
            board.pass();
        } else if let Some(pt) = Point::from_gtp(&result.best_move, board.size) {
            let _ = board.place_stone(pt, color);
        }

        color = color.opponent();
        move_count += 1;
    }

    // Fill final results (Black advantage -> value > 0 means Black wins)
    let final_score = board.final_score(config.komi);
    let black_wins = final_score > 0.0;
    let draw = final_score.abs() < 1e-6;
    let final_val = if draw { 0.5 } else if black_wins { 1.0 } else { 0.0 };

    for (i, ex) in examples.iter_mut().enumerate() {
        // Even indices correspond to Black to play at that ply (first move Black)
        let perspective = if i % 2 == 0 { final_val } else { 1.0 - final_val };
        ex.game_result = Some(perspective);
    }

    examples
}

/// Simple factory: choose evaluator by name.
pub fn evaluator_from_kind(kind: &str) -> Box<dyn GoEvaluator> {
    match kind {
        "neural" => Box::new(crate::neural::NeuralGoEvaluator::default()),
        _ => Box::new(RandomGoEvaluator),
    }
}

fn go_simulate(
    node: &mut GoMctsNode,
    board: &GoBoard,
    color: Stone,
    evaluator: &dyn GoEvaluator,
    config: &GoMctsConfig,
    depth: u32,
) -> f32 {
    if node.is_terminal {
        return node.terminal_value.unwrap_or(0.5);
    }
    if depth >= config.max_moves {
        return evaluator.evaluate_value(board, color);
    }

    if node.visit_count == 0 || node.children.is_empty() {
        let value = evaluator.evaluate_value(board, color);
        node.visit_count += 1;
        node.total_value += value;

        if node.children.is_empty() && !board.is_terminal() {
            let policy = evaluator.evaluate_policy(board, color);
            for (pt, prior) in policy {
                let key = pt.map(|p| p.to_gtp(board.size)).unwrap_or_else(|| "pass".into());
                node.children.insert(key, GoMctsNode { prior, ..Default::default() });
            }
        }
        return 1.0 - value;
    }

    let parent_visits = node.visit_count;
    let best_key = node.children.iter()
        .max_by(|(_, a), (_, b)| {
            a.ucb_score(parent_visits, config.c_puct)
             .partial_cmp(&b.ucb_score(parent_visits, config.c_puct))
             .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(k, _)| k.clone())
        .unwrap();

    let mut child_board = board.clone();
    let next_color = color.opponent();

    let child_value = if best_key == "pass" {
        child_board.pass();
        let child = node.children.get_mut(&best_key).unwrap();
        if child_board.is_terminal() {
            let score = child_board.final_score(config.komi);
            let v = match next_color {
                Stone::Black => if score > 0.0 { 1.0 } else { 0.0 },
                Stone::White => if score < 0.0 { 1.0 } else { 0.0 },
            };
            child.is_terminal = true;
            child.terminal_value = Some(v);
            child.visit_count += 1;
            child.total_value += v;
            1.0 - v
        } else {
            go_simulate(child, &child_board, next_color, evaluator, config, depth + 1)
        }
    } else {
        let pt = Point::from_gtp(&best_key, board.size);
        let valid = pt.map(|p| child_board.place_stone(p, color).is_ok()).unwrap_or(false);
        if !valid {
            node.children.remove(&best_key);
            return 0.5;
        }
        let child = node.children.get_mut(&best_key).unwrap();
        go_simulate(child, &child_board, next_color, evaluator, config, depth + 1)
    };

    node.visit_count += 1;
    node.total_value += 1.0 - child_value;
    1.0 - child_value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_returns_move() {
        let board = GoBoard::new(9);
        let eval = RandomGoEvaluator;
        let config = GoMctsConfig { num_simulations: 20, ..Default::default() };
        let result = go_search(&board, Stone::Black, &eval, &config);
        assert!(!result.best_move.is_empty());
    }
}
