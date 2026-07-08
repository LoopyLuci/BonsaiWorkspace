use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use serde_json::Value;
use uuid::Uuid;

use crate::{app::PanelId, mode::Mode, panel::Panel, theme::Theme};

pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub participants: usize,
    pub active: bool,
}

pub struct CollaborationPanel {
    pub sessions: Vec<SessionInfo>,
    pub selected: usize,
    pub invite_code: String,
}

impl CollaborationPanel {
    pub fn new() -> Self {
        CollaborationPanel {
            sessions: Vec::new(),
            selected: 0,
            invite_code: String::new(),
        }
    }
}

impl Panel for CollaborationPanel {
    fn id(&self) -> PanelId { PanelId::Collaboration }
    fn name(&self) -> &str { "Collab" }
    fn icon(&self) -> &str { "" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

        let items: Vec<ListItem> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let status = if s.active { "●" } else { "○" };
                let style = if i == self.selected {
                    Style::default().fg(theme.accent).bg(theme.selection).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };
                ListItem::new(Line::from(Span::styled(
                    format!(" {} {} ({} participants)", status, s.name, s.participants),
                    style,
                )))
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Sessions (use :collab create / :collab join <code>) ")
            .title_style(Style::default().fg(theme.accent));

        let list = if items.is_empty() {
            List::new(vec![ListItem::new(Line::from(Span::styled(
                " No active sessions. Use :collab create to start one.",
                Style::default().fg(theme.muted),
            )))])
        } else {
            List::new(items)
        };

        frame.render_widget(list.block(block).style(Style::default().bg(theme.bg)), chunks[0]);

        let invite_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Invite Code ")
            .title_style(Style::default().fg(theme.muted));

        let invite_text = if self.invite_code.is_empty() {
            "No active session".to_string()
        } else {
            self.invite_code.clone()
        };

        let para = Paragraph::new(invite_text).block(invite_block).style(Style::default().fg(theme.accent).bg(theme.bg));
        frame.render_widget(para, chunks[1]);
    }

    fn handle_key(&mut self, key: KeyEvent, _mode: &mut Mode) -> bool {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('j') => { if self.selected + 1 < self.sessions.len() { self.selected += 1; } true }
            KeyCode::Char('k') => { if self.selected > 0 { self.selected -= 1; } true }
            _ => false,
        }
    }

    fn handle_daemon_event(&mut self, _event: &Value) {}

    fn run_command(&mut self, cmd: &str, args: &[&str]) -> Option<String> {
        if cmd == "collab" {
            match args.first() {
                Some(&"create") => {
                    let id = Uuid::new_v4().to_string();
                    let code = id[..8].to_uppercase();
                    self.invite_code = code.clone();
                    self.sessions.push(SessionInfo {
                        id: id.clone(),
                        name: format!("Session-{}", &code[..4]),
                        participants: 1,
                        active: true,
                    });
                    Some(format!("Session created. Invite code: {}", code))
                }
                Some(&"join") => {
                    let code = args.get(1).copied().unwrap_or("unknown");
                    Some(format!("Joined session with code: {} (stub)", code))
                }
                _ => Some("Usage: collab create | collab join <code>".into()),
            }
        } else {
            None
        }
    }
}
