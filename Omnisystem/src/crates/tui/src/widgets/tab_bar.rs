use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{app::PanelId, panel::PanelMeta, theme::Theme};

pub fn render_tab_bar(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    panels: &[PanelMeta],
    active: PanelId,
) {
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" ", Style::default()));

    for (i, meta) in panels.iter().enumerate() {
        let label = format!("{}:{} ", i + 1, meta.name);
        if meta.id == active {
            spans.push(Span::styled(
                label,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
        } else {
            spans.push(Span::styled(label, Style::default().fg(theme.muted)));
        }
    }

    spans.push(Span::styled(" F1:Help ", Style::default().fg(theme.muted)));
    spans.push(Span::styled(":cmd", Style::default().fg(theme.warning)));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg));
    frame.render_widget(paragraph, area);
}
