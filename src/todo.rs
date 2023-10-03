use std::{
    cmp::{self, Reverse},
    collections::HashSet,
    fmt::Display,
    num::ParseIntError,
    str::FromStr,
};

use chrono::{Days, Local, Months, NaiveDate};
use pest::Parser;
use pest_derive::Parser;

/// Pest parse for the 'todo.txt' format
#[derive(Parser)]
#[grammar = "./todo_grammar.pest"]
struct TodoListParser;

/// A list of [TodoItem]'s
#[derive(Debug, PartialEq, Eq)]
pub struct TodoList {
    /// Sorted `Vec` of `TodoItem`s
    items: Vec<TodoItem>,
    /// Current filter
    filter: Filter,
    /// Sorted view of filtered `TodoItem`s
    filtered: Vec<usize>,
    /// List of contexts found in `TodoItem`s
    contexts: HashSet<String>,
    /// List of projects found in `TodoItem`s
    projects: HashSet<String>,
}

impl TodoList {
    /// The currently applied filter
    pub fn filter(&self) -> &Filter {
        &self.filter
    }

    /// Mutate the filter and reapply it
    pub fn mutate_filter(&mut self, f: impl FnOnce(&mut Filter)) {
        f(&mut self.filter);
        self.apply_filter();
    }
    /// Update the filtered items
    /// TODO: better filtering (completed etc nicer ui)
    fn apply_filter(&mut self) {
        self.filtered.clear();
        let filter_words: Box<[_]> = if self.filter.ignore_case {
            self.filter.words.iter().map(|w| w.to_lowercase()).collect()
        } else {
            self.filter.words.iter().cloned().collect()
        };

        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                // Filter out priority
                if let Some(priority) = self.filter.priority {
                    if item.priority != priority {
                        return false;
                    }
                }

                // Filter out completed
                if let Some(completed) = self.filter.completed {
                    if item.completed != completed {
                        return false;
                    }
                }

                // Filter words
                let words = if self.filter.ignore_case {
                    item.content.to_lowercase()
                } else {
                    item.content.to_owned()
                };

                for word in filter_words.iter() {
                    if !words.contains(word) {
                        return false;
                    }
                }

                true
            })
            .for_each(|(index, _)| self.filtered.push(index));
    }

    /// The number of filtered `TodoItem`s
    pub fn filter_count(&self) -> usize {
        self.filtered.len()
    }

    /// Iter over the currently filtered `TodoItem`s and their indices
    pub fn iter_filtered(&self) -> impl Iterator<Item = (usize, &TodoItem)> {
        self.filtered.iter().map(|&i| (i, &self.items[i]))
    }

    /// Iter over all contexts
    pub fn contexts(&self) -> impl Iterator<Item = &str> {
        self.contexts.iter().map(String::as_str)
    }

    /// Iter over all projects
    pub fn projects(&self) -> impl Iterator<Item = &str> {
        self.projects.iter().map(String::as_str)
    }

    /// Mutate the list. Afterwards sorts the list and updates contexts and projects
    pub fn mutate_then_update(&mut self, f: impl FnOnce(&mut Vec<TodoItem>)) {
        f(&mut self.items);
        self.items.sort_unstable();
        self.contexts.clear();
        self.projects.clear();
        self.items.iter().for_each(|item| {
            self.contexts.extend(item.contexts().map(str::to_owned));
            self.projects.extend(item.projects().map(str::to_owned));
        });
        self.apply_filter();
    }
}

impl FromStr for TodoList {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pairs = TodoListParser::parse(Rule::main, s)
            .map_err(|e| format!("Parsing error in line {line}", line = e.line()))?;

        let mut items = vec![];

