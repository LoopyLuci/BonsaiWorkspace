use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use serde_json::Value;

use crate::{app::PanelId, mode::Mode, panel::Panel, theme::Theme};

pub struct TxLine {
    pub kind: String,
    pub amount: f64,
    pub timestamp: String,
}

pub struct CreditsPanel {
    pub balance: f64,
    pub pool: f64,
    pub transactions: Vec<TxLine>,
}

impl CreditsPanel {
    pub fn new() -> Self {
        CreditsPanel {
            balance: 42.0,
            pool: 100.0,
            transactions: vec![
                TxLine { kind: "earn".into(), amount: 5.0, timestamp: "2026-05-30 09:00".into() },
                TxLine { kind: "spend".into(), amount: -2.5, timestamp: "2026-05-30 10:00".into() },
            ],
        }
    }

    pub fn get_balance(&self) -> f64 {
        self.balance
    }
}

impl Panel for CreditsPanel {
    fn id(&self) -> PanelId { PanelId::Credits }
    fn name(&self) -> &str { "Credits" }
    fn icon(&self) -> &str { "" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([
            Constraint::Length(4),
            Constraint::Min(1),
        ])
        .split(area);

        let balance_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Wallet ")
            .title_style(Style::default().fg(theme.accent));

        let balance_para = Paragraph::new(Line::from(vec![
            Span::styled("Balance: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{:.2} cr", self.balance),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Pool: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{:.2} cr", self.pool),
                Style::default().fg(theme.success),
            ),
        ]))
        .block(balance_block)
        .style(Style::default().bg(theme.bg));

        frame.render_widget(balance_para, chunks[0]);

        let rows: Vec<Row> = self
            .transactions
            .iter()
            .map(|tx| {
                let color = if tx.amount >= 0.0 { theme.success } else { theme.error };
                Row::new(vec![
                    Cell::from(tx.kind.as_str()),
                    Cell::from(format!("{:+.2}", tx.amount)).style(Style::default().fg(color)),
                    Cell::from(tx.timestamp.as_str()),
                ])
            })
            .collect();

        let widths = [Constraint::Percentage(30), Constraint::Percentage(20), Constraint::Percentage(50)];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" Transactions ")
            .title_style(Style::default().fg(theme.muted));

        let table = Table::new(rows, widths)
            .block(block)
            .header(
                Row::new(vec!["Type", "Amount", "Time"])
                    .style(Style::default().fg(theme.muted).add_modifier(Modifier::BOLD)),
            )
            .style(Style::default().bg(theme.bg));

        frame.render_widget(table, chunks[1]);
    }

    fn handle_key(&mut self, _key: KeyEvent, _mode: &mut Mode) -> bool { false }

    fn handle_daemon_event(&mut self, event: &Value) {
        if let Some(bal) = event.get("params").and_then(|p| p.get("balance")).and_then(|b| b.as_f64()) {
            self.balance = bal;
        }
    }

    fn run_command(&mut self, _cmd: &str, _args: &[&str]) -> Option<String> {
        Some(format!("Balance: {:.2} cr", self.balance))
    }
}
