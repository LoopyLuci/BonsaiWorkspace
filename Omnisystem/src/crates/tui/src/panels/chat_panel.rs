use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use serde_json::Value;

use crate::{app::PanelId, mode::Mode, panel::Panel, theme::Theme};

pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

pub struct ChatPanel {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub scroll: u16,
}

impl ChatPanel {
    pub fn new() -> Self {
        ChatPanel {
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: "Welcome to Bonsai TUI. Type a message and press Enter to chat.".into(),
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                },
            ],
            input: String::new(),
            scroll: 0,
        }
    }
}

impl Panel for ChatPanel {
    fn id(&self) -> PanelId { PanelId::Chat }
    fn name(&self) -> &str { "Chat" }
    fn icon(&self) -> &str { "" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

        // Messages list
        let items: Vec<ListItem> = self
            .messages
            .iter()
            .map(|m| {
                let role_color = match m.role.as_str() {
                    "user" => theme.accent,
                    "assistant" => theme.success,
                    _ => theme.muted,
                };
                let line = Line::from(vec![
                    Span::styled(
                        format!("[{}] ", m.timestamp),
                        Style::default().fg(theme.muted),
                    ),
                    Span::styled(
                        format!("{}: ", m.role),
                        Style::default().fg(role_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(m.content.clone(), Style::default().fg(theme.fg)),
                ]);
                ListItem::new(line)
            })
            .collect();

        let msg_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Chat ")
            .title_style(Style::default().fg(theme.accent));

        let list = List::new(items).block(msg_block).style(Style::default().bg(theme.bg));
        frame.render_widget(list, chunks[0]);

        // Input area
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Message (i=Insert, Esc=Normal) ")
            .title_style(Style::default().fg(theme.muted));

        let input_para = Paragraph::new(self.input.as_str())
            .block(input_block)
            .style(Style::default().fg(theme.fg).bg(theme.bg));

        frame.render_widget(input_para, chunks[1]);
    }

    fn handle_key(&mut self, key: KeyEvent, mode: &mut Mode) -> bool {
        match mode {
            Mode::Insert => {
                match key.code {
                    KeyCode::Esc => {
                        *mode = Mode::Normal;
                        true
                    }
                    KeyCode::Enter => {
                        if !self.input.is_empty() {
                            let content = self.input.clone();
                            self.messages.push(ChatMessage {
                                role: "user".into(),
                                content,
                                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            });
                            self.input.clear();
                        }
                        true
                    }
                    KeyCode::Backspace => {
                        self.input.pop();
                        true
                    }
                    KeyCode::Char(c) => {
                        self.input.push(c);
                        true
                    }
                    _ => false,
                }
            }
            Mode::Normal => {
                if key.code == KeyCode::Char('i') {
                    *mode = Mode::Insert;
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    fn handle_daemon_event(&mut self, event: &Value) {
        if let Some(msg) = event.get("params").and_then(|p| p.get("message")).and_then(|m| m.as_str()) {
            self.messages.push(ChatMessage {
                role: "assistant".into(),
                content: msg.to_string(),
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            });
        }
    }

    fn run_command(&mut self, cmd: &str, args: &[&str]) -> Option<String> {
        if cmd == "send" {
            let content = args.join(" ");
            self.messages.push(ChatMessage {
                role: "user".into(),
                content,
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            });
            Some("Message sent".into())
        } else {
            None
        }
    }
}