        // Parse lines
        for line in pairs.next().unwrap().into_inner() {
            let mut item = TodoItem::default();

            fn parse_date(s: &str, line: usize) -> Result<NaiveDate, String> {
                NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .map_err(|_| format!("Failed to parse date '{s}' in line {line}",))
            }

            // Parse initial marks
            for rule in line.into_inner() {
                match rule.as_rule() {
                    Rule::completed => item.completed = true,
                    Rule::priority => {
                        item.priority =
                            Some(Priority::try_from(rule.as_str().chars().nth(1).unwrap()).unwrap())
                    }
                    Rule::date => {
                        if item.completed && item.creation_date.is_some() {
                            item.completion_date = item.creation_date;
                            item.creation_date =
                                Some(parse_date(rule.as_str(), rule.line_col().0)?);
                        } else {
                            item.creation_date =
                                Some(parse_date(rule.as_str(), rule.line_col().0)?);
                        }
                    }
                    // Parse item content
                    Rule::content => {
                        item.content = rule.as_str().to_owned();
                        let mut content_start = rule.as_span().start();

                        for tag in rule
                            .into_inner()
                            .map(|tag| tag.into_inner().next().unwrap())
                        {
                            let tag_start = tag.as_span().start() - content_start;
                            let tag_end = tag.as_span().end() - content_start;

                            // Handle special case of tag being the first word
                            let tag_span = if tag_start == 0 {
                                tag_start..tag_end + 1
                            } else {
                                tag_start - 1..tag_end
                            };

                            match tag.as_rule() {
                                Rule::rec => {
                                    if item.recurring.is_some() {
                                        return Err(format!(
                                            "Second recurring tag found in line {line}",
                                            line = tag.line_col().0
                                        ));
                                    }
                                    item.content.replace_range(tag_span.clone(), "");
                                    content_start += tag_span.len();
                                    item.recurring =
                                        Some(tag.as_str()[4..].parse().map_err(|e| format!(
                                            "Unable to parse recurring amount in line {line} due to {e}",
                                            line = tag.line_col().0
                                        ))?);
                                }
                                Rule::due => {
                                    if item.due.is_some() {
                                        return Err(format!(
                                            "Second due tag found in line {line}",
                                            line = tag.line_col().0
                                        ));
                                    }
                                    item.content.replace_range(tag_span.clone(), "");
                                    content_start += tag_span.len();
                                    let date = tag.into_inner().next().unwrap();
                                    item.due = Some(parse_date(date.as_str(), date.line_col().0)?);
                                }
                                Rule::t => {
                                    if item.threshhold.is_some() {
                                        return Err(format!(
                                            "Second threshhold tag found in line {line}",
                                            line = tag.line_col().0
                                        ));
                                    }
                                    item.content.replace_range(tag_span.clone(), "");
                                    content_start += tag_span.len();
                                    let date = tag.into_inner().next().unwrap();
                                    item.threshhold =
                                        Some(parse_date(date.as_str(), date.line_col().0)?);
                                }
                                Rule::pri => {
                                    if item.priority.is_some() {
                                        return Err(format!(
                                            "found scond priority in line {line}",
                                            line = tag.line_col().0
                                        ));
                                    }
                                    item.content.replace_range(tag_span.clone(), "");
                                    content_start += tag_span.len();
                                    item.priority = Some(
                                        Priority::try_from(tag.as_str().chars().nth(4).unwrap())
                                            .unwrap(),
                                    );
                                }
                                Rule::context | Rule::project => {}
                                _ => unreachable!(),
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }

            items.push(item)
        }

        // Sort items
        items.sort();

        let filtered = (0..items.len()).collect();

        // Collect projects and contexts
        let contexts = items
            .iter()
            .flat_map(|item| item.contexts().map(str::to_owned))
            .collect();
        let projects = items
            .iter()
            .flat_map(|item| item.projects().map(str::to_owned))
            .collect();

        Ok(TodoList {
            items,
            filter: Default::default(),
            filtered,
            contexts,
            projects,
        })
    }
}

impl Display for TodoList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for item in self.items.iter() {
            writeln!(f, "{item}")?;
        }

        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug, Default, Clone)]
pub struct Filter {
    /// Optional priority to filter for
    pub priority: Option<Option<Priority>>,
    /// Optional completion to filter for
    pub completed: Option<bool>,
    /// Words to search for
    pub words: Vec<String>,
    /// Ignore case while searching words
    pub ignore_case: bool,
}

impl Filter {
    pub fn new(ignore_case: bool) -> Filter {
        Self {
            ignore_case,
            ..Default::default()
        }
    }
}

/// A single todo item
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct TodoItem {
    /// If it has been completed
    completed: bool,
    /// The priority
    pub priority: Option<Priority>,
    /// The content including contexts and projects but excluding
    /// `label:value` tags, `priority` and `completion`
    content: String,
    /// The creation date
    creation_date: Option<NaiveDate>,
    /// The completion date
    completion_date: Option<NaiveDate>,
    /// The due date
    pub due: Option<NaiveDate>,
    /// The threshhold date
    pub threshhold: Option<NaiveDate>,
    /// Recurrence
    pub recurring: Option<Recurring>,
}

impl TodoItem {
    /// Create a new item and applies today as creation date if `creation_date` is `true`
    pub fn new(creation_date: bool) -> TodoItem {
        Self {
            creation_date: creation_date.then_some(Local::now().date_naive()),
            ..Default::default()
        }
    }

