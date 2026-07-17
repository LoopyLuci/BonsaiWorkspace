use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use serde_json::Value;

use crate::{app::PanelId, mode::Mode, panel::Panel, theme::Theme};

pub struct TerminalPanel {
    pub output: Vec<String>,
    pub input: String,
}

impl TerminalPanel {
    pub fn new() -> Self {
        TerminalPanel {
            output: vec!["Bonsai Terminal. Type a command and press Enter.".into()],
            input: String::new(),
        }
    }

    fn run_shell(&mut self, cmd: &str) {
        self.output.push(format!("$ {}", cmd));
        let result = if cfg!(target_os = "windows") {
            std::process::Command::new("cmd").args(["/C", cmd]).output()
        } else {
            std::process::Command::new("sh").args(["-c", cmd]).output()
        };

        match result {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                for line in stdout.lines() {
                    self.output.push(line.to_string());
                }
                for line in stderr.lines() {
                    self.output.push(format!("[stderr] {}", line));
                }
            }
            Err(e) => {
                self.output.push(format!("[error] {}", e));
            }
        }
    }
}

impl Panel for TerminalPanel {
    fn id(&self) -> PanelId { PanelId::Terminal }
    fn name(&self) -> &str { "Terminal" }
    fn icon(&self) -> &str { ">" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

        let items: Vec<ListItem> = self
            .output
            .iter()
            .map(|l| ListItem::new(Line::from(Span::styled(l.as_str(), Style::default().fg(theme.fg)))))
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Terminal ")
            .title_style(Style::default().fg(theme.accent));

        let list = List::new(items).block(block).style(Style::default().bg(theme.bg));
        frame.render_widget(list, chunks[0]);

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Input (i=Insert) ")
            .title_style(Style::default().fg(theme.muted));

        let para = Paragraph::new(format!("$ {}_", self.input))
            .block(input_block)
            .style(Style::default().fg(theme.success).bg(theme.bg));
        frame.render_widget(para, chunks[1]);
    }

    fn handle_key(&mut self, key: KeyEvent, mode: &mut Mode) -> bool {
        match mode {
            Mode::Insert => {
                match key.code {
                    KeyCode::Esc => { *mode = Mode::Normal; true }
                    KeyCode::Enter => {
                        let cmd = self.input.clone();
                        self.input.clear();
                        if !cmd.is_empty() {
                            self.run_shell(&cmd);
                        }
                        true
                    }
                    KeyCode::Backspace => { self.input.pop(); true }
                    KeyCode::Char(c) => { self.input.push(c); true }
                    _ => false,
                }
            }
            Mode::Normal => {
                if key.code == KeyCode::Char('i') { *mode = Mode::Insert; return true; }
                false
            }
            _ => false,
        }
    }

    fn handle_daemon_event(&mut self, _event: &Value) {}

    fn run_command(&mut self, cmd: &str, args: &[&str]) -> Option<String> {
        if cmd == "shell" {
            let full = args.join(" ");
            self.run_shell(&full);
            Some("Command executed".into())
        } else {
            None
        }
    }
}
