use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use crate::app::App;

pub enum LoopControl {
    Continue,
    Quit,
}

pub fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<LoopControl> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') if is_press_like(key.kind) => {
            return Ok(LoopControl::Quit)
        }
        KeyCode::Tab if is_press_like(key.kind) => {
            app.focus = app.focus.next();
        }
        KeyCode::Char(' ') => match key.kind {
            KeyEventKind::Press => {
                // Some terminals do not emit key-release; allow press-to-toggle fallback.
                let action = if app.recording_active() {
                    app.stop_push_to_talk_and_process()
                } else {
                    app.start_push_to_talk()
                };
                if let Err(err) = action {
                    app.push_talk(format!("ERROR: {}", err));
                }
            }
            KeyEventKind::Repeat => {}
            KeyEventKind::Release => {
                if app.recording_active() {
                    if let Err(err) = app.stop_push_to_talk_and_process() {
                        app.push_talk(format!("ERROR: {}", err));
                    }
                }
            }
        },
        KeyCode::Char('t') | KeyCode::Char('T') if is_press_like(key.kind) => {
            if let Err(err) = app.trigger_single_shot(5) {
                app.push_talk(format!("ERROR: {}", err));
            }
        }
        KeyCode::Char('p') | KeyCode::Char('P') if is_press_like(key.kind) => {
            app.profile.rewrite_mode = app.profile.rewrite_mode.next();
            app.save_profile()?;
            app.push_runtime(format!(
                "Rewrite mode set to {}",
                app.profile.rewrite_mode.label()
            ));
        }
        KeyCode::Char('i') | KeyCode::Char('I') if is_press_like(key.kind) => {
            app.profile.inject_app = app.profile.inject_app.next();
            app.save_profile()?;
            app.push_runtime(format!(
                "Inject mode set to {}",
                app.profile.inject_app.label()
            ));
        }
        KeyCode::Char('r') | KeyCode::Char('R') if is_press_like(key.kind) => {
            app.reload_profile()?;
        }
        KeyCode::Char('v') | KeyCode::Char('V') if is_press_like(key.kind) => {
            match app.voxtral.validate() {
                Ok(_) => app.push_runtime("Voxtral validation OK"),
                Err(err) => app.push_runtime(format!("Voxtral validation failed: {}", err)),
            }
        }
        KeyCode::Char('g') | KeyCode::Char('G') if is_press_like(key.kind) => {
            if let Err(err) = app.toggle_global_ptt() {
                app.push_runtime(format!("Global PTT toggle failed: {}", err));
            }
        }
        KeyCode::Char('k') | KeyCode::Char('K') if is_press_like(key.kind) => {
            if let Err(err) = app.cycle_ptt_hotkey() {
                app.push_runtime(format!("PTT hotkey update failed: {}", err));
            }
        }
        KeyCode::Char('c') | KeyCode::Char('C') if is_press_like(key.kind) => {
            if let Err(err) = app.command_mode_rewrite_selected() {
                app.push_talk(format!("ERROR: {}", err));
            }
        }
        _ => {}
    }
    Ok(LoopControl::Continue)
}

fn is_press_like(kind: KeyEventKind) -> bool {
    matches!(kind, KeyEventKind::Press | KeyEventKind::Repeat)
}
