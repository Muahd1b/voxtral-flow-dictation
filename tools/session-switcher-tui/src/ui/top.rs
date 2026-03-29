use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, FocusPane};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let border_style = if app.focus == FocusPane::Top {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let recording = if app.recording_active() {
        format!("ON ({} ms)", app.recording_elapsed_ms())
    } else {
        "OFF".to_string()
    };
    let last_target = app
        .last_injected_app
        .clone()
        .unwrap_or_else(|| "<none>".to_string());

    let text = format!(
        "Model: voxtral | Voxtral procs: {} | Jobs: {}\nPTT: {} | Space start, release or next Space stops + inject | Single-shot: t\nGlobal PTT daemon: {} (toggle g) | Hotkey: {} (cycle k)\nRewrite: {} | Inject: {} | Mic device: {} | Lang: {}\nCmd mode: select text in focused app, press c to rewrite+replace\nLast target: {} | Keys: tab pane, p rewrite, c cmd-mode, i inject, g global, k hotkey, r reload, v validate, q quit",
        app.voxtral_instances(),
        app.jobs_inflight,
        recording,
        if app.global_ptt_running { "ON" } else { "OFF" },
        app.profile.ptt_hotkey,
        app.profile.rewrite_mode.label(),
        app.profile.inject_app.label(),
        app.profile.mic_device_index,
        app.profile.asr_language,
        last_target,
    );

    let widget = Paragraph::new(text)
        .block(
            Block::default()
                .title("Top: Control Panel")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(widget, area);
}
