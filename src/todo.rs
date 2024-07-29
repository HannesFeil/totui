use chrono::NaiveDate;
use std::{fmt::Display, str::FromStr};

pub struct TodoList {
    items: Vec<TodoItem>,
}

impl TodoList {
    pub fn parse(input: &str) -> anyhow::Result<Self> {
        let mut items = vec![];

        for line in input.lines() {
            let item = line.parse()?;
            println!("{item}");
            items.push(item);
        }

        Ok(Self { items })
    }
}

#[derive(Debug)]
pub struct TodoItem {
    pub completion_date: Option<NaiveDate>,
    priority: Option<char>,
    pub creation_date: NaiveDate,
    content: Vec<ContentPart>,
    context_indices: Vec<usize>,
    project_indices: Vec<usize>,
    rec_index: Option<usize>,
    due_index: Option<usize>,
    t_index: Option<usize>,
    pri_index: Option<usize>,
}

impl Display for TodoItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(date) = self.completion_date {
            write!(f, "x {date} ", date = date.format("%Y-%m-%d"))?;
        } else if let Some(priority) = self.priority {
            write!(f, "({priority}) ")?;
        }

        write!(f, "{creation_date} ", creation_date = self.creation_date,)?;

        for part in &self.content {
            write!(f, "{part}")?;
        }

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
            pri_index: None,
        }
    }

    pub fn priority(&self) -> Option<char> {
        self.priority.or(self.pri_index.map(|i| {
            let ContentPart::Pri(priority) = self.content[i] else {
                unreachable!();
            };

            priority
        }))
    }

    pub fn priority_mut(&mut self) -> Option<&mut char> {
        self.priority.as_mut().or(self.pri_index.map(|i| {
            let ContentPart::Pri(priority) = &mut self.content[i] else {
                unreachable!();
            };

            priority
        }))
    }

    pub fn contexts(&self) -> impl Iterator<Item = &str> {
        self.context_indices.iter().map(|i| {
            let ContentPart::Context(s) = &self.content[*i] else {
                unreachable!();
            };

            s.as_str()
        })
    }

    pub fn projects(&self) -> impl Iterator<Item = &str> {
        self.project_indices.iter().map(|i| {
            let ContentPart::Project(s) = &self.content[*i] else {
                unreachable!();
            };

            s.as_str()
        })
    }

    pub fn recurring(&self) -> Option<(bool, u32, RecurringUnit)> {
        self.rec_index.map(|i| {
            let ContentPart::Rec {
                relative,
                amount,
                unit,
            } = self.content[i]
            else {
                unreachable!()
            };

            (relative, amount, unit)
        })
    }

    pub fn recurring_mut(&mut self) -> Option<(&mut bool, &mut u32, &mut RecurringUnit)> {
        self.rec_index.map(|i| {
            let ContentPart::Rec {
                relative,
                amount,
                unit,
            } = &mut self.content[i]
            else {
                unreachable!()
            };

            (relative, amount, unit)
        })
    }

    pub fn due_date(&self) -> Option<&NaiveDate> {
        self.due_index.map(|i| {
            let ContentPart::Due(date) = &self.content[i] else {
                unreachable!()
            };

            date
        })
    }

    pub fn due_date_mut(&mut self) -> Option<&mut NaiveDate> {
        self.due_index.map(|i| {
            let ContentPart::Due(date) = &mut self.content[i] else {
                unreachable!()
            };

            date
        })
    }

    pub fn t_date(&self) -> Option<&NaiveDate> {
        self.t_index.map(|i| {
            let ContentPart::T(date) = &self.content[i] else {
                unreachable!()
            };

            date
        })
    }

    pub fn t_date_mut(&mut self) -> Option<&mut NaiveDate> {
        self.t_index.map(|i| {
            let ContentPart::T(date) = &mut self.content[i] else {
                unreachable!()
            };

            date
        })
    }
}

