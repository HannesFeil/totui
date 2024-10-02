use chrono::NaiveDate;
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub struct TodoList {
    items: Vec<TodoItem>,
}

impl Deref for TodoList {
    type Target = Vec<TodoItem>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl DerefMut for TodoList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
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

#[derive(Debug)]
pub struct TodoItem {
    pub completion_date: Option<NaiveDate>,
    pub priority: Option<char>,
    pub creation_date: NaiveDate,
    pub rec: Option<Recurring>,
    pub due: Option<NaiveDate>,
    pub t: Option<NaiveDate>,
    content: Vec<ContentPart>,
    context_indices: Vec<usize>,
    project_indices: Vec<usize>,
}

#[derive(Debug)]
pub struct ContentPart {
    pub space: String,
    pub content: Content,
}

#[derive(Debug)]
pub enum Content {
    Word(String),
    Context(String),
    Project(String),
}

#[derive(Debug, Clone, Copy)]
pub struct Recurring {
    relative: bool,
    amount: u32,
    unit: RecurringUnit,
}

#[derive(Debug, Clone, Copy)]
pub enum RecurringUnit {
    Days,
    Weeks,
    Months,
    Years,
}

impl Display for TodoItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(date) = self.completion_date {
            write!(f, "x {date} ", date = date.format("%Y-%m-%d"))?;
        } else if let Some(priority) = self.priority {
            write!(f, "({priority}) ")?;
        }

        write!(f, "{creation_date} ", creation_date = self.creation_date)?;

        if let Some(part) = self.content.first() {
            write!(f, "{content}", content = part.content)?;
            for part in &self.content[1..] {
                write!(
                    f,
                    "{space}{content}",
                    space = part.space,
                    content = part.content
                )?;
            }
        };

        if let Some(rec) = self.rec {
            write!(f, " rec:{rec}")?;
        }

        if let Some(due) = self.due {
            write!(f, " due:{date}", date = due.format("%Y-%m-%d"))?;
        }

        if let Some(t) = self.t {
            write!(f, " t:{date}", date = t.format("%Y-%m-%d"))?;
        }

        if let Some(prio) = self.priority {
            if self.completion_date.is_some() {
                write!(f, " pri:{prio}")?;
            }
        }

        Ok(())
    }
}

impl PartialEq for TodoItem {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other).is_some_and(|c| c.is_eq())
    }
}

impl Eq for TodoItem {}

impl PartialOrd for TodoItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TodoItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.completion_date
            .is_some()
            .cmp(&other.completion_date.is_some())
            .then(match (self.priority, other.priority) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(p1), Some(p2)) => p1.cmp(&p2),
            })
            .then(self.creation_date.cmp(&other.creation_date))
    }
}

impl TodoItem {
    pub fn new(creation_date: NaiveDate) -> Self {
        Self {
            completion_date: None,
            priority: None,
            creation_date,
            rec: None,
            due: None,
            t: None,
            content: vec![],
            context_indices: vec![],
            project_indices: vec![],
        }
    }

    fn set_indices(&mut self) {
        self.context_indices.clear();
        self.project_indices.clear();

        for (index, part) in self.content.iter().enumerate() {
            match &part.content {
                Content::Word(_) => {}
                Content::Context(_) => self.context_indices.push(index),
                Content::Project(_) => self.project_indices.push(index),
            }
        }
    }

    pub fn contexts(&self) -> impl Iterator<Item = &str> {
        self.context_indices.iter().map(|i| {
            let Content::Context(s) = &self.content[*i].content else {
                unreachable!();
            };

            s.as_str()
        })
    }

    pub fn projects(&self) -> impl Iterator<Item = &str> {
        self.project_indices.iter().map(|i| {
            let Content::Project(s) = &self.content[*i].content else {
                unreachable!();
            };

            s.as_str()
        })
    }

    pub fn content_parts(&self) -> impl Iterator<Item = &ContentPart> {
        self.content.iter()
    }
}

