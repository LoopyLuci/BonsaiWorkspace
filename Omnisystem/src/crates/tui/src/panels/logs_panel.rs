use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use serde_json::Value;

use crate::{app::PanelId, mode::Mode, panel::Panel, theme::Theme};

pub struct LogLine {
    pub level: String,
    pub msg: String,
    pub time: String,
}

pub struct LogsPanel {
    pub lines: Vec<LogLine>,
    pub scroll: usize,
    pub filter: String,
    pub filter_input: String,
}

impl LogsPanel {
    pub fn new() -> Self {
        LogsPanel {
            lines: vec![
                LogLine { level: "INFO".into(), msg: "Bonsai TUI started".into(), time: chrono::Local::now().format("%H:%M:%S").to_string() },
            ],
            scroll: 0,
            filter: String::new(),
            filter_input: String::new(),
        }
    }

    fn visible_lines(&self) -> Vec<&LogLine> {
        self.lines
            .iter()
            .filter(|l| self.filter.is_empty() || l.msg.contains(&self.filter) || l.level.contains(&self.filter))
            .collect()
    }
}

impl Panel for LogsPanel {
    fn id(&self) -> PanelId { PanelId::Logs }
    fn name(&self) -> &str { "Logs" }
    fn icon(&self) -> &str { "" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let visible = self.visible_lines();
        let total = visible.len();
        let start = self.scroll.min(total.saturating_sub(1));

        let items: Vec<ListItem> = visible
            .iter()
            .skip(start)
            .map(|l| {
                let level_color = match l.level.as_str() {
                    "ERROR" => theme.error,
                    "WARN" => theme.warning,
                    "INFO" => theme.success,
                    _ => theme.muted,
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("[{}] ", l.time), Style::default().fg(theme.muted)),
                    Span::styled(
                        format!("{:5} ", l.level),
                        Style::default().fg(level_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(l.msg.as_str(), Style::default().fg(theme.fg)),
                ]))
            })
            .collect();

        let filter_title = if self.filter.is_empty() {
            " Logs (/ to filter, j/k scroll) ".to_string()
        } else {
            format!(" Logs [filter: {}] ", self.filter)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(filter_title)
            .title_style(Style::default().fg(theme.accent));

        let list = List::new(items).block(block).style(Style::default().bg(theme.bg));
        frame.render_widget(list, area);
    }

    fn handle_key(&mut self, key: KeyEvent, mode: &mut Mode) -> bool {
        match mode {
            Mode::Insert => {
                match key.code {
                    KeyCode::Esc => {
                        self.filter = self.filter_input.clone();
                        *mode = Mode::Normal;
                        true
                    }
                    KeyCode::Enter => {
                        self.filter = self.filter_input.clone();
                        *mode = Mode::Normal;
                        true
                    }
                    KeyCode::Backspace => { self.filter_input.pop(); true }
                    KeyCode::Char(c) => { self.filter_input.push(c); true }
                    _ => false,
                }
            }
            Mode::Normal => {
                match key.code {
                    KeyCode::Char('/') => { self.filter_input.clear(); *mode = Mode::Insert; true }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let max = self.visible_lines().len().saturating_sub(1);
                        if self.scroll < max { self.scroll += 1; }
                        true
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if self.scroll > 0 { self.scroll -= 1; }
                        true
                    }
                    KeyCode::Char('G') => {
                        self.scroll = self.visible_lines().len().saturating_sub(1);
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn handle_daemon_event(&mut self, event: &Value) {
        let msg = serde_json::to_string(event).unwrap_or_default();
        self.lines.push(LogLine {
            level: "DEBUG".into(),
            msg,
            time: chrono::Local::now().format("%H:%M:%S").to_string(),
        });
        // Auto-scroll to bottom
        self.scroll = self.lines.len().saturating_sub(1);
    }

    fn run_command(&mut self, _cmd: &str, _args: &[&str]) -> Option<String> { None }
}
