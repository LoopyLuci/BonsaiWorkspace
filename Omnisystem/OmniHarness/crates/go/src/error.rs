use thiserror::Error;

#[derive(Debug, Error)]
pub enum GoError {
    #[error("invalid position: ({0}, {1})")]
    InvalidPosition(u8, u8),
    #[error("occupied: ({0}, {1})")]
    Occupied(u8, u8),
    #[error("suicide move")]
    Suicide,
    #[error("ko violation")]
    Ko,
    #[error("game already over")]
    GameOver,
    #[error("not your turn")]
    WrongTurn,
    #[error("game not found: {0}")]
    GameNotFound(String),
    #[error("invalid SGF: {0}")]
    InvalidSgf(String),
}
