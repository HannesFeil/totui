use std::fmt;
use std::path::PathBuf;
use std::process::exit;
use std::{collections::HashMap, marker::Sized, str::FromStr};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::BorderType;
use toml_edit::{Document, Formatted, Item};
use toml_edit::{Table, Value};

/// Trait used to de/serialize config components from and to toml
trait ConfigSerialize: Sized {
    /// Deserializes Self from an `Item` or return a human readable error message
    fn from_item(item: Item) -> Result<Self, String>;

    /// Serialize Self to an `Item`, this should never fail
    fn to_item(&self) -> Item;
}

impl ConfigSerialize for bool {
    fn from_item(item: Item) -> Result<Self, String> {
        item.as_bool().ok_or("Expected a boolean value".to_owned())
    }

    fn to_item(&self) -> Item {
        Item::Value(Value::Boolean(Formatted::new(*self)))
    }
}

impl ConfigSerialize for u16 {
    fn from_item(item: Item) -> Result<Self, String> {
        if let Some(Ok(v)) = item.as_integer().map(|v| v.try_into()) {
            Ok(v)
        } else {
            Err("Expected integer value".to_owned())
        }
    }

    fn to_item(&self) -> Item {
        Item::Value(Value::Integer(Formatted::new((*self).into())))
    }
}

impl ConfigSerialize for String {
    fn from_item(item: Item) -> Result<Self, String> {
        item.as_str()
            .ok_or("Expected a string value".to_owned())
            .map(str::to_string)
    }

    fn to_item(&self) -> Item {
        Item::Value(Value::String(Formatted::new(self.clone())))
    }
}

impl ConfigSerialize for Color {
    fn from_item(item: Item) -> Result<Self, String> {
        match item.as_str() {
            Some(s) => s.parse().map_err(|_| "Error parsing color".to_owned()),
            None => Err("Expected string value".to_owned()),
        }
    }

    fn to_item(&self) -> Item {
        Item::Value(Value::String(Formatted::new(self.to_string())))
    }
}

impl ConfigSerialize for Style {
    fn from_item(item: Item) -> Result<Self, String> {
        match item {
            Item::Value(Value::InlineTable(t)) => {
                let mut this = Style::new();

                if let Some(fg) = t.get("fg") {
                    this = this.fg(Color::from_item(Item::Value(fg.clone()))?);
                }

                if let Some(bg) = t.get("bg") {
                    this = this.bg(Color::from_item(Item::Value(bg.clone()))?);
                }

                if let Some(underline_color) = t.get("ul") {
                    this = this
                        .underline_color(Color::from_item(Item::Value(underline_color.clone()))?);
                }

                if let Some(add_modifiers) = t.get("add_modifiers") {
                    match add_modifiers.as_array() {
                        Some(a) => {
                            for val in a {
                                match val {
                                    Value::String(s) => {
                                        this = this.add_modifier(
                                            Modifier::from_name(s.value()).ok_or(format!(
                                                "Expected valid modifier, got '{s}'",
                                                s = s.value()
                                            ))?,
                                        );
                                    }
                                    _ => {
                                        return Err(
                                            "Expected string value for 'add_modifiers' entry"
                                                .to_owned(),
                                        );
                                    }
                                }
                            }
                        }
                        None => return Err("Expected array value for 'add_modifiers'".to_owned()),
                    }
                }

                if let Some(sub_modifiers) = t.get("sub_modifiers") {
                    match sub_modifiers.as_array() {
                        Some(a) => {
                            for val in a {
                                match val {
                                    Value::String(s) => {
                                        this = this.remove_modifier(
                                            Modifier::from_name(&s.to_string())
                                                .ok_or("Expected valid modifier name".to_owned())?,
                                        );
                                    }
                                    _ => {
                                        return Err(
                                            "Expected string value for 'sub_modifiers' entry"
                                                .to_owned(),
                                        );
                                    }
                                }
                            }
                        }
                        None => return Err("Expected array value for 'sub_modifiers'".to_owned()),
                    }
                }

                Ok(this)
            }
            _ => Err("Expected inline table value".to_owned()),
        }
    }

