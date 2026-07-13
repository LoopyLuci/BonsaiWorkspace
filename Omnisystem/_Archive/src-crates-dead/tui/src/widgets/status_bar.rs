use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{mode::Mode, theme::Theme};

pub fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    connected: bool,
    balance: f64,
    active_panel: &str,
    mode: &Mode,
) {
    let daemon_indicator = if connected { "●" } else { "○" };
    let daemon_color = if connected { theme.success } else { theme.error };

    let line = Line::from(vec![
        Span::styled(
            format!(" [{}] ", mode),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Bonsai TUI", Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
        Span::styled(" | panel: ", Style::default().fg(theme.muted)),
        Span::styled(active_panel, Style::default().fg(theme.fg)),
        Span::styled(" | daemon: ", Style::default().fg(theme.muted)),
        Span::styled(daemon_indicator, Style::default().fg(daemon_color)),
        Span::styled(" | wallet: ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{:.2} cr ", balance),
            Style::default().fg(theme.accent),
        ),
    ]);

    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg));
    frame.render_widget(paragraph, area);
}
