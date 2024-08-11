use std::fmt::Debug;

use chrono::NaiveDate;
use ratatui::{
    style::{Style, Stylize},
    text::Span,
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
    filter_completion_disabled: String = "[*]".to_owned(),
    item_priority_mark_format: String = "({p})".to_owned(),
    item_no_priority_mark: String = "".to_owned(),
    filter_priority_disabled: String = "(*)".to_owned(),
    styles: Styles,
}

config_struct! {
    Styles:
    item_word_style: Style,
    item_space_style: Style,
    item_context_style: Style = Style::new().green(),
    item_project_style: Style = Style::new().cyan(),
    filter_disabled_style: Style = Style::new().gray(),
    item_due_style: Style = Style::new().red(),
    item_t_style: Style = Style::new().blue(),
}

impl Config {
    pub fn default_block(&self) -> Block {
        Block::bordered()
    }

    pub fn item_complete_mark(&self) -> Span {
        Span::from(&self.item_complete_mark)
    }

    pub fn item_incomplete_mark(&self) -> Span {
        Span::from(&self.item_incomplete_mark)
    }

    pub fn filter_completion_disabled(&self) -> Span {
        Span::styled(
            &self.filter_completion_disabled,
            self.styles.filter_disabled_style,
        )
    }

    pub fn completion_width(&self) -> usize {
        self.item_complete_mark()
            .width()
            .max(self.item_incomplete_mark().width())
            .max(self.filter_completion_disabled().width())
    }

    pub fn item_priority_mark(&self, prio: char) -> Span {
        Span::from(
            self.item_priority_mark_format
                .replacen("{p}", &prio.to_string(), 1),
        )
    }

    pub fn item_no_priority_mark(&self) -> Span {
        Span::from(&self.item_no_priority_mark)
    }

    pub fn filter_priority_disabled(&self) -> Span {
        Span::styled(
            &self.filter_priority_disabled,
            self.styles.filter_disabled_style,
        )
    }

    pub fn priority_width(&self) -> usize {
        self.item_priority_mark('A')
            .width()
            .max(self.item_no_priority_mark().width())
            .max(self.filter_priority_disabled().width())
    }

    pub fn item_word<'a>(&'a self, word: &'a str) -> Span<'a> {
        Span::styled(word, self.styles.item_word_style)
    }

    pub fn item_space<'a>(&'a self, space: &'a str) -> Span<'a> {
        Span::styled(space, self.styles.item_space_style)
    }

    pub fn item_context<'a>(&'a self, context: &'a str) -> Span<'a> {
        Span::styled(context, self.styles.item_context_style)
    }

    pub fn item_project<'a>(&'a self, project: &'a str) -> Span<'a> {
        Span::styled(project, self.styles.item_project_style)
    }

    pub fn item_due_date(&self, date: NaiveDate) -> Span {
        Span::styled(
            date.format("%d.%m.%Y").to_string(),
            self.styles.item_due_style,
        )
    }

    pub fn item_t_date(&self, date: NaiveDate) -> Span {
        Span::styled(
            date.format("%d.%m.%Y").to_string(),
            self.styles.item_t_style,
        )
    }
}
