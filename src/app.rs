use std::{
    cell::{RefCell, RefMut},
    path::PathBuf,
};

use chrono::Local;
use crokey::Combiner;
use ratatui::widgets::TableState;
use tui_input::Input;

use crate::{
    config::Config,
    todo::{Content, TodoItem, TodoList},
};

/// Application.
#[derive(Debug)]
pub struct App {
    pub key_combiner: Combiner,
    /// Configuration
    pub config: Config,
    /// Archive path
    pub archive_path: Option<PathBuf>,
    /// Is the application running?
    pub running: bool,
    /// Sorted TodoList
    pub todo_list: SortedFilteredTodoList,
    /// Application state
    pub state: FocusState,
}

#[derive(Debug)]
pub struct SortedFilteredTodoList {
    list: TodoList,
    list_table_state: RefCell<TableState>,
    filter: TodoListFilter,
    view_indices: Vec<usize>,
}

#[derive(Debug)]
pub struct TodoListFilter {
    pub input_field: Input,
    pub completion: Option<bool>,
    pub priority: Option<Option<char>>,
    pub t: bool,
}

#[derive(Debug, Default)]
pub struct TodoListSort {}

#[derive(Debug, Default)]
pub enum FocusState {
    SortFocus {},
    FilterFocus {},
    #[default]
    ListFocus,
    Invalid,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(todo_list: TodoList, archive_path: Option<PathBuf>, config: Config) -> Self {
        Self {
            key_combiner: Combiner::default(),
            config,
            archive_path,
            running: true,
            todo_list: SortedFilteredTodoList::new(todo_list),
            state: FocusState::default(),
        }
    }

    pub fn take_state(&mut self) -> FocusState {
        std::mem::replace(&mut self.state, FocusState::Invalid)
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}

impl Default for TodoListFilter {
    fn default() -> Self {
        Self {
            input_field: Input::new("".to_owned()),
            completion: None,
            priority: None,
            t: true,
        }
    }
}

impl TodoListFilter {
    pub fn applies(&self, item: &TodoItem) -> bool {
        if self
            .completion
            .is_some_and(|c| c != item.completion_date.is_some())
        {
            return false;
        }

        if self.priority.is_some_and(|p| p != item.priority) {
            return false;
        }

        if self.t {
            if let Some(t_date) = item.t {
                if Local::now().date_naive() < t_date {
                    return false;
                }
            }
        }

        if !self.input_field.value().is_empty() {
            let lower = self.input_field.value().to_lowercase();
            let words: Vec<_> = lower.split_whitespace().collect();
            let mut matched = false;

            for part in item.content_parts() {
                match &part.content {
                    Content::Word(text) | Content::Context(text) | Content::Project(text) => {
                        for word in &words {
                            if text.to_lowercase().contains(word) {
                                matched = true;
                                break;
                            }
                        }
                    }
                }
            }

            if !matched {
                return false;
            }
        }

        true
    }
}

impl SortedFilteredTodoList {
    pub fn new(list: TodoList) -> Self {
        let filter = TodoListFilter::default();
        let view_indices = Vec::with_capacity(list.len());

        let mut this = Self {
            list,
            filter,
            list_table_state: RefCell::new(TableState::new().with_selected(0)),
            view_indices,
        };
        this.update_view_indices();
        this
    }

    fn update_view_indices(&mut self) {
        self.view_indices.clear();
        self.view_indices.extend(
            self.list
                .iter()
                .enumerate()
                .filter_map(|(i, item)| self.filter.applies(item).then_some(i)),
        );
        self.view_indices.sort_by_key(|i| &self.list[*i]);
    }

    pub fn items(&self) -> impl Iterator<Item = &TodoItem> {
        self.view_indices.iter().copied().map(|i| &self.list[i])
    }

    pub fn filter(&self) -> &TodoListFilter {
        &self.filter
    }

    pub fn mutate_filter(&mut self, f: impl FnOnce(&mut TodoListFilter)) {
        f(&mut self.filter);
        self.update_view_indices();
    }

    pub fn table_state_mut(&self) -> RefMut<TableState> {
        self.list_table_state.borrow_mut()
    }
}
