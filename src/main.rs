use clap::Parser;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::path::PathBuf;
use totui::app::{App, AppResult};
use totui::config::Config;
use totui::event::{Event, EventHandler};
use totui::handler::handle_key_events;
use totui::tui::Tui;

#[derive(clap::Parser, Debug)]
#[command(version, author, about, long_about = None)]
struct Args {
    #[arg()]
    todo_file: PathBuf,
    #[arg(long, short)]
    archive_file: Option<PathBuf>,
    #[arg(long, short)]
    config_file: Option<PathBuf>,
}

fn main() -> AppResult<()> {
    let args = Args::parse();
    let config: Config = match &args.config_file {
        Some(file) => confy::load_path(file)?,
        None => confy::load(env!("CARGO_PKG_NAME"), "config")?,
    };

    // Create an application.
    let mut app = App::new();

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);

    tui.init()?;

    // Start the main loop.
    while app.running {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app)?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
