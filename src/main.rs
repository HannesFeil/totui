use std::{
    fs::OpenOptions,
    io::{self, Write},
    ops::Range,
    path::PathBuf,
    process::exit,
    time::Duration,
};

use clap::{Parser, Subcommand};
use config::Config;
use crossterm::{
    cursor::Show,
    event::{self, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::CrosstermBackend, widgets::ListState};
use todo::{Filter, Priority, TodoItem};
use tui_input::{backend::crossterm::EventHandler, Input, StateChanged};
use widgets::{CalendarPicerState, RecurrencePickerState};

use crate::todo::TodoList;

mod config;
mod todo;
mod ui;
mod widgets;

/// Max millis per redraw
const MILLIS_PER_TICK: u64 = 100;
/// Max redraws per save
const TICKS_PER_SAVE: u32 = 1000;

/// Arguments to the program
#[derive(Parser, Debug)]
#[command(name = "Totui")]
#[command(author = "Hannes Feil")]
#[command(about = "A TUI for managing a todo.txt files")]
#[command(long_about = None)]
#[command(version)]
struct Args {
    /// Command
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
#[command()]
enum Command {
    /// Run the interactive applitcation
    Run {
        /// The todo.txt file
        #[arg(value_name = "File")]
        file: PathBuf,
        /// Config path
        #[arg(long)]
        config: Option<PathBuf>,
        /// Archive path
        #[arg(long)]
        archive: Option<PathBuf>,
    },
    /// Write the default config
    WriteDefaultConfig {
        /// Config path
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

/// Program state
pub struct State {
    /// Path to the file
    file_path: PathBuf,
    /// Path to the archive
    archive_path: PathBuf,
    /// Counter for saving
    save_counter: u32,
    /// The list of todo items
    todo_list: TodoList,
    /// The list state
    todo_list_view: ListState,
    /// The filter input
    filter: Input,
}

/// Current user focus state
#[derive(Clone, Debug)]
pub enum FocusState {
    /// Exiting the application
    Exiting,
    /// Browsing the list
    Browsing,
    /// Typing in the filter field
    FilterTyping {
        previous_filter: Filter,
        previous_filter_content: String,
        previous_list_view: ListState,
        popup: CompletionPopup,
    },
    /// Writing an item
    ItemTyping {
        item_index: Option<usize>,
        input: Input,
        item: TodoItem,
        popup: CompletionPopup,
    },
    /// Picking a priority
    PriorityPicking {
        previous_state: Box<FocusState>,
        view: ListState,
    },
    /// Picking a date
    DatePicking {
        previous_state: Box<FocusState>,
        calendar_view: CalendarPicerState,
        date: EditDate,
    },
    /// Picking a recurrence
    RecurrencePicking {
        previous_state: Box<FocusState>,
        picker_state: RecurrencePickerState,
    },
}

#[derive(Debug, Clone)]
pub enum EditDate {
    Due,
    Threshhold,
}

impl FocusState {
    /// Handle an incoming event
    pub fn handle_event(
        &mut self,
        event: &event::Event,
        state: &mut State,
        config: &Config,
    ) -> Option<FocusState> {
        match event {
            event::Event::FocusLost => {
                std::fs::write(&state.file_path, state.todo_list.to_string())
                    .expect("Expect write to succeed");
                None
            }
            e @ event::Event::Key(event) => match self {
                FocusState::FilterTyping { .. } => {
                    self.handle_filter_typing_event(e, event, state, config)
                }
                FocusState::Browsing => self.handle_browsing_event(event, state, config),
                FocusState::ItemTyping { .. } => {
                    self.handle_item_typing_event(e, event, state, config)
                }
                FocusState::PriorityPicking { .. } => {
                    self.handle_priority_picking_event(event, state, config)
                }
                FocusState::DatePicking { .. } => {
                    self.handle_date_picking_event(event, state, config)
                }
                FocusState::RecurrencePicking { .. } => {
                    self.handle_recurrence_picking_event(event, state, config)
                }
                FocusState::Exiting => unreachable!(),
            },
            _ => None,
        }
    }

    /// Handle `KeyEvent` while browsing
    fn handle_browsing_event(
        &mut self,
        event: &event::KeyEvent,
        state: &mut State,
        config: &Config,
    ) -> Option<FocusState> {
        // Filter count min 1
        let filter_count = state.todo_list.filter_count().max(1);

        if config.keys.quit.applies(event) {
            return Some(FocusState::Exiting);
        } else if config.keys.up.applies(event) {
            state
                .todo_list_view
                .select(state.todo_list_view.selected().map(|i| {
                    if config.general.wrap_around {
                        (i + filter_count - 1) % filter_count
                    } else {
                        i.saturating_sub(1)
                    }
                }));
        } else if config.keys.down.applies(event) {
            state
                .todo_list_view
                .select(state.todo_list_view.selected().map(|i| {
                    if config.general.wrap_around {
                        (i + 1) % filter_count
                    } else {
                        (i + 1).min(filter_count - 1)
                    }
                }));
        } else if config.keys.clear_filter.applies(event) {
            state.filter.reset();
            state
                .todo_list
                .mutate_filter(|f| *f = Filter::new(true, config.general.threshhold_days.into()));
        } else if config.keys.filter.applies(event) {
            return Some(FocusState::FilterTyping {
                previous_filter: state.todo_list.filter().clone(),
                previous_filter_content: state.filter.value().to_owned(),
                previous_list_view: std::mem::take(&mut state.todo_list_view),
                popup: CompletionPopup::default(),
            });
        } else if config.keys.toggle_done.applies(event) {
            let index = state
                .todo_list
                .iter_filtered()
                .nth(state.todo_list_view.selected().unwrap_or_default());
            if let Some((index, _)) = index {
                state.todo_list.mutate_then_update(|list| {
                    let new = list[index].toggle_completed();
                    if let Some(new) = new {
                        if !list.contains(&new) {
                            list.push(new);
                        }
                    }
                });
            }
        } else if config.keys.edit_priority.applies(event) && state.todo_list.filter_count() > 0 {
            let index = match state
                .todo_list
                .iter_filtered()
                .nth(state.todo_list_view.selected().unwrap())
                .unwrap()
                .1
                .priority
            {
                Some(p) => p.0 as usize - 'A' as usize + 1,
                None => 0,
            };

            return Some(FocusState::PriorityPicking {
                previous_state: Box::new(FocusState::Browsing),
                view: ListState::default().with_selected(Some(index)),
            });
        } else if config.keys.edit_threshhold.applies(event) && state.todo_list.filter_count() > 0 {
            return Some(FocusState::DatePicking {
                previous_state: Box::new(self.clone()),
                calendar_view: CalendarPicerState::new(
                    state
                        .todo_list
                        .iter_filtered()
                        .nth(state.todo_list_view.selected().unwrap())
                        .unwrap()
                        .1
                        .threshhold,
                ),
                date: EditDate::Threshhold,
            });
        } else if config.keys.edit_due.applies(event) && state.todo_list.filter_count() > 0 {
            return Some(FocusState::DatePicking {
                previous_state: Box::new(self.clone()),
                calendar_view: CalendarPicerState::new(
                    state
                        .todo_list
                        .iter_filtered()
                        .nth(state.todo_list_view.selected().unwrap())
                        .unwrap()
                        .1
                        .due,
                ),
                date: EditDate::Due,
            });
        } else if config.keys.edit_recurrence.applies(event) && state.todo_list.filter_count() > 0 {
            return Some(FocusState::RecurrencePicking {
                previous_state: Box::new(self.clone()),
                picker_state: RecurrencePickerState::new(
                    state
                        .todo_list
                        .iter_filtered()
                        .nth(state.todo_list_view.selected().unwrap())
                        .unwrap()
                        .1
                        .recurring,
                ),
            });
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
                    popup: Default::default(),
                });
            }
        } else if config.keys.add.applies(event) {
            return Some(FocusState::ItemTyping {
                item_index: None,
                input: Default::default(),
                item: TodoItem::new(config.general.add_creation_date),
                popup: Default::default(),
            });
        } else if config.keys.remove.applies(event) {
            let index = state
                .todo_list
                .iter_filtered()
                .nth(state.todo_list_view.selected().unwrap_or_default())
                .map(|(index, _)| index);

            if let Some(index) = index {
                state.todo_list.mutate_then_update(|l| {
                    let mut item = l.remove(index).to_string();
                    item.push('\n');
                    if config.general.archive_removed {
                        let mut archive = OpenOptions::new()
                            .append(true)
                            .create(true)
                            .open(&state.archive_path)
                            .expect("Expect archive to be openable");
                        archive
                            .write_all(item.as_bytes())
                            .expect("Expect archive to be writeable");
                        archive.flush().expect("Expect archive to be writeable");
                    }
                });
            }
        } else if config.keys.remove_completed.applies(event) {
            let mut items = vec![];
            state.todo_list.mutate_then_update(|l| {
                l.retain(|item| {
                    if !item.completed() {
                        true
                    } else {
                        items.push(item.to_string());
                        false
                    }
                })
            });
            let mut item_string = items.join("\n");
            item_string.push('\n');

            if config.general.archive_removed {
                let mut archive = OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&state.archive_path)
                    .expect("Expect archive to be openable");
                archive
                    .write_all(item_string.as_bytes())
                    .expect("Expect archive to be writeable");
                archive.flush().expect("Expect archive to be writeable");
            }
        }

        None
    }

    /// Handle `KeyEvent` while editing the filter
    fn handle_filter_typing_event(
        &mut self,
        e: &event::Event,
        event: &event::KeyEvent,
        state: &mut State,
        config: &Config,
    ) -> Option<FocusState> {
        let FocusState::FilterTyping {
            previous_filter,
            previous_filter_content,
            previous_list_view,
            popup,
        } = self
        else {
            unreachable!()
        };

        if config.keys.completion_next.applies(event) && popup.visible() {
            popup.next();
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
            state
                .todo_list
                .mutate_filter(|f| *f = std::mem::take(previous_filter));

            return Some(FocusState::Browsing);
        } else if config.keys.confirm.applies(event) {
            state.todo_list_view.select(Some(0));

            return Some(FocusState::Browsing);
        } else if config.keys.typing_toggle_ignore_case.applies(event) {
            state
                .todo_list
                .mutate_filter(|f| f.ignore_case = !f.ignore_case);
        } else if config.keys.typing_toggle_done.applies(event) {
            state.todo_list.mutate_filter(|f| {
                f.completed = match f.completed {
                    None => Some(true),
                    Some(true) => Some(false),
                    Some(false) => None,
                }
            });
        } else if config.keys.typing_edit_priority.applies(event) {
            match state.todo_list.filter().priority {
                Some(_) => state.todo_list.mutate_filter(|f| f.priority = None),
                None => {
                    return Some(FocusState::PriorityPicking {
                        previous_state: Box::new(self.clone()),
                        view: ListState::default().with_selected(Some(0)),
                    })
                }
            }
        } else if config.keys.typing_edit_threshhold.applies(event) {
            state.todo_list.mutate_filter(|f| {
                if f.hide_threshhold_days.is_some() {
                    f.hide_threshhold_days = None;
                } else {
                    f.hide_threshhold_days = Some(config.general.threshhold_days.into());
                }
            })
        } else if let Some(StateChanged { value: true, .. }) = state.filter.handle_event(e) {
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
                state.todo_list.contexts().chain(state.todo_list.projects()),
            );
        }

        None
    }

    /// Handle `KeyEvent` while editing an item
    fn handle_item_typing_event(
        &mut self,
        e: &event::Event,
        event: &event::KeyEvent,
        state: &mut State,
        config: &Config,
    ) -> Option<FocusState> {
        let FocusState::ItemTyping {
            item_index,
            input,
            item,
            popup,
        } = self
        else {
            unreachable!()
        };

        if config.keys.completion_next.applies(event) && popup.visible() {
            popup.next();
        } else if config.keys.completion_finish.applies(event)
            && popup.visible()
            && popup.state.selected().is_some()
        {
            popup.apply(input);
            item.set_content(input.value().to_owned());
        } else if config.keys.cancel.applies(event) {
            return Some(FocusState::Browsing);
        } else if config.keys.confirm.applies(event) && item.valid() {
            if let Some(index) = item_index {
                state.todo_list.mutate_then_update(|items| {
                    items[*index] = item.clone();
                });

                // Filter applies to edited item
                if let Some(index) = state.todo_list.iter_filtered().position(|(_, i)| i == item) {
                    state.todo_list_view.select(Some(index));
                }
            } else {
                state
                    .todo_list
                    .mutate_then_update(|items| items.push(std::mem::take(item)));
            }

            return Some(FocusState::Browsing);
        } else if config.keys.typing_toggle_done.applies(event) {
            item.toggle_completed();
        } else if config.keys.typing_edit_priority.applies(event) {
            let index = match item.priority {
                None => 0,
                Some(p) => p.0 as usize - 'A' as usize + 1,
            };

            return Some(FocusState::PriorityPicking {
                previous_state: Box::new(self.clone()),
                view: ListState::default().with_selected(Some(index)),
            });
        } else if config.keys.typing_edit_threshhold.applies(event) {
            let threshhold = item.threshhold;
            return Some(FocusState::DatePicking {
                previous_state: Box::new(self.clone()),
                calendar_view: CalendarPicerState::new(threshhold),
                date: EditDate::Threshhold,
            });
        } else if config.keys.typing_edit_due.applies(event) {
            let due = item.due;
            return Some(FocusState::DatePicking {
                previous_state: Box::new(self.clone()),
                calendar_view: CalendarPicerState::new(due),
                date: EditDate::Due,
            });
        } else if let Some(StateChanged { value: true, .. }) = input.handle_event(e) {
            item.set_content(input.value().to_owned());
            popup.update_options(
                input,
                state.todo_list.contexts().chain(state.todo_list.projects()),
            );
        }

        None
    }

    /// Handle `KeyEvent` while picking a priority
    fn handle_priority_picking_event(
        &mut self,
        event: &event::KeyEvent,
        state: &mut State,
        config: &Config,
    ) -> Option<FocusState> {
        let FocusState::PriorityPicking {
            previous_state,
            view,
        } = self
        else {
            unreachable!()
        };

        if config.keys.cancel.applies(event) {
            return Some(*previous_state.clone());
        } else if config.keys.confirm.applies(event) {
            let prio: Option<Priority> = match view.selected().unwrap() {
                0 => None,
                v @ 1..=27 => Some(
                    char::from_u32(('A' as usize + v - 1) as u32)
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
                _ => unreachable!(),
            };

            match &mut **previous_state {
                FocusState::Browsing => {
                    let index = state
                        .todo_list
                        .iter_filtered()
                        .nth(state.todo_list_view.selected().unwrap())
                        .unwrap()
                        .0;

                    state
                        .todo_list
                        .mutate_then_update(|list| list[index].priority = prio);
                    return Some(FocusState::Browsing);
                }
                FocusState::FilterTyping { .. } => {
                    state.todo_list.mutate_filter(|f| f.priority = Some(prio));

                    return Some(*previous_state.clone());
                }
                FocusState::ItemTyping { item, .. } => {
                    item.priority = prio;

                    return Some(*previous_state.clone());
                }
                _ => {
                    unreachable!()
                }
            }
        } else if config.keys.up.applies(event) {
            view.select(view.selected().map(|i| {
                if config.general.wrap_around {
                    (i + 26) % 27
                } else {
                    i.saturating_sub(1)
                }
            }));
        } else if config.keys.down.applies(event) {
            view.select(view.selected().map(|i| {
                if config.general.wrap_around {
                    (i + 1) % 27
                } else {
                    (i + 1).min(26)
                }
            }));
        } else if let KeyCode::Char(c @ 'A'..='Z') = event.code {
            view.select(Some(c as usize - 'A' as usize + 1));
        } else if let KeyCode::Char(c @ 'a'..='z') = event.code {
            view.select(Some(c as usize - 'a' as usize + 1));
        } else if let KeyCode::Char(' ') = event.code {
            view.select(Some(0));
        }

        None
    }

    /// Handle `KeyEvent` while picking a date
    fn handle_date_picking_event(
        &mut self,
        event: &event::KeyEvent,
        state: &mut State,
        config: &Config,
    ) -> Option<FocusState> {
        let FocusState::DatePicking {
            previous_state,
            calendar_view,
            date,
        } = self
        else {
            unreachable!()
        };

        if config.keys.cancel.applies(event) {
            return Some(*previous_state.clone());
        } else if config.keys.confirm.applies(event) {
            match &**previous_state {
                FocusState::Browsing => {
                    let index = state
                        .todo_list
                        .iter_filtered()
                        .nth(state.todo_list_view.selected().unwrap())
                        .unwrap()
                        .0;

                    match date {
                        EditDate::Due => state
                            .todo_list
                            .mutate_then_update(|l| l[index].due = calendar_view.locked()),
                        EditDate::Threshhold => state
                            .todo_list
                            .mutate_then_update(|l| l[index].threshhold = calendar_view.locked()),
                    };

                    return Some(FocusState::Browsing);
                }
                FocusState::ItemTyping {
                    item_index,
                    input,
                    item,
                    popup,
                } => {
                    let mut item = item.clone();
                    match date {
                        EditDate::Due => item.due = calendar_view.locked(),
                        EditDate::Threshhold => item.threshhold = calendar_view.locked(),
                    };
                    return Some(FocusState::ItemTyping {
                        item_index: *item_index,
                        input: input.clone(),
                        item,
                        popup: popup.clone(),
                    });
                }
                _ => unreachable!(),
            }
        } else if config.keys.up.applies(event) {
            calendar_view.select_previous_week();
        } else if config.keys.down.applies(event) {
            calendar_view.select_next_week();
        } else if config.keys.left.applies(event) {
            calendar_view.select_previous();
        } else if config.keys.right.applies(event) {
            calendar_view.select_next();
        } else if let (KeyCode::Char('S'), true) =
            (event.code, event.modifiers.contains(KeyModifiers::SHIFT))
        {
            calendar_view.select_locked();
        } else if let (KeyCode::Char('T'), true) =
            (event.code, event.modifiers.contains(KeyModifiers::SHIFT))
        {
            calendar_view.select_today();
        } else if let (KeyCode::Char('C'), true) =
            (event.code, event.modifiers.contains(KeyModifiers::SHIFT))
        {
            calendar_view.clear_locked();
        } else if let KeyCode::Char(' ') = event.code {
            calendar_view.lock_selected();
        }

        None
    }

    /// Handle `KeyEvent` while picking recurrence
    fn handle_recurrence_picking_event(
        &mut self,
        event: &event::KeyEvent,
        state: &mut State,
        config: &Config,
    ) -> Option<FocusState> {
        let FocusState::RecurrencePicking {
            previous_state,
            picker_state,
        } = self
        else {
            unreachable!()
        };

        if config.keys.cancel.applies(event) {
            return Some(*previous_state.clone());
        } else if config.keys.confirm.applies(event) {
            match &**previous_state {
                FocusState::Browsing => {
                    let index = state
                        .todo_list
                        .iter_filtered()
                        .nth(state.todo_list_view.selected().unwrap())
                        .unwrap()
                        .0;

                    state
                        .todo_list
                        .mutate_then_update(|l| l[index].recurring = picker_state.get_recurrence());

                    return Some(FocusState::Browsing);
                }
                FocusState::ItemTyping {
                    item_index,
                    input,
                    item,
                    popup,
                } => {
                    let mut item = item.clone();
                    item.recurring = picker_state.get_recurrence();

                    return Some(FocusState::ItemTyping {
                        item_index: *item_index,
                        input: input.clone(),
                        item,
                        popup: popup.clone(),
                    });
                }
                _ => unreachable!(),
            }
        } else if config.keys.up.applies(event) {
            picker_state.increase();
        } else if config.keys.down.applies(event) {
            picker_state.decrease();
        } else if config.keys.left.applies(event) {
            picker_state.select_previous();
        } else if config.keys.right.applies(event) {
            picker_state.select_next();
        } else if let (KeyCode::Char('C'), true) =
            (event.code, event.modifiers.contains(KeyModifiers::SHIFT))
        {
            picker_state.reset();
        }

        None
    }
}