    fn to_item(&self) -> Item {
        let mut table = Table::new();

        if let Some(fg) = self.fg {
            table.insert("fg", fg.to_item());
        }

        if let Some(bg) = self.bg {
            table.insert("bg", bg.to_item());
        }

        if let Some(underline_color) = self.underline_color {
            table.insert("ul", underline_color.to_item());
        }

        if !self.add_modifier.is_empty() {
            table.insert(
                "add_modifiers",
                Item::Value(Value::Array(
                    self.add_modifier
                        .iter_names()
                        .map(|(name, _)| Value::String(Formatted::new(name.to_owned())))
                        .collect(),
                )),
            );
        }

        if !self.sub_modifier.is_empty() {
            table.insert(
                "sub_modifiers",
                Item::Value(Value::Array(
                    self.sub_modifier
                        .iter_names()
                        .map(|(name, _)| Value::String(Formatted::new(name.to_owned())))
                        .collect(),
                )),
            );
        }

        Item::Value(Value::InlineTable(table.into_inline_table()))
    }
}

impl ConfigSerialize for BorderType {
    fn from_item(item: Item) -> Result<Self, String> {
        match item.as_str() {
            Some(v) => v
                .parse()
                .map_err(|_| "Error parsing border type".to_owned()),
            None => Err("Expected string value".to_owned()),
        }
    }

    fn to_item(&self) -> Item {
        Item::Value(Value::String(Formatted::new(self.to_string())))
    }
}

impl<T> ConfigSerialize for HashMap<String, T>
where
    T: ConfigSerialize,
{
    fn from_item(item: Item) -> Result<Self, String> {
        match item {
            Item::Table(t) => {
                let mut this = Self::new();

                for (key, value) in t {
                    this.insert(key.to_string(), T::from_item(value)?);
                }

                Ok(this)
            }
            _ => Err("Expected table value".to_owned()),
        }
    }

    fn to_item(&self) -> Item {
        let mut table = Table::new();

        for (key, value) in self.iter() {
            table.insert(key, value.to_item());
        }

        Item::Table(table)
    }
}

/// Struct containing all user configuration
#[derive(Default)]
pub struct Config {
    /// General configuration
    pub general: General,
    /// Keybind configuration
    pub keys: Keys,
    /// Display configuration
    pub display: Display,
}

impl Config {
    /// Return the default config path including file name and extension
    pub fn default_path() -> PathBuf {
        let sp = standard_paths::default_paths!();
        let mut path = sp
            .writable_location(standard_paths::LocationType::AppConfigLocation)
            .expect("Expect available config directory");
        path.set_file_name("totui");
        path.set_extension("toml");
        path
    }

