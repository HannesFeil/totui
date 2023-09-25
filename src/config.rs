use std::{fmt::Display, hint::black_box};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    style::{Color, Style},
    widgets::BorderType,
};
use serde::{Deserialize, Serialize};

/// Application config
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Wrap around when browsing
    pub wrap_around: bool,
    /// Show help at the bottom
    pub show_help: bool,
    /// Whether to ignore case when filtering
    pub ignore_filter_case: bool,
    /// Add creation date to new tasks
    pub add_creation_date: bool,
    /// Popup completion size
    pub completion_size: (u16, u16),
    /// Keybind config
    pub keys: Keys,
    /// Visual config
    pub styles: Styles,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wrap_around: true,
            show_help: true,
            ignore_filter_case: true,
            add_creation_date: true,
            completion_size: (20, 8),
            keys: Keys::default(),
            styles: Styles::default(),
        }
    }
}

impl Config {
    /// Try reading the config from the default path
    ///
    /// If no config exists or the config found can not be parsed,
    /// a new one is created
    ///
    /// # Panics
    /// If the default config can not be written
    pub fn read() -> Config {
        let sp = standard_paths::default_paths!();
        let mut path = sp
            .writable_location(standard_paths::LocationType::AppConfigLocation)
            .expect("Expect available config directory");
        path.set_file_name("totui");
        path.set_extension("toml");

        let mut config = if path.exists() {
            match toml::from_str::<Config>(
                &std::fs::read_to_string(path).expect("Expect config read to succeed"),
            ) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Error parsing config, continuing with default config");
                    eprintln!("{e}");

                    Config::default()
                }
            }
        } else {
            println!("Config not found, creating default config at {path:?}");
            let config = Config::default();

            // TODO: disabled while developing to register changes
            black_box(&config);
            //
            // std::fs::write(
            //     path,
            //     toml::to_string_pretty(&config).expect("Config should always be serializable"),
            // )
            // .expect("Should be able to write config");

            config
        };

        config.post_read();

        config
    }

    fn post_read(&mut self) {
        macro_rules! patch_with_default {
            ($( $( $style:ident).+ ),* ,) => {
                $(
                    self.$( $style ).* = self.styles.default_style.patch(self.$( $style ).*);
                 )*
            };
        }

        // Patch styles
        patch_with_default! {
            styles.border_style,
            styles.help_style,
            styles.selection_symbol_style,
            styles.item.context_style,
            styles.item.project_style,
            styles.item.complete_style,
            styles.item.ignore_complete_style,
            styles.item.incomplete_style,
            styles.item.no_priority_style,
            styles.item.ignore_priority_style,
            styles.item.default_priority_style,
        }
    }
}

