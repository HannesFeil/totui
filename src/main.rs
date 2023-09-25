use std::{io, ops::Range, path::PathBuf, process::exit, time::Duration};

use clap::Parser;
use config::Config;
use crossterm::{
    cursor::Show,
    event::{self},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::CrosstermBackend, widgets::ListState};
use todo::{Filter, TodoItem};
use tui_input::{backend::crossterm::EventHandler, Input, StateChanged};

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
    /// The list of todo items
    todo_list: TodoList,
    /// The list state
    todo_list_view: ListState,
    /// The filter input
    filter: Input,
}

/// Current user focus state
pub enum FocusState {
    /// Browsing the list
    Browsing,
    /// Typing in the filter field
    FilterTyping {
        previous_filter: Filter,
        previous_filter_content: String,
        previous_list_view: ListState,
        popup: Popup,
    },
    /// Writing an item
    ItemTyping {
        item_index: Option<usize>,
        input: Input,
        item: TodoItem,
    },
    PriorityPicking {
        previous_state: Box<FocusState>,
        view: ListState,
    }
}

#[allow(clippy::single_match)]
impl FocusState {
    pub fn handle_event(
        &mut self,
        event: &event::Event,
        state: &mut State,
        config: &Config,
        exit: &mut bool,
    ) -> Option<FocusState> {
        match event {
            e @ event::Event::Key(event) => {
                match self {
                    FocusState::FilterTyping {
                        previous_filter,
                        previous_filter_content,
                        previous_list_view,
                        popup,
                    } => {
                        if config.keys.completion_next.applies(event) && popup.visible() {
                            popup
                                .state
                                .select(popup.state.selected().map_or(Some(0), |i| {
                                    if i + 1 < popup.options.len() {
                                        Some(i + 1)
                                    } else {
                                        None
                                    }
                                }))
                        } else if config.keys.completion_finish.applies(event)
                            && popup.visible()
                            && popup.state.selected().is_some()
                        {
                            popup.apply(&mut state.filter);
                            state.todo_list.mutate_filter(|f| {
                                f.words = state
                                    .filter
                                    .value()
                                    .split_whitespace()
                                    .map(str::to_owned)
                                    .collect()
                            });
                        } else if config.keys.cancel.applies(event) {
                            state.filter = Input::new(std::mem::take(previous_filter_content));
                            state.todo_list_view = std::mem::take(previous_list_view);
                            state.todo_list.mutate_filter(|f| {
                                f.words = state
                                    .filter
                                    .value()
                                    .split_whitespace()
                                    .map(str::to_owned)
                                    .collect()
                            });
                            state.todo_list.mutate_filter(|f| *f = std::mem::take(previous_filter));

                            return Some(FocusState::Browsing);
                        } else if config.keys.confirm.applies(event) {
                            state.todo_list_view.select(Some(0));

                            return Some(FocusState::Browsing);
                        } else if config.keys.input_toggle_ignore_case.applies(event) {
                            state
                                .todo_list
                                .mutate_filter(|f| f.ignore_case = !f.ignore_case);
                        } else if config.keys.input_toggle_done.applies(event) {
                            state.todo_list.mutate_filter(|f| {
                                f.completed = match f.completed {
                                    None => Some(true),
                                    Some(true) => Some(false),
                                    Some(false) => None,
                                }
                            });
                        } else if config.keys.input_edit_priority.applies(event) {
                            // FIXME: fix me
                        } else {
                            match state.filter.handle_event(e) {
                                Some(StateChanged { value: true, .. }) => {
                                    state.todo_list.mutate_filter(|f| {
                                        f.words = state
                                            .filter
                                            .value()
                                            .split_whitespace()
                                            .map(str::to_owned)
                                            .collect()
                                    });

                                    popup.update_options(
                                        &state.filter,
                                        state
                                            .todo_list
                                            .contexts()
                                            .chain(state.todo_list.projects()),
                                    );
                                }
                                _ => (),
                            }
                        }
                    }
                    FocusState::Browsing => {
                        // Filter count min 1
                        let filter_count = state.todo_list.filter_count().max(1);

                        if config.keys.quit.applies(event) {
                            // TODO: write on quit and quit
                            *exit = true;
                        } else if config.keys.up.applies(event) {
                            state
                                .todo_list_view
                                .select(state.todo_list_view.selected().map(|i| {
                                    if config.wrap_around {
                                        (i + filter_count - 1) % filter_count
                                    } else {
                                        i.saturating_sub(1)
                                    }
                                }));
                        } else if config.keys.down.applies(event) {
                            state
                                .todo_list_view
                                .select(state.todo_list_view.selected().map(|i| {
                                    if config.wrap_around {
                                        (i + 1) % filter_count
                                    } else {
                                        (i + 1).min(filter_count)
                                    }
                                }));
                        } else if config.keys.clear_filter.applies(event) {
                            state.filter.reset();
                            state.todo_list.mutate_filter(|f| *f = Default::default());
                        } else if config.keys.filter.applies(event) {
                            return Some(FocusState::FilterTyping {
                                previous_filter: state.todo_list.filter().clone(),
                                previous_filter_content: state.filter.value().to_owned(),
                                previous_list_view: std::mem::take(&mut state.todo_list_view),
                                popup: Popup::default(),
                            });
                        } else if config.keys.toggle_done.applies(event) {
                            let index = state
                                .todo_list
                                .iter_filtered()
                                .nth(state.todo_list_view.selected().unwrap_or_default());
                            if let Some((index, _)) = index {
                                state
                                    .todo_list
                                    .mutate_then_update(|list| list[index].toggle_completed());
                            }
                        } else if config.keys.edit.applies(event) {
                            if let Some((index, selected_item)) = state
                                .todo_list
                                .iter_filtered()
                                .nth(state.todo_list_view.selected().unwrap_or_default())
                            {
                                let content = selected_item.content().to_owned();

                                return Some(FocusState::ItemTyping {
                                    item_index: Some(index),
                                    input: Input::new(content),
                                    item: selected_item.clone(),
                                });
                            }
                        } else if config.keys.add.applies(event) {
                            return Some(FocusState::ItemTyping {
                                item_index: None,
                                input: Default::default(),
                                item: TodoItem::new(config.add_creation_date),
                            });
                        }
                    }
                    FocusState::ItemTyping {
                        item_index,
                        input,
                        item,
                    } => {
                        if config.keys.cancel.applies(event) {
                            return Some(FocusState::Browsing);
                        } else if config.keys.confirm.applies(event) {
                            if let Some(index) = item_index {
                                state.todo_list.mutate_then_update(|items| {
                                    items[*index] = item.clone();
                                });

                                // Filter applies to edited item
                                if let Some(index) =
                                    state.todo_list.iter_filtered().position(|(_, i)| i == item)
                                {
                                    state.todo_list_view.select(Some(index));
                                }
                            } else {
                                state
                                    .todo_list
                                    .mutate_then_update(|items| items.push(std::mem::take(item)));
                            }

                            return Some(FocusState::Browsing);
                        } else {
                            input.handle_event(e);
                            item.set_content(input.value().to_owned());
                        }
                    }
                    Self::PriorityPicking { previous_state, view } => todo!(),
                }
            }
            _ => {}
        }
        None
    }
}

