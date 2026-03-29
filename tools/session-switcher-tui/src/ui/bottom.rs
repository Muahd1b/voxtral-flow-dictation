use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, FocusPane};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let border_style = if app.focus == FocusPane::Bottom {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let max_lines = area.height.saturating_sub(3) as usize;
    let lines = if app.runtime_logs.is_empty() {
        vec!["Runtime log is empty".to_string()]
    } else {
        app.runtime_logs
            .iter()
            .rev()
            .take(max_lines)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
    };

    let widget = Paragraph::new(lines.join("\n"))
        .block(
            Block::default()
                .title("Bottom: Runtime & Diagnostics")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    frame.render_widget(widget, area);
}
