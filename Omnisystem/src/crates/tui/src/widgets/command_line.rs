use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::theme::Theme;

pub fn render_command_line(frame: &mut Frame, area: Rect, theme: &Theme, buf: &str) {
    // Show as a popup at the bottom of the given area
    let popup_height = 3u16;
    let popup_y = area.y + area.height.saturating_sub(popup_height);
    let popup_area = Rect {
        x: area.x,
        y: popup_y,
        width: area.width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let line = Line::from(vec![
        Span::styled(":", Style::default().fg(theme.accent)),
        Span::styled(buf, Style::default().fg(theme.fg)),
        Span::styled("█", Style::default().fg(theme.accent)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" Command ")
        .title_style(Style::default().fg(theme.accent));

    let paragraph = Paragraph::new(line)
        .block(block)
        .style(Style::default().bg(theme.bg));

    // Use the inner area trick via layout
    let inner_layout = Layout::vertical([
        Constraint::Length(popup_height),
    ])
    .split(popup_area);

    frame.render_widget(paragraph, inner_layout[0]);
}
