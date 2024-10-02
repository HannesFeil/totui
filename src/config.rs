use std::fmt::Debug;

use chrono::NaiveDate;
use crokey::{key, KeyCombination};
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
    ui: UI,
    pub keys: Keys,
}

config_struct! {
    Keys:
    pub up: KeyCombination = key!(k),
    pub down: KeyCombination = key!(j),
    pub left: KeyCombination = key!(h),
    pub right: KeyCombination = key!(l),
    pub confirm: KeyCombination = key!(Enter),
    pub cancel: KeyCombination = key!(Esc),
    pub quit: KeyCombination = key!(q),
    pub focus_filter: KeyCombination = key!('/'),
    pub focus_sort: KeyCombination = key!(s),
    pub priority: KeyCombination = key!(ctrl-p),
    pub completion: KeyCombination = key!(ctrl-d),
    pub t: KeyCombination = key!(ctrl-t),
}

config_struct! {
    UI:
    // -- Item --
    item_selection_mark: String = "> ".to_owned(),
    item_complete_mark: String = "[x]".to_owned(),
    item_incomplete_mark: String = "[ ]".to_owned(),
    item_priority_mark_format: String = "({p})".to_owned(),
    item_no_priority_mark: String = "".to_owned(),
    // -- Filter --
    filter_completion_disabled: String = "[*]".to_owned(),
    filter_priority_disabled: String = "(*)".to_owned(),
    filter_t_enabled: String = "t".to_owned(),
    filter_t_disabled: String = "t".to_owned(),
    /// Styles
    styles: Styles,
}

config_struct! {
    Styles:
    // -- General --
    border: Style = Style::new().gray(),
    // -- Item --
    item_word: Style,
    item_space: Style,
    item_context: Style = Style::new().green().bold(),
    item_project: Style = Style::new().cyan().bold(),
    item_due: Style = Style::new().red(),
    item_t: Style = Style::new().blue(),
    // -- Filter --
    filter_disabled: Style = Style::new().gray(),
}

impl Config {
    pub fn default_block(&self) -> Block {
        Block::bordered().border_style(self.ui.styles.border)
    }

    pub fn item_selection_mark(&self) -> Span {
        Span::from(&self.ui.item_selection_mark)
    }

    pub fn item_complete_mark(&self) -> Span {
        Span::from(&self.ui.item_complete_mark)
    }

    pub fn item_incomplete_mark(&self) -> Span {
        Span::from(&self.ui.item_incomplete_mark)
    }

    pub fn filter_completion_disabled(&self) -> Span {
        Span::styled(
            &self.ui.filter_completion_disabled,
            self.ui.styles.filter_disabled,
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
            self.ui.item_priority_mark_format
                .replacen("{p}", &prio.to_string(), 1),
        )
    }

    pub fn item_no_priority_mark(&self) -> Span {
        Span::from(&self.ui.item_no_priority_mark)
    }

    pub fn filter_priority_disabled(&self) -> Span {
        Span::styled(
            &self.ui.filter_priority_disabled,
            self.ui.styles.filter_disabled,
        )
    }

    pub fn priority_width(&self) -> usize {
        self.item_priority_mark('A')
            .width()
            .max(self.item_no_priority_mark().width())
            .max(self.filter_priority_disabled().width())
    }

    pub fn item_word<'a>(&'a self, word: &'a str) -> Span<'a> {
        Span::styled(word, self.ui.styles.item_word)
    }

    pub fn item_space<'a>(&'a self, space: &'a str) -> Span<'a> {
        Span::styled(space, self.ui.styles.item_space)
    }

    pub fn item_context<'a>(&'a self, context: &'a str) -> Span<'a> {
        Span::styled(context, self.ui.styles.item_context)
    }

    pub fn item_project<'a>(&'a self, project: &'a str) -> Span<'a> {
        Span::styled(project, self.ui.styles.item_project)
    }

    pub fn item_due_date(&self, date: NaiveDate) -> Span {
        Span::styled(
            date.format("%d.%m.%Y").to_string(),
            self.ui.styles.item_due,
        )
    }

    pub fn item_t_date(&self, date: NaiveDate) -> Span {
        Span::styled(
            date.format("%d.%m.%Y").to_string(),
            self.ui.styles.item_t,
        )
    }

    pub fn filter_t_enabled(&self) -> Span {
        Span::styled(&self.ui.filter_t_enabled, self.ui.styles.item_t)
    }

    pub fn filter_t_disabled(&self) -> Span {
        Span::styled(&self.ui.filter_t_disabled, self.ui.styles.filter_disabled)
    }

    pub fn t_width(&self) -> usize {
        self.filter_t_enabled()
            .width()
            .max(self.filter_t_disabled().width())
    }
}
