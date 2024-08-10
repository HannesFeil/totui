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

#[derive(Debug)]
pub struct TodoItem {
    pub completion_date: Option<NaiveDate>,
    priority: Option<Priority>,
    pub creation_date: NaiveDate,
    content: Vec<ContentPart>,
    context_indices: Vec<usize>,
    project_indices: Vec<usize>,
    rec_index: Option<usize>,
    due_index: Option<usize>,
    t_index: Option<usize>,
}

#[derive(Debug, PartialEq, Eq)]
enum Priority {
    Literal(char),
    Index(usize),
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
    Rec {
        relative: bool,
        amount: u32,
        unit: RecurringUnit,
    },
    Due(NaiveDate),
    T(NaiveDate),
    Pri(char),
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
        } else if let Some(Priority::Literal(priority)) = self.priority {
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

        Ok(())
    }
}

impl TodoItem {
    pub fn new(creation_date: NaiveDate) -> Self {
        Self {
            completion_date: None,
            priority: None,
            creation_date,
            content: vec![],
            context_indices: vec![],
            project_indices: vec![],
            rec_index: None,
            due_index: None,
            t_index: None,
        }
    }

    fn set_indices(&mut self) {
        self.rec_index = None;
        self.due_index = None;
        self.t_index = None;
        self.context_indices.clear();
        self.project_indices.clear();
        if self
            .priority
            .as_ref()
            .is_some_and(|p| matches!(p, Priority::Index(_)))
        {
            self.priority = None;
        }

        for (index, part) in self.content.iter().enumerate() {
            match &part.content {
                Content::Word(_) => {}
                Content::Context(_) => self.context_indices.push(index),
                Content::Project(_) => self.project_indices.push(index),
                Content::Rec { .. } => {
                    assert_eq!(self.rec_index, None);
                    self.rec_index = Some(index);
                }
                Content::Due(_) => {
                    assert_eq!(self.due_index, None);
                    self.due_index = Some(index);
                }
                Content::T(_) => {
                    assert_eq!(self.t_index, None);
                    self.t_index = Some(index);
                }
                Content::Pri(_) => {
                    assert_eq!(self.priority, None);
                    self.priority = Some(Priority::Index(index));
                }
            }
        }
    }

    pub fn priority(&self) -> Option<char> {
        Some(match self.priority.as_ref()? {
            Priority::Literal(priority) => *priority,
            Priority::Index(index) => {
                let Content::Pri(priority) = self.content[*index].content else {
                    unreachable!();
                };

                priority
            }
        })
    }