    /// Return if this item is valid e.g. the content does not contain tags like 'due:...'
    pub fn valid(&self) -> bool {
        !self.iter_content_parts().any(|p| {
            matches!(
                p,
                ContentPart::Due(_)
                    | ContentPart::Priority(_)
                    | ContentPart::Recurrence(_)
                    | ContentPart::Threshhold(_)
            )
        })
    }

    /// Return if this item has been completed
    pub fn completed(&self) -> bool {
        self.completed
    }

    /// Toggle item completion, updating completion date if creation date was set
    /// Optionally returns a new item if recurrence was set
    pub fn toggle_completed(&mut self) -> Option<TodoItem> {
        self.completed = !self.completed;
        self.completion_date =
            (self.completed && self.creation_date.is_some()).then_some(Local::now().date_naive());

        if let (true, Some(rec)) = (self.completed, self.recurring) {
            let mut copy = self.clone();
            let now = Local::now().date_naive();
            copy.completion_date = None;
            copy.completed = false;

            if self.creation_date.is_some() {
                copy.creation_date = Some(now);
            }

            if let Some(date) = self.due {
                copy.due = Some(rec.apply(date, now));
            }

            if let Some(date) = self.threshhold {
                copy.threshhold = Some(rec.apply(date, now));
            }

            Some(copy)
        } else {
            None
        }
    }

    /// Return the item content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set the item content
    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }

    /// Iter through the contexts present in the item content
    pub fn contexts(&self) -> impl Iterator<Item = &str> {
        self.iter_content_parts().filter_map(|p| match p {
            ContentPart::Context(s) => Some(s),
            _ => None,
        })
    }

    /// Iter through the projects present in the item content
    pub fn projects(&self) -> impl Iterator<Item = &str> {
        self.iter_content_parts().filter_map(|p| match p {
            ContentPart::Project(s) => Some(s),
            _ => None,
        })
    }

    /// Iter through all parts of the item content
    pub fn iter_content_parts(&self) -> impl Iterator<Item = ContentPart> {
        let mut content = vec![];

        if self.content.is_empty() {
            return content.into_iter();
        }

        let mut index = 0;

        for item in TodoListParser::parse(Rule::content, &self.content)
            .unwrap()
            .next()
            .unwrap()
            .into_inner()
            .flat_map(|item| item.into_inner())
        {
            if item.as_span().start() > index {
                content.push(ContentPart::Normal(
                    &self.content[index..item.as_span().start()],
                ))
            }
            index = item.as_span().end();

            match item.as_rule() {
                Rule::context => content.push(ContentPart::Context(item.as_str())),
                Rule::project => content.push(ContentPart::Project(item.as_str())),
                Rule::rec => content.push(ContentPart::Recurrence(item.as_str())),
                Rule::due => content.push(ContentPart::Due(item.as_str())),
                Rule::pri => content.push(ContentPart::Priority(item.as_str())),
                Rule::t => content.push(ContentPart::Threshhold(item.as_str())),
                _ => unreachable!(),
            }
        }

        if index < self.content.len() {
            content.push(ContentPart::Normal(&self.content[index..]));
        }

        content.into_iter()
    }
}

