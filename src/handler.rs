use crate::app::{App, FocusState, SortedFilteredTodoList};
use crokey::{key, KeyCombination};
use ratatui::crossterm::event::KeyEvent;
use tui_input::InputRequest;

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_event(
    key_event: KeyEvent,
    input: Option<InputRequest>,
    app: &mut App,
) -> anyhow::Result<()> {
    match app.key_combiner.transform(key_event) {
        Some(key) => {
            if let key!(ctrl - c) = key {
                app.quit();
                return Ok(());
            }

            let old_state = app.take_state();
            let new_app_state = handle_state(input, key, app, old_state);
            app.state = new_app_state;

            Ok(())
        }
        None => Ok(()),
    }
}

fn handle_state(
    input: Option<InputRequest>,
    key: KeyCombination,
    app: &mut App,
    state: FocusState,
) -> FocusState {
    match state {
        FocusState::FilterFocus {
            previous_selection_index,
            previous_selection_item,
        } => {
            let update_index = |todo_list: &mut SortedFilteredTodoList| {
                let index = todo_list
                    .items()
                    .enumerate()
                    .find(|(_, item)| *item == &previous_selection_item)
                    .map(|(index, _)| index)
                    .unwrap_or_default();
                todo_list.table_state_mut().select(Some(index));
            };
            if [app.config.keys.cancel, app.config.keys.confirm].contains(&key) {
                return FocusState::ListFocus;
            } else if key == app.config.keys.priority {
                // TODO
            } else if key == app.config.keys.completion {
                app.todo_list.mutate_filter(|f| {
                    f.completion = match f.completion {
                        None => Some(true),
                        Some(true) => Some(false),
                        Some(false) => None,
                    };
                });
                update_index(&mut app.todo_list);
            } else if key == app.config.keys.t {
                app.todo_list.mutate_filter(|f| {
                    f.t = !f.t;
                });
                update_index(&mut app.todo_list);
            } else if let Some(input) = input {
                app.todo_list.mutate_filter(|f| {
                    f.input_field.handle(input);
                });
                update_index(&mut app.todo_list);
            }

            FocusState::FilterFocus {
                previous_selection_index,
                previous_selection_item,
            }
        }
        FocusState::ListFocus => {
            if key == app.config.keys.quit {
                app.quit();
            } else if key == app.config.keys.focus_filter {
                let previous_selection_index = app
                    .todo_list
                    .table_state_mut()
                    .selected()
                    .expect("There should be one item selected");
                let previous_selection_item = app
                    .todo_list
                    .items()
                    .nth(previous_selection_index)
                    .unwrap()
                    .clone();
                return FocusState::FilterFocus {
                    previous_selection_index,
                    previous_selection_item,
                };
            } else if app.todo_list.items().len() > 0 {
                if key == app.config.keys.up {
                    let mut table_state = app.todo_list.table_state_mut();
                    let len = app.todo_list.items().len();
                    let selected = table_state.selected().map(|i| (i + len - 1) % len);
                    table_state.select(selected);
                } else if key == app.config.keys.down {
                    let mut table_state = app.todo_list.table_state_mut();
                    let len = app.todo_list.items().len();
                    let selected = table_state.selected().map(|i| (i + 1) % len);
                    table_state.select(selected);
                }
            }

            FocusState::ListFocus
        }
        FocusState::Invalid => unreachable!(),
    }
}
