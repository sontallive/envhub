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

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), CoreError> {
    // Check for version/help flags
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-v" => {
                println!("envhub {}", VERSION);
                return Ok(());
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown option: {}", args[1]);
                eprintln!("Run 'envhub --help' for usage information.");
                return Err(CoreError::new(
                    envhub_core::ErrorCode::InvalidState,
                    "Invalid argument".to_string(),
                ));
            }
        }
    }

    run_tui().map_err(|err| CoreError::new(envhub_core::ErrorCode::Io, err.to_string()))
}

fn print_help() {
    println!("envhub {}", VERSION);
    println!();
    println!("ABOUT:");
    println!("  A terminal user interface (TUI) for managing environment variables across");
    println!("  different applications and profiles.");
    println!();
    println!("USAGE:");
    println!("  envhub [OPTIONS]");
    println!();
    println!("  If no options are provided, the interactive TUI will launch.");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help       Show this help message");
    println!("  -v, --version    Show version information");
    println!();
    println!("KEYBOARD SHORTCUTS (in TUI):");
    println!("  q                Quit");
    println!("  a                Add app (on Apps List) / Add env var (on Env Vars)");
    println!("  p                Add profile (on App Detail)");
    println!("  i                Install shim for selected app");
    println!("  e                Edit selected environment variable");
    println!("  d                Delete selected environment variable");
    println!("  r                Reload configuration");
    println!("  Enter            Enter app detail / Activate profile");
    println!("  Esc              Go back / Cancel");
    println!("  Tab              Switch focus between Profiles and Env Vars");
    println!("  Up/Down          Navigate lists");
    println!();
    println!("For more information: https://github.com/sontallive/envhub");
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