#[derive(Debug)]
pub enum ContentPart {
    Space(String),
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

impl Display for ContentPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentPart::Space(string) => f.write_str(string),
            ContentPart::Word(string) => f.write_str(string),
            ContentPart::Context(string) => write!(f, "@{string}"),
            ContentPart::Project(string) => write!(f, "+{string}"),
            ContentPart::Rec {
                relative,
                amount,
                unit,
            } => write!(
                f,
                "rec:{rel}{amount}{unit}",
                rel = if *relative { "+" } else { "" }
            ),
            ContentPart::Due(date) => write!(f, "due:{date}", date = date.format("%Y-%m-%d")),
            ContentPart::T(date) => write!(f, "t:{date}", date = date.format("%Y-%m-%d")),
            ContentPart::Pri(prio) => write!(f, "pri:{prio}"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RecurringUnit {
    Days,
    Weeks,
    Months,
    Years,
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

    use crate::todo::ContentPart;

    use super::TodoItem;

    #[derive(Parser)]
    #[grammar = "./todo_grammar.pest"]
    struct TodoParser;

    #[derive(Debug)]
    pub struct ItemParseError {
        error_message: String,
        error_span: Range<usize>,
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

    impl TodoItem {
        fn from_item_pair(item_pair: Pair<Rule>) -> Result<Self, ItemParseError> {
            let mut completion_date = None;
            let mut priority = None;
            let mut creation_date = None;
            let mut content = vec![];
            let mut context_indices = vec![];
            let mut project_indices = vec![];
            let mut rec_index = None;
            let mut due_index = None;
            let mut t_index = None;
            let mut pri_index = None;

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
                                    content.push(ContentPart::Space(part.as_str().to_owned()));
                                }
                                Rule::word => {
                                    content.push(ContentPart::Word(part.as_str().to_owned()));
                                }
                                Rule::context => {
                                    context_indices.push(content.len());
                                    let inner_word = unwrap_single_inner(part, Rule::word);
                                    content
                                        .push(ContentPart::Context(inner_word.as_str().to_owned()));
                                }
                                Rule::project => {
                                    project_indices.push(content.len());
                                    let inner_word = unwrap_single_inner(part, Rule::word);
                                    content
                                        .push(ContentPart::Project(inner_word.as_str().to_owned()));
                                }
                                Rule::rec => {
                                    let None = rec_index else {
                                        return Err(ItemParseError {
                                            error_message: "Illegal second 'rec' definition"
                                                .to_owned(),
                                            error_span: span,
                                        });
                                    };

                                    rec_index = Some(content.len());

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

                                    content.push(ContentPart::Rec {
                                        relative,
                                        amount,
                                        unit,
                                    })
                                }
                                Rule::due => {
                                    let None = due_index else {
                                        return Err(ItemParseError {
                                            error_message: "Illegal second 'due' definition"
                                                .to_owned(),
                                            error_span: span,
                                        });
                                    };

                                    due_index = Some(content.len());
                                    let inner = unwrap_single_inner(part, Rule::date);
                                    let due_date = parse_date(inner.as_str(), inner.as_span())?;
                                    content.push(ContentPart::Due(due_date));
                                }
                                Rule::pri => {
                                    let (None, None) = (pri_index, priority) else {
                                        return Err(ItemParseError {
                                            error_message: "Illegal second 'pri' definition"
                                                .to_owned(),
                                            error_span: span,
                                        });
                                    };

                                    pri_index = Some(content.len());
                                    let inner = part.into_inner().next().unwrap();
                                    let pri_char = inner.as_str().chars().next().unwrap();
                                    content.push(ContentPart::Pri(pri_char));
                                }
                                Rule::t => {
                                    let None = t_index else {
                                        return Err(ItemParseError {
                                            error_message: "Illegal second 't' definition"
                                                .to_owned(),
                                            error_span: span,
                                        });
                                    };

                                    t_index = Some(content.len());
                                    let inner = unwrap_single_inner(part, Rule::date);
                                    let t_date = parse_date(inner.as_str(), inner.as_span())?;
                                    content.push(ContentPart::T(t_date));
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }

            Ok(Self {
                completion_date,
                priority,
                creation_date: creation_date.unwrap(),
                content,
                context_indices,
                project_indices,
                rec_index,
                due_index,
                t_index,
                pri_index,
            })
        }
    }
}
