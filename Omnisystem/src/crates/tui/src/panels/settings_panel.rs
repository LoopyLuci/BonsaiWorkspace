use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use serde_json::Value;

use crate::{app::PanelId, mode::Mode, panel::Panel, theme::Theme};

pub struct SettingItem {
    pub key: String,
    pub value: String,
    pub description: String,
}

pub struct SettingsPanel {
    pub items: Vec<SettingItem>,
    pub selected: usize,
}

impl SettingsPanel {
    pub fn new() -> Self {
        SettingsPanel {
            items: vec![
                SettingItem { key: "theme".into(), value: "dark".into(), description: "UI color theme (dark/light)".into() },
                SettingItem { key: "vim_mode".into(), value: "true".into(), description: "Enable vim-style keybindings".into() },
                SettingItem { key: "fps".into(), value: "60".into(), description: "Target frame rate".into() },
                SettingItem { key: "sidebar_width".into(), value: "20".into(), description: "Sidebar width percentage".into() },
            ],
            selected: 0,
        }
    }
}

impl Panel for SettingsPanel {
    fn id(&self) -> PanelId { PanelId::Settings }
    fn name(&self) -> &str { "Settings" }
    fn icon(&self) -> &str { "" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let rows: Vec<Row> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.selected {
                    Style::default().fg(theme.accent).bg(theme.selection).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };
                Row::new(vec![
                    Cell::from(item.key.as_str()),
                    Cell::from(item.value.as_str()),
                    Cell::from(item.description.as_str()),
                ])
                .style(style)
            })
            .collect();

        let widths = [Constraint::Percentage(20), Constraint::Percentage(20), Constraint::Percentage(60)];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Settings (j/k navigate) ")
            .title_style(Style::default().fg(theme.accent));

        let table = Table::new(rows, widths)
            .block(block)
            .header(
                Row::new(vec!["Key", "Value", "Description"])
                    .style(Style::default().fg(theme.muted).add_modifier(Modifier::BOLD)),
            )
            .style(Style::default().bg(theme.bg));

        frame.render_widget(table, area);
    }

    fn handle_key(&mut self, key: KeyEvent, _mode: &mut Mode) -> bool {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.selected + 1 < self.items.len() { self.selected += 1; }
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 { self.selected -= 1; }
                true
            }
            _ => false,
        }
    }

    fn handle_daemon_event(&mut self, _event: &Value) {}

    fn run_command(&mut self, _cmd: &str, _args: &[&str]) -> Option<String> { None }
}