    /// Read a config file or return a default config
    /// If `path` is `None`, tries to read from the [Self::default_path()]
    pub fn read(path: Option<PathBuf>) -> Config {
        let path = path.unwrap_or_else(Self::default_path);

        let mut config = if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(toml) => match Config::from_toml(&toml) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!("Error reading config: {e}");
                        exit(-1);
                    }
                },
                Err(e) => {
                    eprintln!("Config could not be read, continuing with default config");
                    eprintln!("{e}");

                    Config::default()
                }
            }
        } else {
            Config::default()
        };

        config.post_read();

        config
    }

    /// Writes the default config to the given `path`
    /// If `path` is `None`, writes to the [Self::default_path()]
    pub fn write_default(&self, path: Option<PathBuf>) -> PathBuf {
        let path = path.unwrap_or_else(Self::default_path);

        std::fs::create_dir_all(path.with_file_name("").with_extension(""))
            .expect("Expect valid path");
        std::fs::write(&path, self.to_toml()).expect("Expect config write to succeed");

        path
    }

    /// Serialize the configuration to a toml `String`
    fn to_toml(&self) -> String {
        let mut doc = Document::new();

        doc.insert("General", self.general.to_item());
        doc.insert("Keys", self.keys.to_item());
        doc.insert("Display", self.display.to_item());

        doc.to_string()
    }

    /// Try deserializing the configuration from a toml `str` or return a human readable error `String`
    fn from_toml(toml: &str) -> Result<Self, String> {
        let doc: Document = toml
            .parse()
            .map_err(|e| format!("Error while parsing config file: {e}"))?;

        let general = match doc.get("General") {
            Some(i) => General::from_item(i.clone())
                .map_err(|e| format!("Error while parsing 'General': {e}"))?,
            None => General::default(),
        };

        let keys = match doc.get("Keys") {
            Some(i) => Keys::from_item(i.clone())
                .map_err(|e| format!("Error while parsing 'Keys': {e}"))?,
            None => Keys::default(),
        };

        let display = match doc.get("Display") {
            Some(i) => Display::from_item(i.clone())
                .map_err(|e| format!("Error while parsing 'Display': {e}"))?,
            None => Display::default(),
        };

        if let Some(unknown) = doc
            .iter()
            .find_map(|(key, _)| (!["General", "Keys", "Display"].contains(&key)).then_some(key))
        {
            return Err(format!("Error unknown key '{unknown}'"));
        }

        Ok(Self {
            general,
            keys,
            display,
        })
    }

    /// Perform post read tweaking (e.g. patching styles)
    fn post_read(&mut self) {
        macro_rules! patch_with_default {
            ($( $style:ident),* $(,)?) => {
                $(
                    self.display.$style = self.display.default_style.patch(self.display.$style);
                 )*
            };
        }

        // Patch styles
        patch_with_default! {
            border_style,
            seperator_style,
            selection_style,
            help_style,
            ignore_case_style,
            sensitive_case_style,
            threshhold_style,
            due_style,
            recurrence_style,
            error_style,
            context_style,
            project_style,
            ignore_case_style,
            uncompleted_style,
            completed_style,
            ignore_priority_style,
            no_priority_style,
            default_priority_style,
        }

        self.display.default_style = Style::reset().patch(self.display.default_style);
    }

    /// Return text representation and style for the given context
    pub fn context_look<'a>(&'a self, context: &'a str) -> (&str, Style) {
        let content = &context[1..];
        let text = match self.display.specific_context_text.get(content) {
            Some(t) => t,
            None if self.display.hide_prefix_symbol => content,
            _ => context,
        };
        let style = match self.display.specific_context_styles.get(content) {
            Some(s) => *s,
            None => self.display.context_style,
        };

        (text, style)
    }

    /// Return text representation and style for the given project
    pub fn project_look<'a>(&'a self, project: &'a str) -> (&str, Style) {
        let content = &project[1..];
        let text = match self.display.specific_project_text.get(content) {
            Some(t) => t,
            None if self.display.hide_prefix_symbol => content,
            _ => project,
        };
        let style = match self.display.specific_project_styles.get(content) {
            Some(s) => *s,
            None => self.display.project_style,
        };

        (text, style)
    }

    /// Return text representation and style for the given priority
    pub fn priority_look(&self, priority: &str) -> (String, Style) {
        let text = self
            .display
            .specific_priority_text
            .get(priority)
            .cloned()
            .unwrap_or(self.display.default_priority_text.replace("%p", priority));
        let style = *self
            .display
            .specific_priority_styles
            .get(priority)
            .unwrap_or(&self.display.default_priority_style);

        (text, style)
    }
}

