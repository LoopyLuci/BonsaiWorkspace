use crate::evaluator::NeuralEvaluator;
use go::board::{GoBoard, Stone};
use go::mcts::{go_search, GoMctsConfig};
use candle_core::Device;
use std::sync::Arc;

/// A stub for self‑play training. In production, this would:
/// - Run many self‑play games using the current neural network.
/// - Collect game records (states, policies, outcomes).
/// - Periodically train the network via supervised learning.
pub async fn self_play_loop(
    model_path: &str,
    device: Device,
    num_games: usize,
) -> anyhow::Result<()> {
    let evaluator = Arc::new(NeuralEvaluator::new(model_path, device)?);
    let config = GoMctsConfig {
        num_simulations: 100,
        temperature: 1.0,
        ..Default::default()
    };

    for game_idx in 0..num_games {
        let mut board = GoBoard::new(19);
        let mut moves = Vec::new();
        tracing::info!("Starting self-play game {}", game_idx + 1);
        let mut color = Stone::Black;

        while !board.is_terminal() {
            let result = go_search(&board, color, evaluator.as_ref(), &config);
            let best_move = result.best_move.clone();
            if best_move == "pass" || best_move.is_empty() {
                board.pass();
            } else if let Some(pt) = result.best_point {
                let _ = board.place_stone(pt, color);
            } else {
                // No valid move
                break;
            }
            moves.push(best_move);
            color = color.opponent();
        }

        tracing::info!("Game {} finished after {} moves", game_idx + 1, moves.len());
    }

    Ok(())
}
