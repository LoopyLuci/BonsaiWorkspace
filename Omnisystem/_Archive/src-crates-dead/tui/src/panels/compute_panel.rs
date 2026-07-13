use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};
use serde_json::Value;

use crate::{app::PanelId, mode::Mode, panel::Panel, theme::Theme};

pub struct FreePoolInfo {
    pub cpu_urv: f64,
    pub gpu_urv: f64,
    pub active_projects: usize,
    pub device_count: usize,
}

pub struct ComputePanel {
    pub free_pool: FreePoolInfo,
    pub my_free_pct: u8,
    pub my_paid_pct: u8,
    pub bonus: f64,
}

impl ComputePanel {
    pub fn new() -> Self {
        ComputePanel {
            free_pool: FreePoolInfo {
                cpu_urv: 42.5,
                gpu_urv: 12.0,
                active_projects: 7,
                device_count: 23,
            },
            my_free_pct: 20,
            my_paid_pct: 30,
            bonus: 1.25,
        }
    }
}

impl Panel for ComputePanel {
    fn id(&self) -> PanelId { PanelId::Compute }
    fn name(&self) -> &str { "Compute" }
    fn icon(&self) -> &str { "" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

        let free_gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .title(" My Free Contribution ")
                    .title_style(Style::default().fg(theme.muted)),
            )
            .gauge_style(Style::default().fg(theme.success).bg(theme.bg))
            .ratio(self.my_free_pct as f64 / 100.0)
            .label(format!("{}%", self.my_free_pct));
        frame.render_widget(free_gauge, chunks[0]);

        let paid_gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .title(" My Paid Contribution ")
                    .title_style(Style::default().fg(theme.muted)),
            )
            .gauge_style(Style::default().fg(theme.accent).bg(theme.bg))
            .ratio(self.my_paid_pct as f64 / 100.0)
            .label(format!("{}%", self.my_paid_pct));
        frame.render_widget(paid_gauge, chunks[1]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Free Pool Stats ")
            .title_style(Style::default().fg(theme.accent));

        let text = format!(
            "CPU URV: {:.1}  GPU URV: {:.1}\nActive Projects: {}  Devices: {}\nBonus Multiplier: {:.2}x",
            self.free_pool.cpu_urv,
            self.free_pool.gpu_urv,
            self.free_pool.active_projects,
            self.free_pool.device_count,
            self.bonus
        );

        let para = Paragraph::new(text).block(block).style(Style::default().fg(theme.fg).bg(theme.bg));
        frame.render_widget(para, chunks[2]);
    }

    fn handle_key(&mut self, _key: KeyEvent, _mode: &mut Mode) -> bool { false }

    fn handle_daemon_event(&mut self, _event: &Value) {}

    fn run_command(&mut self, cmd: &str, args: &[&str]) -> Option<String> {
        if cmd == "contribute" {
            self.my_free_pct = args.first().and_then(|s| s.parse().ok()).unwrap_or(self.my_free_pct);
            self.my_paid_pct = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(self.my_paid_pct);
            Some(format!("Contribution set: free={}%, paid={}%", self.my_free_pct, self.my_paid_pct))
        } else {
            None
        }
    }
}
