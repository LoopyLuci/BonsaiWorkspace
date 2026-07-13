use thiserror::Error;

pub type Result<T> = std::result::Result<T, OptimizationError>;

#[derive(Error, Debug)]
pub enum OptimizationError {
    #[error("Queue is empty")]
    Empty,

    #[error("Queue is full")]
    Full,

    #[error("Poisoned lock")]
    PoisonedLock,

    #[error("Unknown error")]
    Unknown,
}
