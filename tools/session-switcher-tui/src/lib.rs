mod app;
mod asr;
mod audio;
mod config;
mod daemon;
mod event;
mod inject;
mod paths;
mod transform;
mod ui;
pub mod util;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self as ct_event, Event};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;
use event::{handle_key_event, LoopControl};

pub fn run() -> Result<()> {
    let mut app = App::new()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app);
    app.shutdown();

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

pub fn run_daemon() -> Result<()> {
    daemon::run_daemon()
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        app.drain_worker_events();

        terminal.draw(|frame| ui::draw(frame, app))?;

        if ct_event::poll(Duration::from_millis(50))? {
            match ct_event::read()? {
                Event::Key(key) => {
                    if let LoopControl::Quit = handle_key_event(app, key)? {
                        return Ok(());
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
}
