use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem},
    Frame,
};
use serde_json::Value;

use crate::{app::PanelId, mode::Mode, panel::Panel, theme::Theme};

pub struct TrainingPhase {
    pub name: String,
    pub status: String,
    pub epochs_done: u32,
    pub epochs_total: u32,
}

pub struct TrainerPanel {
    pub phases: Vec<TrainingPhase>,
    pub selected: usize,
    pub logs: Vec<String>,
    pub progress: f64,
}

impl TrainerPanel {
    pub fn new() -> Self {
        TrainerPanel {
            phases: vec![
                TrainingPhase { name: "Pretraining".into(), status: "idle".into(), epochs_done: 0, epochs_total: 10 },
                TrainingPhase { name: "Fine-tuning".into(), status: "idle".into(), epochs_done: 0, epochs_total: 5 },
                TrainingPhase { name: "DPO Safety".into(), status: "idle".into(), epochs_done: 0, epochs_total: 3 },
            ],
            selected: 0,
            logs: vec!["Trainer ready. Use :train start <phase> to begin.".into()],
            progress: 0.0,
        }
    }
}

impl Panel for TrainerPanel {
    fn id(&self) -> PanelId { PanelId::Trainer }
    fn name(&self) -> &str { "Trainer" }
    fn icon(&self) -> &str { "" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

        // Top: phase list with gauges
        let phase_chunks = Layout::vertical(
            self.phases.iter().map(|_| Constraint::Length(3)).collect::<Vec<_>>(),
        )
        .split(chunks[0]);

        for (i, phase) in self.phases.iter().enumerate() {
            if i >= phase_chunks.len() { break; }
            let pct = if phase.epochs_total > 0 {
                (phase.epochs_done as f64 / phase.epochs_total as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let label = format!(
                " {} [{}] {}/{} epochs",
                phase.name, phase.status, phase.epochs_done, phase.epochs_total
            );
            let color = if i == self.selected { theme.accent } else { theme.muted };
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(color)))
                .gauge_style(Style::default().fg(color).bg(theme.bg))
                .ratio(pct)
                .label(label);
            frame.render_widget(gauge, phase_chunks[i]);
        }

        // Bottom: logs
        let log_items: Vec<ListItem> = self
            .logs
            .iter()
            .map(|l| ListItem::new(Line::from(Span::styled(l.as_str(), Style::default().fg(theme.fg)))))
            .collect();

        let log_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Training Logs ")
            .title_style(Style::default().fg(theme.muted));

        let log_list = List::new(log_items).block(log_block).style(Style::default().bg(theme.bg));
        frame.render_widget(log_list, chunks[1]);
    }

    fn handle_key(&mut self, key: KeyEvent, _mode: &mut Mode) -> bool {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.selected + 1 < self.phases.len() { self.selected += 1; }
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 { self.selected -= 1; }
                true
            }
            _ => false,
        }
    }

    fn handle_daemon_event(&mut self, event: &Value) {
        if let Some(msg) = event.get("params").and_then(|p| p.get("log")).and_then(|l| l.as_str()) {
            self.logs.push(msg.to_string());
        }
    }

    fn run_command(&mut self, cmd: &str, args: &[&str]) -> Option<String> {
        if cmd == "train" && args.first() == Some(&"start") {
            let phase = args.get(1).copied().unwrap_or("unknown");
            self.logs.push(format!("[{}] Starting phase: {}", chrono::Local::now().format("%H:%M:%S"), phase));
            Some(format!("Started training phase: {}", phase))
        } else {
            None
        }
    }
}