/// Easily create configuration value structs with default values, which de/serialize from and to
/// toml. Documentation comments are kept and used as documentation in the serialized toml.
///
/// # Example
/// The following macro invocation
/// ```
/// config_struct! {
///     /// Top documentation
///     Test {
///         /// Number field documentation
///         pub number: u16 = 42,
///         /// String field documentation
///         pub string: String = "test".to_owned(),
///     }
/// }
/// ```
/// results in the following generated code
/// ```
/// /// Top documentation
/// pub struct Test {
///     /// Number field documentation
///     pub number: u16,
///     /// String field documentation
///     pub string: String,
/// }
///
/// impl Default for Test {
///     fn default() -> Test {
///         Test {
///             number: 42,
///             string: "test".to_owned()
///         }
///     }
/// }
///
/// // impl ConfigSerialize for Test { ... }
/// ```
/// and the following toml serialization
/// ```toml
/// # Top documentation
/// [table_name]
/// # Number field documentation
/// number = 42
/// # String field documentation
/// string = "test"
/// ```
macro_rules! config_struct {
    (
        $(#[doc = $top_comment:literal])*
        $name:ident {
            $(
                $(#[doc = $comment:literal])*
                $(#[name = $toml_name:literal])?
                $visibility:vis $field_name:ident: $type:ty = $default:expr
            ),*
            $(,)?
        }
    ) => {
        $(#[doc = $top_comment])*
        pub struct $name {
            $(
                $(#[doc = $comment])*
                $visibility $field_name: $type
             ),*
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $(
                        $field_name: $default
                     ),*
                }
            }
        }

        impl ConfigSerialize for $name {
            fn from_item(item: Item) -> Result<Self, String> {
                let table = item.as_table().ok_or("Expected a table")?;
                let mut this = Self::default();
                let mut totals = vec![];

                $(
                    {
                        let key = [$($toml_name,)? stringify!($field_name)].first().unwrap();
                        totals.push(*key);
                        if let Some(item) = table.get(key) {
                            let val = <$type as ConfigSerialize>::from_item(item.clone()).map_err(|e| format!("Error while parsing {key}: {e}"))?;
                            this.$field_name = val;
                        }
                    }
                 )*

                if let Some(key) = table.iter().map(|(key, _)| key).find(|key| !totals.contains(&key)) {
                    return Err(format!("Unknown key {key}"));
                }

                Ok(this)
            }

            fn to_item(&self) -> Item {
                let mut table = Table::new();

                $(
                    {
                        let key = [$($toml_name,)? stringify!($field_name)].first().unwrap();
                        let item = <$type as ConfigSerialize>::to_item(&self.$field_name);
                        table.insert(key, item);
                        match table.get_mut(key).unwrap() {
                            Item::Table(t) => t.decor_mut().set_prefix(["\n", $("#", $comment, "\n"),*].join("")),
                            Item::None => {}
                            _ => table.key_decor_mut(key).unwrap().set_prefix([$("#", $comment, "\n"),*].join("")),

                        }
                    }
                 )*

                table.decor_mut().set_prefix(<[&str]>::join(&["\n\n", $("#", $top_comment, "\n"),*], ""));

                Item::Table(table)
            }
        }
    };
}

config_struct! {
    /// General configuration
    General {
        /// List scrolling wraps
        pub wrap_around: bool = false,
        /// Display help at the bottom
        pub show_help: bool = true,
        /// Set the creation date when adding items
        pub add_creation_date: bool = true,
        /// Max completion window width
        pub completion_window_width: u16 = 20,
        /// Max completion window height
        pub completion_window_height: u16 = 10,
        /// Adds removed items to an archive
        pub archive_removed: bool = true,
    }
}

config_struct! {
    /// Keybind configuration
    /// A keybind takes the following form
    ///
    ///     [ modifer + .. + ] code
    ///
    /// where `modifier` is one of `Shift`, `Ctrl` and `Alt`
    /// and `key` is a character or a named key like `Enter`
    /// note that an uppercase character automatically implies the `Shift` modifier
    Keys {
        /// Quit the program
        pub quit: KeyBind = "q".parse().unwrap(),
        /// Move selection up
        pub up: KeyBind = "k".parse().unwrap(),
        /// Move selection down
        pub down: KeyBind = "j".parse().unwrap(),
        /// Move selection up
        pub left: KeyBind = "h".parse().unwrap(),
        /// Move selection down
        pub right: KeyBind = "l".parse().unwrap(),
        /// Confirm
        pub confirm: KeyBind = "Enter".parse().unwrap(),
        /// Cancel
        pub cancel: KeyBind = "Esc".parse().unwrap(),
        /// Start filtering
        pub filter: KeyBind = "f".parse().unwrap(),
        /// Clear filter
        pub clear_filter: KeyBind = "F".parse().unwrap(),
        /// Toggle completed
        pub toggle_done: KeyBind = "c".parse().unwrap(),
        /// Edit priority
        pub edit_priority: KeyBind = "p".parse().unwrap(),
        /// Edit threshhold
        pub edit_threshhold: KeyBind = "t".parse().unwrap(),
        /// Edit recurrence
        pub edit_recurrence: KeyBind = "r".parse().unwrap(),
        /// Edit due
        pub edit_due: KeyBind = "d".parse().unwrap(),
        /// Toggle ignore case while filtering
        pub typing_toggle_ignore_case: KeyBind = "Ctrl + a".parse().unwrap(),
        /// Toggle done while typing
        pub typing_toggle_done: KeyBind = "Ctrl + c".parse().unwrap(),
        /// Edit priority while typing
        pub typing_edit_priority: KeyBind = "Ctrl + p".parse().unwrap(),
        /// Edit threshhold while typing
        pub typing_edit_threshhold: KeyBind = "Ctrl + t".parse().unwrap(),
        /// Edit recurrence while typing
        pub typing_edit_recurrence: KeyBind = "Ctrl + r".parse().unwrap(),
        /// Edit due while typing
        pub typing_edit_due: KeyBind = "Ctrl + d".parse().unwrap(),
        /// Edit selected item
        pub edit: KeyBind = "e".parse().unwrap(),
        /// Add an item
        pub add: KeyBind = "a".parse().unwrap(),
        /// Remove an item
        pub remove: KeyBind = "x".parse().unwrap(),
        /// Remove completed items
        pub remove_completed: KeyBind = "X".parse().unwrap(),
        /// Select the next completion entry
        pub completion_next: KeyBind = "Tab".parse().unwrap(),
        /// Insert the selected completion entry
        pub completion_finish: KeyBind = "Enter".parse().unwrap(),
    }
}

config_struct! {
    /// Display configuration
    ///
    /// Styles are represented by inline tables with the following (optional) attributes
    /// - fg: the foreground color
    /// - bg: the background color
    /// - ul: the underline color
    /// - add_modifiers: A list of modifiers to add
    /// - remove_modifiers: A list of modifiers to remove
    /// Note: styles are patched together, so changing the default style affects most others
    ///
    /// Colors are represented by strings in the following ways
    /// - the name of the color including `reset` (e.g. `black` or `lightred`)
    /// - the index of the color in the ansi palette
    /// - A hex representation of a specific color (e.g. `#ff00ff`)
    ///
    /// the following are valid Modifier strings
    /// BOLD, DIM, ITALIC, UNDERLINED, SLOW_BLINK, RAPID_BLINK, REVERSED, HIDDEN, CROSSED_OUT
    Display {
        /// The date format see [chrono::format::strftime]
        pub date_format: String = "%d.%m.%Y".to_owned(),
        /// Hide the @ and + from contexts and projects
        pub hide_prefix_symbol: bool = true,

        /// The base style
        pub default_style: Style = Style::new(),

        /// The type of each border
        /// One of: Plain, Thick, Double, Rounded
        pub border_type: BorderType = BorderType::Plain,
        /// The style for each border
        pub border_style: Style = Style::new().fg(Color::Gray),

        /// The text seperating components in items
        pub seperator_text: String = " ".to_owned(),
        /// The style used for seperator text
        pub seperator_style: Style = Style::new(),

        /// The text in front of the selected item
        pub selection_text: String = "🞂 ".to_owned(),
        /// The style used for the selected item
        pub selection_style: Style = Style::new().bg(Color::DarkGray),

        /// The style used for the selected item
        pub help_style: Style = Style::new().bg(Color::DarkGray).fg(Color::Gray),

        /// The text used to signal ignored case
        pub ignore_case_text: String = " ".to_owned(),
        /// The style of the ignored case text
        pub ignore_case_style: Style = Style::new().fg(Color::DarkGray),
        /// The text used to signal sensitive case
        pub sensitive_case_text: String = " ".to_owned(),
        /// The style of the sensitive case text
        pub sensitive_case_style: Style = Style::new().fg(Color::White),

        /// The text for the threshhold date (%d will be replaced with the date)
        pub threshhold_text: String = "start: %d".to_owned(),
        /// The style for the threshhold date
        pub threshhold_style: Style = Style::new().fg(Color::Blue),

        /// The text for the due date (%d will be replaced with the date)
        pub due_text: String = "due: %d".to_owned(),
        /// The style for the due date
        pub due_style: Style = Style::new().fg(Color::Red),

        /// The text for recurrence (%r will be replaced with the recurrence)
        pub recurrence_text: String = "repeat: %r".to_owned(),
        /// The style for recurrence
        pub recurrence_style: Style = Style::new().fg(Color::Magenta),

        /// The error style for item editing
        pub error_style: Style = Style::new().fg(Color::Red).add_modifier(Modifier::REVERSED),
        /// The default style for contexts
        pub context_style: Style = Style::new().fg(Color::LightGreen),
        /// The default style for projects
        pub project_style: Style = Style::new().fg(Color::LightCyan),

        /// The text for ignoring completion status in the filter
        pub ignore_completed_text: String = "[*]".to_owned(),
        /// The style for the ignored completed text
        pub ignore_completed_style: Style = Style::new().fg(Color::DarkGray),
        /// The text for uncompleted items
        pub uncompleted_text: String = "[ ]".to_owned(),
        /// The style for uncompleted text
        pub uncompleted_style: Style = Style::new().fg(Color::Gray),
        /// The text for completed items
        pub completed_text: String = "[x]".to_owned(),
        /// The style for the completed completion text
        pub completed_style: Style = Style::new().fg(Color::Green),

        /// The text for ignored priority in the filter
        pub ignore_priority_text: String = "(*)".to_owned(),
        /// The style for ignore priority text
        pub ignore_priority_style: Style = Style::new().fg(Color::DarkGray),
        /// The text for items without priority
        pub no_priority_text: String = "".to_owned(),
        /// The style for no priority text
        pub no_priority_style: Style = Style::new(),

        /// The default text for items without priority, %p will be replaced with the priority
        pub default_priority_text: String = "(%p)".to_owned(),
        /// The default style for priority text
        pub default_priority_style: Style = Style::new().fg(Color::Yellow),

        /// Additional more specific context replacement text
        /// For example the following will replace @example tags with 'awesome context'
        /// example = "awesome context"
        specific_context_text: HashMap<String, String> = HashMap::new(),
        /// Additional more specific context styles
        /// For example the following will color @example tags purple
        /// example = { fg = "Magenta" }
        specific_context_styles: HashMap<String, Style> = HashMap::new(),
        /// Additional more specific project replacement text
        /// For example the following will replace +example tags with 'awesome project'
        /// example = "awesome project"
        specific_project_text: HashMap<String, String> = HashMap::new(),
        /// Additional more specific project styles
        /// For example the following will color +example tags purple
        /// example = { fg = "Magenta" }
        specific_project_styles: HashMap<String, Style> = HashMap::new(),
        /// Additional more specific priority text
        /// For example the following will replace (C) priority with `/!\`
        /// C = "/!\"
        specific_priority_text: HashMap<String, String> = HashMap::new(),
        /// Additional more specific priority styles
        /// For example the following will color (C) priority purple
        /// C = { fg = "Magenta" }
        specific_priority_styles: HashMap<String, Style> = HashMap::new(),
    }
}

pub struct KeyBind {
    /// The key code
    key: KeyCode,
    /// Additional modifiers
    modifiers: KeyModifiers,
}

impl KeyBind {
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

impl ConfigSerialize for KeyBind {
    fn from_item(item: Item) -> Result<Self, String> {
        match item.as_str() {
            Some(s) => s.parse(),
            None => Err("Expected string value".to_owned()),
        }
    }

    fn to_item(&self) -> Item {
        Item::Value(Value::String(Formatted::new(self.to_string())))
    }
}

impl fmt::Display for KeyBind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let modifiers: Vec<_> = self
            .modifiers
            .iter_names()
            .filter_map(|(name, modifier)| {
                if !matches!(self.key, KeyCode::Char('A'..='Z')) || modifier != KeyModifiers::SHIFT
                {
                    Some(match name {
                        "SHIFT" => "Shift",
                        "CONTROL" => "Ctrl",
                        "ALT" => "Alt",
                        _ => unreachable!("Should never contain any other modifiers"),
                    })
                } else {
                    None
                }
            })
            .collect();

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
            space = if modifiers.is_empty() { "" } else { " + " },
        )
    }
}

impl FromStr for KeyBind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split = s.split('+').map(|p| p.trim()).collect::<Box<[_]>>();
        let (modifiers, code) = match *split {
            [code] => (&[] as &[&str], code),
            [ref mods @ .., code] => (mods, code),
            _ => {
                return Err(
                    "Expected String in the form of '[modifier + modifier + ..] code".to_owned(),
                )
            }
        };

        let key = match code {
            "BS" => KeyCode::Backspace,
            "Enter" => KeyCode::Enter,
            "Left" => KeyCode::Left,
            "Right" => KeyCode::Right,
            "Up" => KeyCode::Up,
            "Down" => KeyCode::Down,
            "Home" => KeyCode::Home,
            "End" => KeyCode::End,
            "PUp" => KeyCode::PageUp,
            "PDown" => KeyCode::PageDown,
            "Tab" => KeyCode::Tab,
            "BTab" => KeyCode::BackTab,
            "Delete" => KeyCode::Delete,
            "Insert" => KeyCode::Insert,
            "Null" => KeyCode::Null,
            "Esc" => KeyCode::Esc,
            "CapsLock" => KeyCode::CapsLock,
            "ScrollLock" => KeyCode::ScrollLock,
            "NumLock" => KeyCode::NumLock,
            "Print" => KeyCode::PrintScreen,
            "Pause" => KeyCode::Pause,
            "Menu" => KeyCode::Menu,
            "KeypadBegin" => KeyCode::KeypadBegin,
            code if code.len() == 1 => KeyCode::Char(code.chars().next().unwrap()),
            code if code.starts_with('F') => KeyCode::F(
                code[1..]
                    .parse()
                    .map_err(|_| "Expected valid F key".to_owned())?,
            ),
            _ => return Err(format!("Unknown key code '{code}'")),
        };

        let mut modifiers = modifiers
            .iter()
            .map(|m| match *m {
                "Shift" => Ok(KeyModifiers::SHIFT),
                "Ctrl" => Ok(KeyModifiers::CONTROL),
                "Alt" => Ok(KeyModifiers::ALT),
                m => Err(format!("Unknown modifier '{m}'")),
            })
            .reduce(|m1, m2| match (m1, m2) {
                (Ok(m1), Ok(m2)) => Ok(m1 | m2),
                (Err(e), _) | (_, Err(e)) => Err(e),
            })
            .unwrap_or(Ok(KeyModifiers::empty()))?;

        if let KeyCode::Char('A'..='Z') = key {
            modifiers |= KeyModifiers::SHIFT;
        }

        Ok(Self { key, modifiers })
    }
}
