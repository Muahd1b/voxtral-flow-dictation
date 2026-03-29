use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, FocusPane};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let border_style = if app.focus == FocusPane::Middle {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let mut lines = Vec::new();
    if let Some(partial) = &app.live_partial {
        lines.push(format!("[live] {}", partial));
        lines.push(String::new());
    }

    if app.talk_logs.is_empty() {
        lines.push("No transcript events yet. Hold Space to talk.".to_string());
    } else {
        let max_lines = area.height.saturating_sub(3) as usize;
        let tail = app
            .talk_logs
            .iter()
            .rev()
            .take(max_lines)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        lines.extend(tail);
    }

    let text = lines.join("\n");
    let widget = Paragraph::new(text)
        .block(
            Block::default()
                .title("Middle: Live Composer")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    frame.render_widget(widget, area);
}
