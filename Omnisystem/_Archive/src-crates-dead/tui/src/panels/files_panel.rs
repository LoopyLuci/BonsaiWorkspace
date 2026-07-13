use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use serde_json::Value;
use std::path::PathBuf;

use crate::{app::PanelId, mode::Mode, panel::Panel, theme::Theme};

pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
}

pub struct FilesPanel {
    pub entries: Vec<FileEntry>,
    pub selected: usize,
    pub root: PathBuf,
}

impl FilesPanel {
    pub fn new() -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut panel = FilesPanel {
            entries: Vec::new(),
            selected: 0,
            root,
        };
        panel.refresh();
        panel
    }

    fn refresh(&mut self) {
        self.entries.clear();
        self.walk_dir(&self.root.clone(), 0, 2);
    }

    fn walk_dir(&mut self, path: &PathBuf, depth: usize, max_depth: usize) {
        if depth > max_depth {
            return;
        }
        if let Ok(read_dir) = std::fs::read_dir(path) {
            let mut entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
            entries.sort_by_key(|e| {
                let is_file = e.file_type().map(|t| t.is_file()).unwrap_or(false);
                (is_file, e.file_name())
            });

            for entry in entries {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                self.entries.push(FileEntry { name, is_dir, depth });
                if is_dir && depth < max_depth {
                    self.walk_dir(&entry.path(), depth + 1, max_depth);
                }
            }
        }
    }
}

impl Panel for FilesPanel {
    fn id(&self) -> PanelId { PanelId::Files }
    fn name(&self) -> &str { "Files" }
    fn icon(&self) -> &str { "" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let indent = "  ".repeat(entry.depth);
                let icon = if entry.is_dir { "▶ " } else { "  " };
                let label = format!("{}{}{}", indent, icon, entry.name);
                let style = if i == self.selected {
                    Style::default().fg(theme.accent).bg(theme.selection).add_modifier(Modifier::BOLD)
                } else if entry.is_dir {
                    Style::default().fg(theme.warning)
                } else {
                    Style::default().fg(theme.fg)
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(format!(" Files: {} ", self.root.display()))
            .title_style(Style::default().fg(theme.accent));

        let list = List::new(items).block(block).style(Style::default().bg(theme.bg));
        frame.render_widget(list, area);
    }

    fn handle_key(&mut self, key: KeyEvent, _mode: &mut Mode) -> bool {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.selected + 1 < self.entries.len() {
                    self.selected += 1;
                }
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                true
            }
            KeyCode::Enter => {
                // stub: expand/collapse
                true
            }
            KeyCode::Char('l') => {
                // stub: open in editor
                true
            }
            _ => false,
        }
    }

    fn handle_daemon_event(&mut self, _event: &Value) {}

    fn run_command(&mut self, _cmd: &str, _args: &[&str]) -> Option<String> {
        None
    }
}
