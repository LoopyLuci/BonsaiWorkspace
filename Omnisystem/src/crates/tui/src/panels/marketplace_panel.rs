use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use serde_json::Value;

use crate::{app::PanelId, mode::Mode, panel::Panel, theme::Theme};

pub struct MarketplaceListing {
    pub display_name: String,
    pub urv: f64,
    pub price_per_min: f64,
    pub reliability: f64,
    pub has_gpu: bool,
}

pub struct MarketplacePanel {
    pub listings: Vec<MarketplaceListing>,
    pub selected: usize,
    pub filter_gpu: bool,
    pub active_contracts: Vec<String>,
}

impl MarketplacePanel {
    pub fn new() -> Self {
        MarketplacePanel {
            listings: vec![
                MarketplaceListing {
                    display_name: "node-alpha".into(),
                    urv: 8.5,
                    price_per_min: 0.02,
                    reliability: 0.99,
                    has_gpu: true,
                },
                MarketplaceListing {
                    display_name: "node-beta".into(),
                    urv: 4.2,
                    price_per_min: 0.01,
                    reliability: 0.95,
                    has_gpu: false,
                },
                MarketplaceListing {
                    display_name: "node-gamma".into(),
                    urv: 12.0,
                    price_per_min: 0.05,
                    reliability: 0.97,
                    has_gpu: true,
                },
            ],
            selected: 0,
            filter_gpu: false,
            active_contracts: Vec::new(),
        }
    }

    fn visible_listings(&self) -> Vec<&MarketplaceListing> {
        self.listings
            .iter()
            .filter(|l| !self.filter_gpu || l.has_gpu)
            .collect()
    }
}

impl Panel for MarketplacePanel {
    fn id(&self) -> PanelId { PanelId::Marketplace }
    fn name(&self) -> &str { "Market" }
    fn icon(&self) -> &str { "" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let visible = self.visible_listings();
        let rows: Vec<Row> = visible
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let gpu_str = if l.has_gpu { "GPU" } else { "CPU" };
                let style = if i == self.selected {
                    Style::default().fg(theme.accent).bg(theme.selection).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };
                Row::new(vec![
                    Cell::from(l.display_name.as_str()),
                    Cell::from(format!("{:.1}", l.urv)),
                    Cell::from(format!("{:.4}", l.price_per_min)),
                    Cell::from(format!("{:.0}%", l.reliability * 100.0)),
                    Cell::from(gpu_str),
                ])
                .style(style)
            })
            .collect();

        let widths = [
            Constraint::Percentage(30),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(15),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(format!(
                " Marketplace ({} listings, {} contracts active) ",
                visible.len(),
                self.active_contracts.len()
            ))
            .title_style(Style::default().fg(theme.accent));

        let table = Table::new(rows, widths)
            .block(block)
            .header(
                Row::new(vec!["Name", "URV", "Price/min", "Reliability", "GPU"])
                    .style(Style::default().fg(theme.muted).add_modifier(Modifier::BOLD)),
            )
            .style(Style::default().bg(theme.bg));

        frame.render_widget(table, area);
    }

    fn handle_key(&mut self, key: KeyEvent, _mode: &mut Mode) -> bool {
        let visible_count = self.visible_listings().len();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.selected + 1 < visible_count { self.selected += 1; }
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 { self.selected -= 1; }
                true
            }
            KeyCode::Char('g') => {
                self.filter_gpu = !self.filter_gpu;
                self.selected = 0;
                true
            }
            KeyCode::Char('r') => {
                let visible = self.visible_listings();
                if let Some(listing) = visible.get(self.selected) {
                    self.active_contracts.push(listing.display_name.clone());
                }
                true
            }
            _ => false,
        }
    }

    fn handle_daemon_event(&mut self, _event: &Value) {}

    fn run_command(&mut self, cmd: &str, _args: &[&str]) -> Option<String> {
        match cmd {
            "marketplace" => Some(format!("{} listings available", self.listings.len())),
            "rent" => Some("Rent stub: not connected to daemon".into()),
            _ => None,
        }
    }
}
