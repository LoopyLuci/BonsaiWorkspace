use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem},
    Frame,
};
use serde_json::Value;

use crate::{app::PanelId, mode::Mode, panel::Panel, theme::Theme};

pub struct SidecarStatus {
    pub name: String,
    pub running: bool,
}

pub struct HealthPanel {
    pub cpu_pct: f64,
    pub ram_pct: f64,
    pub sidecars: Vec<SidecarStatus>,
}

impl HealthPanel {
    pub fn new() -> Self {
        HealthPanel {
            cpu_pct: 0.0,
            ram_pct: 0.0,
            sidecars: vec![
                SidecarStatus { name: "bonsai-daemon".into(), running: false },
                SidecarStatus { name: "bonsai-relay".into(), running: false },
                SidecarStatus { name: "bonsai-credits".into(), running: false },
            ],
        }
    }
}

impl Panel for HealthPanel {
    fn id(&self) -> PanelId { PanelId::Health }
    fn name(&self) -> &str { "Health" }
    fn icon(&self) -> &str { "♥" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

        let cpu_gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .title(" CPU ")
                    .title_style(Style::default().fg(theme.muted)),
            )
            .gauge_style(Style::default().fg(theme.accent).bg(theme.bg))
            .ratio(self.cpu_pct.clamp(0.0, 1.0))
            .label(format!("{:.1}%", self.cpu_pct * 100.0));
        frame.render_widget(cpu_gauge, chunks[0]);

        let ram_gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .title(" RAM ")
                    .title_style(Style::default().fg(theme.muted)),
            )
            .gauge_style(Style::default().fg(theme.warning).bg(theme.bg))
            .ratio(self.ram_pct.clamp(0.0, 1.0))
            .label(format!("{:.1}%", self.ram_pct * 100.0));
        frame.render_widget(ram_gauge, chunks[1]);

        let sidecar_items: Vec<ListItem> = self
            .sidecars
            .iter()
            .map(|s| {
                let (indicator, color) = if s.running { ("●", theme.success) } else { ("○", theme.error) };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", indicator), Style::default().fg(color)),
                    Span::styled(s.name.as_str(), Style::default().fg(theme.fg)),
                ]))
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Sidecars ")
            .title_style(Style::default().fg(theme.accent));

        let list = List::new(sidecar_items).block(block).style(Style::default().bg(theme.bg));
        frame.render_widget(list, chunks[2]);
    }

    fn handle_key(&mut self, _key: KeyEvent, _mode: &mut Mode) -> bool { false }

    fn handle_daemon_event(&mut self, event: &Value) {
        if event.get("method").and_then(|v| v.as_str()) == Some("health_update") {
            if let Some(params) = event.get("params") {
                if let Some(cpu) = params.get("cpu_pct").and_then(|v| v.as_f64()) {
                    self.cpu_pct = cpu / 100.0;
                }
                if let Some(ram) = params.get("ram_pct").and_then(|v| v.as_f64()) {
                    self.ram_pct = ram / 100.0;
                }
            }
        }
    }

    fn run_command(&mut self, _cmd: &str, _args: &[&str]) -> Option<String> { None }
}