impl Display for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Content::Word(string) => f.write_str(string),
            Content::Context(string) => write!(f, "@{string}"),
            Content::Project(string) => write!(f, "+{string}"),
        }
    }
}

impl Display for Recurring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.relative {
            write!(f, "+")?;
        }

        write!(f, "{amount}{unit}", amount = self.amount, unit = self.unit)
    }
}

impl Display for RecurringUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RecurringUnit::Days => "d",
            RecurringUnit::Weeks => "w",
            RecurringUnit::Months => "m",
            RecurringUnit::Years => "y",
        })
    }
}

pub mod parsing {
    use chrono::NaiveDate;
    use pest::{error::LineColLocation, iterators::Pair, Parser};
    use pest_derive::Parser;
    use std::{fmt::Display, ops::Range, str::FromStr};

    use crate::todo::{Content, ContentPart, Recurring};

    use super::{RecurringUnit, TodoItem, TodoList};

    #[derive(Parser)]
    #[grammar = "./todo_grammar.pest"]
    struct TodoParser;

    #[derive(Debug)]
    pub struct ItemParseError {
        pub error_message: String,
        pub error_span: Range<usize>,
    }

    impl Display for ItemParseError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{error} at {span:?}",
                error = self.error_message,
                span = self.error_span
            )
        }
    }

    impl std::error::Error for ItemParseError {}

    impl FromStr for TodoList {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let mut items = vec![];

            for (i, line) in s.lines().enumerate() {
                let item = line.parse().map_err(|e: ItemParseError| {
                    let mut markers = String::new();
                    markers.extend((0..e.error_span.start - 1).map(|_| ' '));
                    markers.extend(e.error_span.map(|_| '^'));
                    format!(
                        "Failed to parse item in line {i}:\n{line}\n{markers}\n{message}",
                        message = e.error_message
                    )
                })?;
                items.push(item);
            }
            Ok(Self { items })
        }
    }

    impl FromStr for TodoItem {
        type Err = ItemParseError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let parsed = TodoParser::parse(Rule::item, s);

            match parsed {
                Ok(mut pairs) => {
                    let item_pair = pairs.next().unwrap();
                    assert!(matches!(item_pair.as_rule(), Rule::item));
                    Self::from_item_pair(item_pair)
                }
                Err(e) => match e.line_col {
                    LineColLocation::Pos((col_pos, _)) => Err(ItemParseError {
                        error_message: e.variant.message().into_owned(),
                        error_span: col_pos..col_pos + 1,
                    }),
                    LineColLocation::Span((col_start, _), (col_end, _)) => Err(ItemParseError {
                        error_message: e.variant.message().into_owned(),
                        error_span: col_start..col_end,
                    }),
                },
            }
        }
    }

    impl FromStr for RecurringUnit {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "d" => Ok(RecurringUnit::Days),
                "w" => Ok(RecurringUnit::Weeks),
                "m" => Ok(RecurringUnit::Months),
                "y" => Ok(RecurringUnit::Years),
                _ => Err(format!("Invalid recurring unit '{s}'")),
            }
        }
    }

    impl TodoItem {
        fn from_item_pair(item_pair: Pair<Rule>) -> Result<Self, ItemParseError> {
            let mut completion_date = None;
            let mut priority = None;
            let mut creation_date = None;
            let mut due = None;
            let mut t = None;
            let mut rec = None;
            let mut content = vec![];

            fn parse_date(date: &str, span: pest::Span) -> Result<NaiveDate, ItemParseError> {
                NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| ItemParseError {
                    error_message: "Failed to parse date".to_owned(),
                    error_span: span.start_pos().line_col().1..span.end_pos().line_col().1,
                })
            }

            fn unwrap_single_inner(pair: Pair<Rule>, expected_rule: Rule) -> Pair<Rule> {
                let mut inner = pair.into_inner();
                debug_assert_eq!(inner.len(), 1);
                let single = inner.next().unwrap();
                debug_assert_eq!(single.as_rule(), expected_rule);
                single
            }

            let mut preceding_space = Some(" ".to_owned());
            for pair in item_pair.into_inner() {
                match pair.as_rule() {
                    Rule::completed => {
                        let date_pair = unwrap_single_inner(pair, Rule::date);
                        completion_date =
                            Some(parse_date(date_pair.as_str(), date_pair.as_span())?);
                    }
                    Rule::priority_char => {
                        priority = Some(pair.as_str().chars().next().unwrap());
                    }
                    Rule::date => {
                        creation_date = Some(parse_date(pair.as_str(), pair.as_span())?);
                    }
                    Rule::content => {
                        for part in pair.into_inner() {
                            let span = part.as_span().start_pos().line_col().1
                                ..part.as_span().end_pos().line_col().1;
                            match part.as_rule() {
                                Rule::content_space => {
                                    preceding_space = Some(part.as_str().to_owned());
                                }
                                Rule::word => {
                                    content.push(ContentPart {
                                        space: preceding_space.take().unwrap(),
                                        content: Content::Word(part.as_str().to_owned()),
                                    });
                                }
                                Rule::context => {
                                    let inner_word = unwrap_single_inner(part, Rule::word);
                                    content.push(ContentPart {
                                        space: preceding_space.take().unwrap(),
                                        content: Content::Context(inner_word.as_str().to_owned()),
                                    });
                                }
                                Rule::project => {
                                    let inner_word = unwrap_single_inner(part, Rule::word);
                                    content.push(ContentPart {
                                        space: preceding_space.take().unwrap(),
                                        content: Content::Project(inner_word.as_str().to_owned()),
                                    });
                                }
                                Rule::rec => {
                                    if rec.is_some() {
                                        return Err(ItemParseError {
                                            error_message: "Illegal second 'rec' definition"
                                                .to_owned(),
                                            error_span: span,
                                        });
                                    };

                                    let rec_inner = part.into_inner().next().unwrap();
                                    let (relative, rec_time) = match rec_inner.as_rule() {
                                        Rule::rec_time_rel => {
                                            (true, rec_inner.into_inner().next().unwrap())
                                        }
                                        Rule::rec_time => (false, rec_inner),
                                        _ => unreachable!(),
                                    };
                                    debug_assert!(matches!(rec_time.as_rule(), Rule::rec_time));
                                    let mut time_parts = rec_time.into_inner();
                                    let amount =
                                        time_parts.next().unwrap().as_str().parse().unwrap();
                                    let unit = time_parts.next().unwrap().as_str().parse().unwrap();
                                    rec = Some(Recurring {
                                        relative,
                                        amount,
                                        unit,
                                    });
                                }
                                Rule::due => {
                                    if due.is_some() {
                                        return Err(ItemParseError {
                                            error_message: "Illegal second 'due' definition"
                                                .to_owned(),
                                            error_span: span,
                                        });
                                    };

                                    let inner = unwrap_single_inner(part, Rule::date);
                                    let due_date = parse_date(inner.as_str(), inner.as_span())?;
                                    due = Some(due_date);
                                }
                                Rule::pri => {
                                    if priority.is_some() {
                                        return Err(ItemParseError {
                                            error_message: "Illegal second 'pri' definition"
                                                .to_owned(),
                                            error_span: span,
                                        });
                                    };

                                    let inner = part.into_inner().next().unwrap();
                                    let pri_char = inner.as_str().chars().next().unwrap();
                                    priority = Some(pri_char);
                                }
                                Rule::t => {
                                    if t.is_some() {
                                        return Err(ItemParseError {
                                            error_message: "Illegal second 't' definition"
                                                .to_owned(),
                                            error_span: span,
                                        });
                                    };

                                    let inner = unwrap_single_inner(part, Rule::date);
                                    let t_date = parse_date(inner.as_str(), inner.as_span())?;
                                    t = Some(t_date);
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }

            let mut this = Self {
                completion_date,
                priority,
                creation_date: creation_date.unwrap(),
                rec,
                due,
                t,
                content,
                context_indices: vec![],
                project_indices: vec![],
            };
            this.set_indices();
            Ok(this)
        }
    }
}
