use std::path::PathBuf;

use ratatui::{
    layout::{Margin, Rect}, style::{Style, Stylize}, widgets::{Block, TableState, Widget}
};
use tui_input::Input;

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
    view_indices: Vec<usize>,
}

impl SortedFilteredTodoList {
    pub fn new(list: TodoList) -> Self {
        let sort_filter = SortFilter::default();
        let view_indices = Vec::with_capacity(list.len());

        let mut this = Self {
            list,
            sort_filter,
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
                .filter_map(|(i, item)| self.sort_filter.applies(item).then_some(i)),
        );
    }

    pub fn items(&self) -> impl Iterator<Item = &TodoItem> {
        self.view_indices.iter().copied().map(|i| &self.list[i])
    }

    pub fn sort_filter(&self) -> &SortFilter {
        &self.sort_filter
    }
}

#[derive(Debug, Default)]
pub struct SortFilter {
    pub filter: TodoListFilter,
    pub sort: TodoListSort,
}

#[derive(Debug, Default)]
pub struct TodoListFilter {
    pub input_field: Input,
    pub completion: Option<bool>,
    pub priority: Option<Option<char>>,
    pub t: bool,
}

#[derive(Debug, Default)]
pub struct TodoListSort {}

impl SortFilter {
    pub fn applies(&self, item: &TodoItem) -> bool {
        self.filter.applies(item)
    }
}

impl TodoListFilter {
    pub fn applies(&self, item: &TodoItem) -> bool {
        true
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

    pub fn cursor_pos(&self) -> Option<(u16, u16)> {
        None
    }
}