    pub fn set_priority(&mut self, priority: Option<char>) {
        match (&self.priority, priority) {
            (None, None) => {}
            (None, Some(prio)) => {
                if self.completion_date.is_some() {
                    self.priority = Some(Priority::Index(self.content.len()));
                    self.content.push(ContentPart {
                        space: " ".to_owned(),
                        content: Content::Pri(prio),
                    });
                } else {
                    self.priority = Some(Priority::Literal(prio));
                }
            }
            (Some(Priority::Literal(_)), _) => self.priority = priority.map(Priority::Literal),
            (Some(Priority::Index(index)), None) => {
                self.content.remove(*index);
                self.set_indices();
                self.priority = None;
            }
            (Some(Priority::Index(index)), Some(prio)) => {
                let Content::Pri(p) = &mut self.content[*index].content else {
                    unreachable!();
                };

                *p = prio;
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

    pub fn recurring(&self) -> Option<(bool, u32, RecurringUnit)> {
        self.rec_index.map(|i| {
            let Content::Rec {
                relative,
                amount,
                unit,
            } = self.content[i].content
            else {
                unreachable!()
            };

            (relative, amount, unit)
        })
    }

    pub fn set_recurring(&mut self, recurring: Option<(bool, u32, RecurringUnit)>) {
        match (self.rec_index, recurring) {
            (None, None) => {}
            (Some(index), None) => {
                self.rec_index = None;
                self.content.remove(index);
                self.set_indices();
            }
            (Some(index), Some((relative, amount, unit))) => {
                assert!(matches!(self.content[index].content, Content::Rec { .. }));
                self.content[index].content = Content::Rec {
                    relative,
                    amount,
                    unit,
                };
            }
            (None, Some((relative, amount, unit))) => {
                self.rec_index = Some(self.content.len());
                self.content.push(ContentPart {
                    space: " ".to_owned(),
                    content: Content::Rec {
                        relative,
                        amount,
                        unit,
                    },
                })
            }
        }
    }

    pub fn due_date(&self) -> Option<&NaiveDate> {
        self.due_index.map(|i| {
            let Content::Due(date) = &self.content[i].content else {
                unreachable!()
            };

            date
        })
    }

    pub fn set_due_date(&mut self, date: Option<&mut NaiveDate>) {
        self.due_index.map(|i| {
            let Content::Due(date) = &mut self.content[i].content else {
                unreachable!()
            };

            date
        })
    }

    pub fn t_date(&self) -> Option<&NaiveDate> {
        self.t_index.map(|i| {
            let Content::T(date) = &self.content[i].content else {
                unreachable!()
            };

            date
        })
    }

    pub fn t_date_mut(&mut self) -> Option<&mut NaiveDate> {
        self.t_index.map(|i| {
            let Content::T(date) = &mut self.content[i].content else {
                unreachable!()
            };

            date
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
            Content::Rec {
                relative,
                amount,
                unit,
            } => write!(
                f,
                "rec:{rel}{amount}{unit}",
                rel = if *relative { "+" } else { "" }
            ),
            Content::Due(date) => write!(f, "due:{date}", date = date.format("%Y-%m-%d")),
            Content::T(date) => write!(f, "t:{date}", date = date.format("%Y-%m-%d")),
            Content::Pri(prio) => write!(f, "pri:{prio}"),
        }
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

    use crate::todo::{Content, ContentPart, Priority};

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
            let mut content = vec![];
            let mut due_set = false;
            let mut t_set = false;
            let mut rec_set = false;

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
                        priority = Some(Priority::Literal(pair.as_str().chars().next().unwrap()));
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
                                    assert_eq!(preceding_space, None);
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
                                    if rec_set {
                                        return Err(ItemParseError {
                                            error_message: "Illegal second 'rec' definition"
                                                .to_owned(),
                                            error_span: span,
                                        });
                                    };
                                    rec_set = true;

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

                                    content.push(ContentPart {
                                        space: preceding_space.take().unwrap(),
                                        content: Content::Rec {
                                            relative,
                                            amount,
                                            unit,
                                        },
                                    })
                                }
                                Rule::due => {
                                    if due_set {
                                        return Err(ItemParseError {
                                            error_message: "Illegal second 'due' definition"
                                                .to_owned(),
                                            error_span: span,
                                        });
                                    };
                                    due_set = true;

                                    let inner = unwrap_single_inner(part, Rule::date);
                                    let due_date = parse_date(inner.as_str(), inner.as_span())?;
                                    content.push(ContentPart {
                                        space: preceding_space.take().unwrap(),
                                        content: Content::Due(due_date),
                                    });
                                }
                                Rule::pri => {
                                    if priority.is_some() {
                                        return Err(ItemParseError {
                                            error_message: "Illegal second 'pri' definition"
                                                .to_owned(),
                                            error_span: span,
                                        });
                                    };

                                    priority = Some(Priority::Index(content.len()));
                                    let inner = part.into_inner().next().unwrap();
                                    let pri_char = inner.as_str().chars().next().unwrap();
                                    content.push(ContentPart {
                                        space: preceding_space.take().unwrap(),
                                        content: Content::Pri(pri_char),
                                    });
                                }
                                Rule::t => {
                                    if t_set {
                                        return Err(ItemParseError {
                                            error_message: "Illegal second 't' definition"
                                                .to_owned(),
                                            error_span: span,
                                        });
                                    };
                                    t_set = true;

                                    let inner = unwrap_single_inner(part, Rule::date);
                                    let t_date = parse_date(inner.as_str(), inner.as_span())?;
                                    content.push(ContentPart {
                                        space: preceding_space.take().unwrap(),
                                        content: Content::T(t_date),
                                    });
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
                content,
                context_indices: vec![],
                project_indices: vec![],
                rec_index: None,
                due_index: None,
                t_index: None,
            };
            this.set_indices();
            Ok(this)
        }
    }
}
