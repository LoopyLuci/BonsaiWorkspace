use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalEvent {
    Input(String),
    Output(String),
    Resize { cols: u16, rows: u16 },
    Close,
}

pub struct TerminalShareStream;

impl TerminalShareStream {
    pub fn encode(event: &TerminalEvent) -> Vec<u8> {
        serde_json::to_vec(event).unwrap_or_default()
    }

    pub fn decode(data: &[u8]) -> Option<TerminalEvent> {
        serde_json::from_slice(data).ok()
    }
}
