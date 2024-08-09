use std::fmt::Debug;

use chrono::NaiveDate;
use ratatui::{
    style::{Color, Style},
    text::{Span, Text},
    widgets::Block,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub item_complete_mark: String,
    pub item_incomplete_mark: String,
    pub item_priority_mark_format: String,
    pub item_no_priority_mark: String,
    pub item_context_style: Style,
    pub item_project_style: Style,
    pub item_due_style: Style,
    pub item_t_style: Style,
}

impl Config {
    pub fn default_block(&self) -> Block {
        Block::bordered()
    }

    pub fn item_complete_mark(&self) -> Text {
        Span::from(&self.item_complete_mark).into()
    }

    pub fn item_incomplete_mark(&self) -> Text {
        Span::from(&self.item_incomplete_mark).into()
    }

    pub fn item_priority_mark(&self, prio: char) -> Text {
        Span::from(
            self.item_priority_mark_format
                .replacen("{p}", &prio.to_string(), 1),
        )
        .into()
    }

    pub fn item_no_priority_mark(&self) -> Text {
        Span::from(&self.item_no_priority_mark).into()
    }

    pub fn item_context<'a>(&'a self, context: &'a str) -> Span<'a> {
        Span::styled(context, self.item_context_style)
    }

    pub fn item_project<'a>(&'a self, project: &'a str) -> Span<'a> {
        Span::styled(project, self.item_project_style)
    }

    pub fn item_due_date(&self, date: NaiveDate) -> Span {
        Span::styled(date.format("%d.%m.%Y").to_string(), self.item_due_style)
    }

    pub fn item_t_date(&self, date: NaiveDate) -> Span {
        Span::styled(date.format("%d.%m.%Y").to_string(), self.item_t_style)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            item_complete_mark: "[x]".to_owned(),
            item_incomplete_mark: "[ ]".to_owned(),
            item_priority_mark_format: "({p})".to_owned(),
            item_no_priority_mark: "".to_owned(),
            item_context_style: Style::new().fg(Color::Green),
            item_project_style: Style::new().fg(Color::Cyan),
            item_due_style: Style::new().fg(Color::Red),
            item_t_style: Style::new().fg(Color::Blue),
        }
    }
}