macro_rules! declare_keys {
    ($name:ident { $( $( #[$at:meta] )* ($($bind:tt)+) -> $bind_name:ident )* }) => {
        /// Key bindings config
        #[derive(Serialize, Deserialize)]
        pub struct $name {
            $( $( #[$at] )* pub $bind_name: KeyBind),*
        }

        impl Default for $name {
            fn default() -> $name {
                $name {
                    $($bind_name: declare_keys!(@key_bind $($bind)+)),*
                }
            }
        }
    };
    (@key_bind $c:literal) => {
        KeyBind::new(KeyCode::Char($c))
    };
    (@key_bind $b:ident) => {
        KeyBind::new(KeyCode::$b)
    };
    (@key_bind $( $m:ident )|* + $c:literal) => {
        KeyBind::new_with_modifiers(KeyCode::Char($c), $( KeyModifiers::$m )|*)
    };
    (@key_bind $( $m:ident )|* + $b:ident) => {
        KeyBind::new_with_modifiers(KeyCode::$b, $( KeyModifiers::$m )|*)
    };
}

declare_keys! {
    Keys {
        /// `KeyBind` for quitting the application
        ('q') -> quit
        /// `KeyBind` for moving up
        ('k') -> up
        /// `KeyBind` for moving down
        ('j') -> down
        /// `KeyBind` for commiting
        (Enter) -> confirm
        /// `KeyBind` for cancelling
        (Esc) -> cancel
        /// `KeyBind` for entering filter typing
        ('f') -> filter
        /// `KeyBind` for clearing the filter
        ('F') -> clear_filter
        /// `KeyBind` for toggling completion of an item
        ('d') -> toggle_done
        /// `KeyBind` for editing the priority of an item
        ('p') -> edit_priority
        /// `KeyBind` for toggling ignore case when filtering
        (CONTROL + 'c') -> input_toggle_ignore_case
        /// `KeyBind` for toggling completion while typing (or filtering)
        (CONTROL + 'd') -> input_toggle_done
        /// `KeyBind` for editing the priority while typing (or filtering)
        (CONTROL + 'p') -> input_edit_priority
        /// `KeyBind` for editing an item
        ('e') -> edit
        /// `KeyBind` for adding an item
        ('a') -> add
        /// `KeyBind` for moving down auto completion
        (Tab) -> completion_next
        /// `KeyBind` for finishing auto completion
        (Enter) -> completion_finish
    }
}

#[derive(Serialize, Deserialize)]
pub struct KeyBind {
    /// The key code
    key: KeyCode,
    /// Additional modifiers
    modifiers: KeyModifiers,
}

impl KeyBind {
    /// Create a new KeyBind with the given `KeyCode` and no modifiers
    pub fn new(key: KeyCode) -> KeyBind {
        if let KeyCode::Char(c @ 'A'..='Z') = key {
            Self {
                key: KeyCode::Char(c),
                modifiers: KeyModifiers::SHIFT,
            }
        } else {
            Self {
                key,
                modifiers: KeyModifiers::empty(),
            }
        }
    }

    /// Create a new KeyBind with the given `KeyCode` and modifiers
    pub fn new_with_modifiers(key: KeyCode, modifiers: KeyModifiers) -> KeyBind {
        if let KeyCode::Char(c @ 'A'..='Z') = key {
            Self {
                key: KeyCode::Char(c),
                modifiers: modifiers | KeyModifiers::SHIFT,
            }
        } else {
            Self { key, modifiers }
        }
    }

    /// Return if this `KeyBind` should apply on the given `KeyEvent`
    pub fn applies(&self, event: &KeyEvent) -> bool {
        if let KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press | KeyEventKind::Repeat,
            ..
        } = event
        {
            *code == self.key && *modifiers == self.modifiers
        } else {
            false
        }
    }
}

impl Display for KeyBind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let modifiers: Vec<_> = self.modifiers.iter_names().map(|(name, _)| name).collect();

        let code = match self.key {
            KeyCode::Backspace => "BS".to_owned(),
            KeyCode::Enter => "Enter".to_owned(),
            KeyCode::Left => "Left".to_owned(),
            KeyCode::Right => "Right".to_owned(),
            KeyCode::Up => "Up".to_owned(),
            KeyCode::Down => "Down".to_owned(),
            KeyCode::Home => "Home".to_owned(),
            KeyCode::End => "End".to_owned(),
            KeyCode::PageUp => "PUp".to_owned(),
            KeyCode::PageDown => "PDown".to_owned(),
            KeyCode::Tab => "Tab".to_owned(),
            KeyCode::BackTab => "BTab".to_owned(),
            KeyCode::Delete => "Delete".to_owned(),
            KeyCode::Insert => "Insert".to_owned(),
            KeyCode::F(num) => format!("F{num}"),
            KeyCode::Char(c) => format!("{c}"),
            KeyCode::Null => "Null".to_owned(),
            KeyCode::Esc => "Esc".to_owned(),
            KeyCode::CapsLock => "CapsLock".to_owned(),
            KeyCode::ScrollLock => "ScrollLock".to_owned(),
            KeyCode::NumLock => "NumLock".to_owned(),
            KeyCode::PrintScreen => "Print".to_owned(),
            KeyCode::Pause => "Pause".to_owned(),
            KeyCode::Menu => "Menu".to_owned(),
            KeyCode::KeypadBegin => "KeypadBegin".to_owned(),
            _ => unreachable!(),
        };

        write!(
            f,
            "{mods}{space}{code}",
            mods = modifiers.join(" + "),
            space = if modifiers.is_empty() { "" } else { " + " }
        )
    }
}

/// Visuals config
#[derive(Serialize, Deserialize)]
pub struct Styles {
    /// Item styles
    pub item: Item,
    /// The default `Style` for text
    pub default_style: Style,
    /// The [BorderType] for each border
    pub border_type: BorderTypeConfig,
    /// The `Style` for each border
    pub border_style: Style,
    /// The symbol to seperate help hints on the left
    pub left_help_seperator_symbol: String,
    /// The symbol to seperate help hints on the right
    pub right_help_seperator_symbol: String,
    /// The `Style` for help hints
    pub help_style: Style,
    /// The symbol used to mark the selected item
    pub selection_symbol: String,
    /// The `Style` of the selection symbol
    pub selection_symbol_style: Style,
    /// The symbol used to signal ignored case
    pub ignore_case_symbol: String,
    /// The `Style` of the ignored case symbol
    pub ignore_case_style: Style,
    /// The symbol used to signal sensitive case
    pub sensitive_case_symbol: String,
    /// The `Style` of the sensitive case symbol
    pub sensitive_case_style: Style,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            item: Default::default(),
            default_style: Style::new(),
            border_type: Default::default(),
            border_style: Style::new().fg(Color::Gray),
            left_help_seperator_symbol: " ".to_owned(),
            right_help_seperator_symbol: "".to_owned(),
            help_style: Style::new().fg(Color::Gray).bg(Color::DarkGray),
            selection_symbol: "🞂 ".to_owned(),
            selection_symbol_style: Style::new().fg(Color::White),
            ignore_case_symbol: "  ".to_owned(),
            ignore_case_style: Style::new().fg(Color::DarkGray),
            sensitive_case_symbol: "  ".to_owned(),
            sensitive_case_style: Style::new().fg(Color::White),
        }
    }
}

