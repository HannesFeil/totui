use std::path::PathBuf;

use ratatui::widgets::TableState;

use crate::{
    config::Config,
    todo::{TodoItem, TodoList},
};

/// Application.
#[derive(Debug)]
pub struct App {
    /// Configuration
    pub config: Config,
    /// Archive path
    pub archive_path: Option<PathBuf>,
    /// Is the application running?
    pub running: bool,
    /// Sorted TodoList
    pub todo_list: SortedFilteredTodoList,
    /// Application state
    pub state: State,
}

#[derive(Debug)]
pub struct SortedFilteredTodoList {
    list: TodoList,
    sort_filter: SortFilter,
    view_indices: Box<[usize]>,
}

impl SortedFilteredTodoList {
    pub fn new(list: TodoList) -> Self {
        let sort_filter = SortFilter::default();
        let view_indices = sort_filter.apply(&list);

        Self {
            list,
            sort_filter,
            view_indices,
        }
    }

    pub fn items(&self) -> Box<[&TodoItem]> {
        self.view_indices
            .iter()
            .copied()
            .map(|i| &self.list[i])
            .collect()
    }
}

#[derive(Debug, Default)]
pub struct SortFilter {}

impl SortFilter {
    pub fn apply(&self, list: &TodoList) -> Box<[usize]> {
        list.iter().enumerate().map(|(i, _)| i).collect()
    }
}

#[derive(Debug, Default)]
pub struct State {
    pub todo_table_state: TableState,
    pub focus_state: FocusState,
}

#[derive(Debug, Default)]
pub enum FocusState {
    #[default]
    None,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(todo_list: TodoList, archive_path: Option<PathBuf>, config: Config) -> Self {
        Self {
            config,
            archive_path,
            running: true,
            todo_list: SortedFilteredTodoList::new(todo_list),
            state: State::default(),
        }
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
