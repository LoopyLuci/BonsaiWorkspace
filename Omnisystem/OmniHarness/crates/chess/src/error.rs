use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChessError {
    #[error("invalid FEN: {0}")]
    InvalidFen(String),
    #[error("invalid move: {0}")]
    InvalidMove(String),
    #[error("illegal move: {0}")]
    IllegalMove(String),
    #[error("game already over")]
    GameOver,
    #[error("not your turn")]
    WrongTurn,
    #[error("game not found: {0}")]
    GameNotFound(String),
}
