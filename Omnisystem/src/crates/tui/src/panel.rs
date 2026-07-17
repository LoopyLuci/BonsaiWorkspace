use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};
use serde_json::Value;

use crate::{app::PanelId, mode::Mode, theme::Theme};

pub trait Panel: Send + Sync {
    fn id(&self) -> PanelId;
    fn name(&self) -> &str;
    fn icon(&self) -> &str;
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    /// Returns true if the key was consumed
    fn handle_key(&mut self, key: KeyEvent, mode: &mut Mode) -> bool;
    fn handle_daemon_event(&mut self, event: &Value);
    /// Returns optional feedback message
    fn run_command(&mut self, cmd: &str, args: &[&str]) -> Option<String>;
}

#[derive(Clone)]
pub struct PanelMeta {
    pub id: PanelId,
    pub name: String,
    pub icon: String,
}
