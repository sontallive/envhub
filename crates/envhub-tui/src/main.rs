use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use envhub_core::CoreError;

mod app;
mod ui;

use app::App;

fn main() -> Result<(), CoreError> {
    run_tui().map_err(|err| CoreError::new(envhub_core::ErrorCode::Io, err.to_string()))
}

fn run_tui() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create the app logic
    let mut app = match App::load() {
        Ok(app) => app,
        Err(e) => {
            // Fallback if loading fails, just to show error or exit.
            // But App::load returns io::Error wrapped.
            // We should probably just let it bubble up, but maybe show a nice error screen?
            // For now, simpler to propagate.
            return Err(e);
        }
    };

    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        let timeout = Duration::from_millis(200);
        let waited = timeout.saturating_sub(last_tick.elapsed());
        if event::poll(waited)? {
            if let Event::Key(key) = event::read()? {
                // Global exit on Ctrl+C is handled in handle_key but standard convention is good too.
                // handle_key returns true if we should quit
                if app.handle_key(key)? {
                    break;
                }
            }
        }
        if last_tick.elapsed() >= timeout {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    Ok(())
}
