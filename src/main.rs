use clap::Parser;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use totui::todo::TodoList;
use std::io;
use std::path::PathBuf;
use totui::app::App;
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

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config: Config = match &args.config_file {
        Some(file) => confy::load_path(file)?,
        None => confy::load(env!("CARGO_PKG_NAME"), "config")?,
    };

    let todo_file_content = std::fs::read_to_string(&args.todo_file)?;
    let todo_list = TodoList::parse(&todo_file_content).or_else(|e| {
        anyhow::bail!("Failed to parse TODO file!\n{e}")
    })?;

    // Create an application.
    let mut app = App::new(todo_list, args.archive_file, config);

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
