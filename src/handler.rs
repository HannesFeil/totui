use crate::app::{App, FocusState};
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
            let new_app_state = handle_state(key_event, input, key, app, old_state);
            app.state = new_app_state;

            Ok(())
        }
        None => Ok(()),
    }
}

fn handle_state(
    key_event: KeyEvent,
    input: Option<InputRequest>,
    key: KeyCombination,
    app: &mut App,
    state: FocusState,
) -> FocusState {
    match state {
        FocusState::SortFocus {} => {
            if key == app.config.keys.cancel {
                return FocusState::ListFocus;
            }

            FocusState::SortFocus {}
        }
        FocusState::FilterFocus {} => {
            if key == app.config.keys.cancel {
                return FocusState::ListFocus;
            }

            if let Some(input) = input {
                app.todo_list.mutate_sort_filter(|sf| {
                    sf.filter.input_field.handle(input);
                });
            }

            FocusState::FilterFocus {}
        }
        FocusState::ListFocus => {
            if key == app.config.keys.quit {
                app.quit();
            } else if key == app.config.keys.focus_filter {
                return FocusState::FilterFocus {};
            } else if key == app.config.keys.focus_sort {
                return FocusState::SortFocus {};
            }

            FocusState::ListFocus
        }
        FocusState::Invalid => unreachable!(),
    }
}
