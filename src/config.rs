use ratatui::{
    style::{Color, Style},
    widgets::BorderType,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub keys: Keys,
    pub styles: Styles,
}

impl Config {
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

#[derive(Serialize, Deserialize, Default)]
pub struct Keys {}

#[derive(Serialize, Deserialize)]
pub struct Styles {
    pub border_type: BorderTypeConfig,
    pub border_style: Style,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            border_type: Default::default(),
            border_style: Style {
                fg: Some(Color::Gray),
                ..Default::default()
            },
        }
    }
}

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