impl PartialOrd for TodoItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TodoItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare by completion
        self.completed
            .cmp(&other.completed)
            // Compare by priority
            .then(
                self.priority
                    .map(Reverse)
                    .cmp(&other.priority.map(Reverse))
                    .reverse(),
            )
            // Compare by due date
            .then(
                self.due
                    .map(cmp::Reverse)
                    .cmp(&other.due.map(cmp::Reverse))
                    .reverse(),
            )
            // Compare by threshhold date
            .then(self.threshhold.cmp(&other.threshhold))
            // Compare by completion date
            .then(self.completion_date.cmp(&other.completion_date))
            // Compare by creation date
            .then(self.creation_date.cmp(&other.creation_date))
            // Compare by recurrence
            .then(self.recurring.is_none().cmp(&other.recurring.is_none()))
            // Compare by content
            .then(self.content.cmp(&other.content))
    }
}

impl Display for TodoItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.completed {
            write!(f, "x ")?;
        } else if let Some(Priority(prio)) = self.priority {
            write!(f, "({prio}) ")?;
        }
        if let Some(date) = self.completion_date {
            write!(f, "{date} ")?;
        }
        if let Some(date) = self.creation_date {
            write!(f, "{date} ")?;
        }

        write!(f, "{content}", content = self.content)?;

        if let (true, Some(Priority(priority))) = (self.completed, self.priority) {
            write!(f, " pri:{priority}")?;
        }
        if let Some(date) = self.due {
            write!(f, " due:{date}")?;
        }
        if let Some(date) = self.threshhold {
            write!(f, " t:{date}")?;
        }
        if let Some(recurring) = &self.recurring {
            write!(f, " rec:{recurring}")?;
        }

        Ok(())
    }
}

/// A part of an item content
pub enum ContentPart<'item> {
    Normal(&'item str),
    Context(&'item str),
    Project(&'item str),
    Recurrence(&'item str),
    Due(&'item str),
    Priority(&'item str),
    Threshhold(&'item str),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default, Clone, Copy)]
pub struct Priority(pub char);

impl TryFrom<char> for Priority {
    type Error = String;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        if !value.is_ascii_uppercase() {
            Err(format!("Not a valid priority: '{value}'"))
        } else {
            Ok(Priority(value))
        }
    }
}

/// A recurrence pattern for todo items
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Recurring {
    /// Whether its relative to completion
    pub relative: bool,
    /// The amount
    pub amount: u32,
    /// The unit of recurrence
    pub unit: RecurringUnit,
}

impl Recurring {
    pub fn apply(&self, date: NaiveDate, now: NaiveDate) -> NaiveDate {
        let date = if self.relative { now } else { date };
        match self.unit {
            RecurringUnit::Days => date
                .checked_add_days(Days::new(self.amount as u64))
                .unwrap(),
            RecurringUnit::Weeks => date
                .checked_add_days(Days::new(self.amount as u64 * 7))
                .unwrap(),
            RecurringUnit::Months => date.checked_add_months(Months::new(self.amount)).unwrap(),
            RecurringUnit::Years => date
                .checked_add_months(Months::new(self.amount * 12))
                .unwrap(),
        }
    }
}

/// Unit of a recurrence pattern
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RecurringUnit {
    Days,
    Weeks,
    Months,
    Years,
}

impl FromStr for Recurring {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let relative = s.starts_with('+');
        let amount = s[if relative { 1 } else { 0 }..s.len() - 1].parse()?;
        let unit = match s.chars().last().unwrap() {
            'd' => RecurringUnit::Days,
            'w' => RecurringUnit::Weeks,
            'm' => RecurringUnit::Months,
            'y' => RecurringUnit::Years,
            _ => unreachable!(),
        };

        Ok(Self {
            relative,
            amount,
            unit,
        })
    }
}

impl Display for Recurring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let unit = match self.unit {
            RecurringUnit::Days => "d",
            RecurringUnit::Weeks => "w",
            RecurringUnit::Months => "m",
            RecurringUnit::Years => "y",
        };

        write!(
            f,
            "{relative}{amount}{unit}",
            amount = self.amount,
            relative = if self.relative { "+" } else { "" }
        )
    }
}
