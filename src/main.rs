use std::{io, path::PathBuf, process::exit, time::Duration};

use clap::Parser;
use config::Config;
use crossterm::{
    cursor::Show,
    event::{self, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::CrosstermBackend, widgets::ListState};
use todo::TodoItem;
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::todo::TodoList;

mod config;
mod todo;
mod ui;

const MILLIS_PER_TICK: u64 = 100;

/// Arguments to the program
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

/// Program state
pub struct State {
    /// Current user focus state
    focus_state: FocusState,
    /// The list of todo items
    todo_list: TodoList,
    /// The list state
    todo_list_view: ListState,
    /// The filter input
    filter: Input,
    /// Whether to ignore case while filtering
    ignore_case: bool,
}

/// Current user focus state
pub enum FocusState {
    /// Browsing the list
    Browsing,
    /// Typing in the filter field
    FilterTyping {
        previous_filter: String,
        previous_list_view: ListState,
    },
}

/// Entry point
///
/// # Panics
/// - If the todo file can not be read
/// - If the terminal does not support TUI mode
fn main() {
    // TODO: implement args
    // Parse arguments
    let args = Args::parse();

    // Read config
    let config = Config::read();

    // Open todo file
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

    // Enable TUI mode
    let mut terminal = enable_tui().unwrap_or_else(|e| {
        eprintln!("Failed to enable TUI");
        eprintln!("{e}");
        exit(-1)
    });

    // Catch panics to disable TUI mode
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        if let Err(e) = disable_tui() {
            eprintln!("An Error ocurred during panic disabling tui");
            eprintln!("{e}");
        };

        default_hook(panic_info)
    }));

    let state = State {
        focus_state: FocusState::Browsing,
        todo_list,
        todo_list_view: ListState::default().with_selected(Some(1)),
        filter: Input::from(""),
        ignore_case: true,
    };

    // Run the application
    if let Err(e) = run(&mut terminal, state, &config) {
        eprintln!("Error ocurred: {e}");
    };

    // Disable TUI mode
    if let Err(e) = disable_tui() {
        eprintln!("Failed to disable TUI");
        eprintln!("{e}");
        exit(-1)
    }
}

type Terminal = ratatui::Terminal<CrosstermBackend<io::Stdout>>;

/// Enable TUI mode
///
/// This means enabling ['raw mode'](enable_raw_mode) and switching to the alternate screen.
///
/// # Return
/// A newly constructed terminal from [io::stdout()]
fn enable_tui() -> io::Result<Terminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

/// Disable TUI mode
///
/// This means disabling ['raw mode'](disable_raw_mode) and returning from the alternate screen.
fn disable_tui() -> io::Result<()> {
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    io::stdout().execute(Show)?;

    Ok(())
}

/// Runs the application
#[allow(clippy::single_match)]
fn run(terminal: &mut Terminal, mut state: State, config: &Config) -> io::Result<()> {
    loop {
        let filtered: Box<[&TodoItem]> = match state.filter.value().trim() {
            "" => state.todo_list.iter().collect(),
            filter => state.todo_list.filtered(filter, state.ignore_case),
        };

        if event::poll(Duration::from_millis(MILLIS_PER_TICK))? {
            match &event::read()? {
                e @ event::Event::Key(event) => {
                    state.focus_state = match state.focus_state {
                        FocusState::FilterTyping {
                            previous_filter,
                            previous_list_view,
                        } => match event.code {
                            KeyCode::Esc if event.kind == KeyEventKind::Press => {
                                state.filter = Input::new(previous_filter);
                                state.todo_list_view = previous_list_view;

                                FocusState::Browsing
                            }
                            KeyCode::Enter if event.kind == KeyEventKind::Press => {
                                state.todo_list_view.select(Some(0));

                                FocusState::Browsing
                            }
                            _ => {
                                state.filter.handle_event(e);

                                FocusState::FilterTyping {
                                    previous_filter,
                                    previous_list_view,
                                }
                            }
                        },
                        FocusState::Browsing => {
                            if config.keys.quit.applies(event) {
                                // TODO: write on quit
                                break Ok(());
                            }
                            if config.keys.up.applies(event) {
                                state.todo_list_view.select(
                                    state
                                        .todo_list_view
                                        .selected()
                                        .map(|i| (i + std::cmp::max(1, filtered.len()) - 1) % std::cmp::max(1, filtered.len())),
                                )
                            }
                            if config.keys.down.applies(event) {
                                state.todo_list_view.select(
                                    state
                                        .todo_list_view
                                        .selected()
                                        .map(|i| (i + 1) % std::cmp::max(1, filtered.len())),
                                )
                            }

                            if config.keys.filter.applies(event) {
                                FocusState::FilterTyping {
                                    previous_filter: state.filter.value().to_owned(),
                                    previous_list_view: std::mem::take(&mut state.todo_list_view),
                                }
                            } else {
                                FocusState::Browsing
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        terminal.draw(|f| {
            ui::draw(
                f,
                &filtered,
                &mut state.todo_list_view,
                &state.focus_state,
                &state.filter,
                config,
            )
        })?;
    }
}
