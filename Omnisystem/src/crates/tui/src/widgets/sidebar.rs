use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::{app::PanelId, panel::PanelMeta, theme::Theme};

pub fn render_sidebar(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    panels: &[PanelMeta],
    active: PanelId,
) {
    let items: Vec<ListItem> = panels
        .iter()
        .map(|meta| {
            let label = format!(" {} {}", meta.icon, meta.name);
            if meta.id == active {
                ListItem::new(Line::from(Span::styled(
                    label,
                    Style::default()
                        .fg(theme.accent)
                        .bg(theme.selection)
                        .add_modifier(Modifier::BOLD),
                )))
            } else {
                ListItem::new(Line::from(Span::styled(
                    label,
                    Style::default().fg(theme.fg),
                )))
            }
        })
        .collect();

    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(theme.border))
        .title(" Panels ")
        .title_style(Style::default().fg(theme.muted));

    let list = List::new(items)
        .block(block)
        .style(Style::default().bg(theme.bg));

    frame.render_widget(list, area);
}
