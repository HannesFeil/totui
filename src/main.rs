use std::{io, path::PathBuf, process::exit, time::Duration};

use clap::Parser;
use config::Config;
use crossterm::{
    cursor::Show,
    event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::CrosstermBackend, widgets::ListState};

use crate::todo::TodoList;

mod config;
mod todo;
mod ui;

const MILLIS_PER_DRAW: u64 = 100;

#[derive(Parser, Debug)]
#[command(name = "Totui")]
#[command(author = "Hannes Feil")]
#[command(about = "A TUI for managing a todo.txt files")]
#[command(long_about = None)]
#[command(version)]
struct Args {
    #[arg(value_name = "File")]
    /// The todo.txt file
    file: PathBuf,
}

pub struct State {
    todo_list: TodoList,
    todo_list_state: ListState,
}

fn main() {
    // TODO: implement args
    let args = Args::parse();

    let config = Config::read();

    let todo_list: TodoList = match std::fs::read_to_string(args.file) {
        Ok(string) => match string.parse() {
            Ok(list) => list,
            Err(e) => {
                eprintln!("Error parsing todo file: {e}");
                exit(-1)
            }
        },
        Err(e) => {
            eprintln!("Error reading todo file: {e}");
            exit(-1)
        }
    };

    let mut terminal = enable_tui().unwrap_or_else(|e| {
        eprintln!("Failed to enable TUI");
        eprintln!("{e}");
        exit(-1)
    });

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        if let Err(e) = disable_tui() {
            eprintln!("An Error ocurred during panic disabling tui");
            eprintln!("{e}");
        };

        default_hook(panic_info)
    }));

    let state = State {
        todo_list,
        todo_list_state: ListState::default().with_selected(Some(1)),
    };

    if let Err(e) = run(&mut terminal, state, &config) {
        eprintln!("Error ocurred: {e}");
    };

    if let Err(e) = disable_tui() {
        eprintln!("Failed to disable TUI");
        eprintln!("{e}");
        exit(-1)
    }
}

type Terminal = ratatui::Terminal<CrosstermBackend<io::Stdout>>;

fn enable_tui() -> io::Result<Terminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn disable_tui() -> io::Result<()> {
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    io::stdout().execute(Show)?;

    Ok(())
}

#[allow(clippy::single_match)]
fn run(terminal: &mut Terminal, mut state: State, config: &Config) -> io::Result<()> {
    loop {
        if event::poll(Duration::from_millis(MILLIS_PER_DRAW))? {
            match event::read()? {
                event::Event::Key(KeyEvent {
                    code,
                    modifiers,
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) => match code {
                    KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => {
                        panic!("Program interrupted")
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        terminal.draw(|f| ui::draw(f, &mut state, config))?;
    }
}
