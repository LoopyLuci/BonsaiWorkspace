use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use serde_json::Value;

use crate::{app::PanelId, mode::Mode, panel::Panel, theme::Theme};

pub struct EstimateResult {
    pub eta_min: f64,
    pub eta_max: f64,
    pub credits: f64,
    pub free_eta: Option<f64>,
}

pub struct EstimatePanel {
    pub task_type: String,
    pub units: f64,
    pub paid_urv: f64,
    pub result: Option<EstimateResult>,
    pub cursor_field: u8,
    field_inputs: [String; 3],
}

impl EstimatePanel {
    pub fn new() -> Self {
        EstimatePanel {
            task_type: String::new(),
            units: 0.0,
            paid_urv: 0.0,
            result: None,
            cursor_field: 0,
            field_inputs: [String::new(), String::new(), String::new()],
        }
    }

    fn run_estimate(&mut self) {
        self.task_type = self.field_inputs[0].clone();
        self.units = self.field_inputs[1].parse().unwrap_or(1.0);
        self.paid_urv = self.field_inputs[2].parse().unwrap_or(0.0);

        // Stub calculation
        let base = self.units * 0.5;
        self.result = Some(EstimateResult {
            eta_min: base * 0.8,
            eta_max: base * 1.2,
            credits: base * 0.1,
            free_eta: if self.paid_urv > 0.0 { None } else { Some(base * 2.0) },
        });
    }
}

impl Panel for EstimatePanel {
    fn id(&self) -> PanelId { PanelId::Estimate }
    fn name(&self) -> &str { "Estimate" }
    fn icon(&self) -> &str { "~" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

        let field_labels = ["Task Type:", "Units:", "Paid URV:"];
        for (i, (label, chunk)) in field_labels.iter().zip(chunks.iter()).enumerate() {
            let is_active = self.cursor_field as usize == i;
            let border_color = if is_active { theme.accent } else { theme.border };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(format!(" {} ", label))
                .title_style(Style::default().fg(if is_active { theme.accent } else { theme.muted }));

            let cursor = if is_active { "█" } else { "" };
            let para = Paragraph::new(format!("{}{}", self.field_inputs[i], cursor))
                .block(block)
                .style(Style::default().fg(theme.fg).bg(theme.bg));
            frame.render_widget(para, *chunk);
        }

        let result_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Estimate Result (Enter to compute) ")
            .title_style(Style::default().fg(theme.muted));

        let result_text = if let Some(r) = &self.result {
            format!(
                "ETA: {:.1}–{:.1} min  |  Cost: {:.2} credits{}",
                r.eta_min,
                r.eta_max,
                r.credits,
                r.free_eta
                    .map(|e| format!("  |  Free ETA: {:.1} min", e))
                    .unwrap_or_default()
            )
        } else {
            "No estimate yet. Fill fields and press Enter.".into()
        };

        let para = Paragraph::new(result_text)
            .block(result_block)
            .style(Style::default().fg(theme.accent).bg(theme.bg));
        frame.render_widget(para, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent, mode: &mut Mode) -> bool {
        match mode {
            Mode::Insert => {
                match key.code {
                    KeyCode::Esc => { *mode = Mode::Normal; true }
                    KeyCode::Tab => {
                        self.cursor_field = (self.cursor_field + 1) % 3;
                        true
                    }
                    KeyCode::Enter => {
                        self.run_estimate();
                        true
                    }
                    KeyCode::Backspace => {
                        let idx = self.cursor_field as usize;
                        self.field_inputs[idx].pop();
                        true
                    }
                    KeyCode::Char(c) => {
                        let idx = self.cursor_field as usize;
                        self.field_inputs[idx].push(c);
                        true
                    }
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

    fn run_command(&mut self, _cmd: &str, _args: &[&str]) -> Option<String> {
        self.run_estimate();
        self.result.as_ref().map(|r| format!("ETA: {:.1}-{:.1} min, cost: {:.2} cr", r.eta_min, r.eta_max, r.credits))
    }
}
