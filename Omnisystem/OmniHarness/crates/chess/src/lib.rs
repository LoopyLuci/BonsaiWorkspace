pub mod distributed_self_play;
pub mod error;
pub mod game_session;
pub mod mcts;
pub mod network;
pub mod position;

pub use distributed_self_play::{DistributedSelfPlayEngine, DistributedSelfPlayState, GameRecord};
pub use error::ChessError;
pub use game_session::{
    ChessColor, ChessGameSession, DrawReason, GameEndReason, GameResult, MoveRecord, Player,
    PlayerKind,
};
pub use mcts::{
    search, self_play_game, BoardEvaluator, MaterialEvaluator, MctsConfig, MctsNode, SearchResult,
    TrainingExample,
};
pub use network::{
    teacher_distill_examples, train_epoch as chess_train_epoch, AdamState as ChessAdamState,
    ChessNetWeights, NetworkEvaluator,
};
pub use position::{material_evaluation, ChessMove, ChessPosition};