#[derive(Default)]
pub struct Popup {
    options: Vec<String>,
    state: ListState,
}

impl Popup {
    pub fn visible(&self) -> bool {
        !self.options.is_empty()
    }

    pub fn update_options<'a>(
        &mut self,
        input: &Input,
        total_options: impl Iterator<Item = &'a str>,
    ) {
        let text = &input.value()[Popup::text_range(input)];
        let selected = self.state.selected().map(|i| self.options[i].clone());
        self.options = total_options
            .filter(|o| !text.is_empty() && o.starts_with(text))
            .map(str::to_owned)
            .collect();
        if self.options.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(selected.map(|s| {
                self.options
                    .iter()
                    .position(|o| o == &s)
                    .unwrap_or_default()
            }));
        }
    }

    pub fn apply(&mut self, input: &mut Input) {
        if let Some(index) = self.state.selected() {
            let range = Popup::text_range(input);
            let selected = &self.options[index];
            let new_value = format!(
                "{}{}{}",
                &input.value()[..range.start],
                selected,
                &input.value()[range.end..]
            );
            let new_input = input
                .clone()
                .with_value(new_value)
                .with_cursor(range.start + selected.len());
            *input = new_input;
        }
        self.options.clear();
        self.state.select(None);
    }

    fn text_range(input: &Input) -> Range<usize> {
        let start = input.value()[..input.cursor()]
            .rfind(char::is_whitespace)
            .map_or(0, |i| i + 1);
        start..input.cursor()
    }
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
    let mut todo_list: TodoList = match std::fs::read_to_string(args.file) {
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

    todo_list.mutate_filter(|f| *f = Filter::new(config.ignore_filter_case));

    let state = State {
        todo_list,
        todo_list_view: ListState::default().with_selected(Some(0)),
        filter: Input::from(""),
    };

    let focus_state = FocusState::Browsing;

    // Run the application
    if let Err(e) = run(&mut terminal, focus_state, state, &config) {
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
fn run(
    terminal: &mut Terminal,
    mut focus_state: FocusState,
    mut state: State,
    config: &Config,
) -> io::Result<()> {
    loop {
        if event::poll(Duration::from_millis(MILLIS_PER_TICK))? {
            let mut exit = false;
            if let Some(fs) =
                focus_state.handle_event(&event::read()?, &mut state, config, &mut exit)
            {
                focus_state = fs;
            }

            if exit {
                break Ok(());
            }
        }

        // Display the application
        terminal.draw(|f| ui::draw(f, &mut focus_state, &mut state, config))?;
    }
}