/// Item visuals config
#[derive(Serialize, Deserialize)]
pub struct Item {
    /// The default `Style` for contexts
    pub context_style: Style,
    /// The default `Style` for projects
    pub project_style: Style,
    /// The `Style` for the complete completion symbol
    pub incomplete_style: Style,
    /// The symbol for incomplete items
    pub incomplete_symbol: String,
    /// The `Style` for the ignored completion symbol
    pub ignore_complete_style: Style,
    /// The symbol for ignored completion filter
    pub ignore_complete_symbol: String,
    /// The `Style` for the completed completion symbol
    pub complete_style: Style,
    /// The symbol for completed items
    pub complete_symbol: String,
    /// The `Style` for items without priority
    pub no_priority_style: Style,
    /// The symbol for items without priority
    pub no_priority_symbol: String,
    /// The `Style` for ignored priority symbol
    pub ignore_priority_style: Style,
    /// The symbol for ignored priority filter
    pub ignore_priority_symbol: String,
    /// The default `Style` for items without priority
    pub default_priority_style: Style,
    /// The default symbol for items without priority, %p will be replaced be the priority
    pub default_priority_symbol: String,
}

impl Default for Item {
    fn default() -> Self {
        Self {
            context_style: Style::new().fg(Color::LightGreen),
            project_style: Style::new().fg(Color::LightCyan),
            incomplete_style: Style::new().fg(Color::Gray),
            incomplete_symbol: "[ ] ".to_owned(),
            ignore_complete_style: Style::new().fg(Color::DarkGray),
            ignore_complete_symbol: "[*] ".to_owned(),
            complete_style: Style::new().fg(Color::LightGreen),
            complete_symbol: "[󰸞] ".to_owned(),
            no_priority_style: Style::new().fg(Color::Gray),
            no_priority_symbol: "( ) ".to_owned(),
            ignore_priority_style: Style::new().fg(Color::DarkGray),
            ignore_priority_symbol: "(*) ".to_owned(),
            default_priority_style: Style::new().fg(Color::Red),
            default_priority_symbol: "(%a) ".to_owned(),
        }
    }
}

/// Helper struct for parsing [BorderType]
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub enum BorderTypeConfig {
    #[default]
    Plain,
    Rounded,
    Double,
    Thick,
}

impl From<BorderTypeConfig> for BorderType {
    fn from(value: BorderTypeConfig) -> Self {
        match value {
            BorderTypeConfig::Plain => BorderType::Plain,
            BorderTypeConfig::Rounded => BorderType::Rounded,
            BorderTypeConfig::Double => BorderType::Double,
            BorderTypeConfig::Thick => BorderType::Thick,
        }
    }
}