#[derive(Clone, Debug, Default)]
pub struct CompletionPopup {
    options: Vec<String>,
    state: ListState,
}

impl CompletionPopup {
    /// If this popup should be shown
    pub fn visible(&self) -> bool {
        !self.options.is_empty()
    }

    /// Update the completion suggestions
    pub fn update_options<'a>(
        &mut self,
        input: &Input,
        total_options: impl Iterator<Item = &'a str>,
    ) {
        if input.value().is_empty() {
            return;
        }

        let text = &input.value()[CompletionPopup::text_range(input)];
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

    /// Apply the selected suggestion
    pub fn apply(&mut self, input: &mut Input) {
        if let Some(index) = self.state.selected() {
            let range = CompletionPopup::text_range(input);
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

    /// Get the range from the current word until cursor
    fn text_range(input: &Input) -> Range<usize> {
        let index = input
            .value()
            .char_indices()
            .nth(input.visual_cursor() - 1)
            .unwrap()
            .0;
        let start = input.value()[..index]
            .rfind(char::is_whitespace)
            .map_or(0, |i| i + 1);
        start..index
    }

    /// Select the next entry, wrapping around to no selection
    pub fn next(&mut self) {
        self.state
            .select(self.state.selected().map_or(Some(0), |i| {
                if i + 1 < self.options.len() {
                    Some(i + 1)
                } else {
                    None
                }
            }))
    }
}

/// Entry point
///
/// # Panics
/// - If the todo file can not be read
/// - If the terminal does not support TUI mode
fn main() {
    let args = Args::parse();

    match args.command {
        Command::Run {
            file,
            config,
            archive,
        } => {
            // Read config
            let config = Config::read(config);

            // Open todo file
            let mut todo_list: TodoList = match std::fs::read_to_string(&file) {
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

            todo_list
                .mutate_filter(|f| *f = Filter::new(true, config.general.threshhold_days.into()));

            let state = State {
                file_path: file.clone(),
                archive_path: archive
                    .unwrap_or(file.with_file_name("archive").with_extension("txt")),
                save_counter: 0,
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
        Command::WriteDefaultConfig { config } => {
            let path = Config::default().write_default(config);

            println!("Default config written to {path:?}");
        }
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
fn run(
    terminal: &mut Terminal,
    mut focus_state: FocusState,
    mut state: State,
    config: &Config,
) -> io::Result<()> {
    loop {
        if event::poll(Duration::from_millis(MILLIS_PER_TICK))? {
            if let Some(fs) = focus_state.handle_event(&event::read()?, &mut state, config) {
                focus_state = fs;
            }

            if let FocusState::Exiting = focus_state {
                std::fs::write(&state.file_path, state.todo_list.to_string())
                    .expect("Expect write to succeed");
                break Ok(());
            }
        }

        if state.save_counter == 0 {
            std::fs::write(&state.file_path, state.todo_list.to_string())
                .expect("Expect write to succeed");
        } else {
            state.save_counter = (state.save_counter + 1) % TICKS_PER_SAVE;
        }

        // Display the application
        terminal.draw(|f| ui::draw(f, &mut focus_state, &mut state, config))?;
    }
}
