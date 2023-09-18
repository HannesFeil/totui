use std::hint::black_box;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    style::{Color, Style},
    widgets::BorderType,
};
use serde::{Deserialize, Serialize};

/// Application config
#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Keybind config
    pub keys: Keys,
    /// Visual config
    pub styles: Styles,
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

        if path.exists() {
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
        }
    }
}

/// Key bindings config
#[derive(Serialize, Deserialize)]
pub struct Keys {
    /// `KeyBind` for quitting the application
    pub quit: KeyBind,
    /// `KeyBind` for moving up
    pub up: KeyBind,
    /// `KeyBind` for moving down
    pub down: KeyBind,
    /// `KeyBind` for entering filter typing
    pub filter: KeyBind,
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            quit: KeyBind::new(KeyCode::Char('q')),
            up: KeyBind::new(KeyCode::Char('k')),
            down: KeyBind::new(KeyCode::Char('j')),
            filter: KeyBind::new(KeyCode::Char('f')),
        }
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
        Self {
            key,
            modifiers: KeyModifiers::empty(),
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

/// Visuals config
#[derive(Serialize, Deserialize)]
pub struct Styles {
    /// The default style for text
    pub default_style: Style,
    /// The [BorderType] for each border
    pub border_type: BorderTypeConfig,
    /// The [Style] for each border
    pub border_style: Style,
    /// The symbol used to mark the selected item
    pub selection_symbol: String,
    /// The [Style] of the selection symbol
    pub selection_symbol_style: Style,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            default_style: Style {
                ..Default::default()
            },
            border_type: Default::default(),
            border_style: Style {
                fg: Some(Color::Gray),
                ..Default::default()
            },
            selection_symbol: "🞂 ".to_owned(),
            selection_symbol_style: Style {
                fg: Some(Color::White),
                ..Default::default()
            },
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
