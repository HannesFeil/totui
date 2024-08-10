use std::fmt::Debug;

use chrono::NaiveDate;
use ratatui::{
    style::{Color, Style},
    text::{Span, Text},
    widgets::Block,
};
use serde::{Deserialize, Serialize};

macro_rules! config_struct {
    (
        $name:ident:
        $(
            $(
                #[$( $attr:tt )*]
            )*
            $vi:vis $item_name:ident: $item_type:ty $( = $item_default:expr )?
        ),+
        $( , )?
    ) => {
        #[derive(Debug, Deserialize, Serialize)]
        pub struct $name {
            $(
                $(
                    #[$( $attr )*]
                )*
                $vi $item_name: $item_type
            ),+
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $(
                        $item_name: config_struct!(@item_init $item_type $( = $item_default)?)
                    ),+
                }
            }
        }
    };
    (@item_init $item_type:ty) => {
        <$item_type>::default()
    };
    (@item_init $item_type:ty = $item_default:expr) => {
        $item_default
    };
}

config_struct! {
    Config:
    item_complete_mark: String = "[x]".to_owned(),
    item_incomplete_mark: String = "[ ]".to_owned(),
    item_priority_mark_format: String = "({p})".to_owned(),
    item_no_priority_mark: String = "".to_owned(),
    item_word_style: Style = Style::new(),
    item_space_style: Style = Style::new(),
    item_context_style: Style = Style::new().fg(Color::Green),
    item_project_style: Style = Style::new().fg(Color::Cyan),
    item_due_style: Style = Style::new().fg(Color::Red),
    item_t_style: Style = Style::new().fg(Color::Blue),
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

    pub fn item_word<'a>(&'a self, word: &'a str) -> Span<'a> {
        Span::styled(word, self.item_word_style)
    }

    pub fn item_space<'a>(&'a self, space: &'a str) -> Span<'a> {
        Span::styled(space, self.item_space_style)
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
