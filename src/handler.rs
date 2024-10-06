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
        FocusState::FilterFocus {} => {
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
                })
            } else if key == app.config.keys.t {
                app.todo_list.mutate_filter(|f| {
                    f.t = !f.t;
                })
            } else if let Some(input) = input {
                app.todo_list.mutate_filter(|f| {
                    f.input_field.handle(input);
                });
            }

            FocusState::FilterFocus {}
        }
        FocusState::ListFocus => {
            if key == app.config.keys.quit {
                app.quit();
            } else if key == app.config.keys.focus_filter {
                return FocusState::FilterFocus {};
            }

            FocusState::ListFocus
        }
        FocusState::Invalid => unreachable!(),
    }
}
