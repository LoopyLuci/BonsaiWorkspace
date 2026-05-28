use thiserror::Error;

#[derive(Debug, Error)]
pub enum MailboxError {
    #[error("unknown recipient: {0}")]
    UnknownRecipient(String),
    #[error("inbox full for agent: {0}")]
    InboxFull(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("relay error: {0}")]
    Relay(#[from] bonsai_relay::error::RelayError),
    #[error("mailbox closed")]
    Closed,
}

pub type MailboxResult<T> = Result<T, MailboxError>;
