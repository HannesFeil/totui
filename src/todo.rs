use std::{fmt::Display, num::ParseIntError, ops::Range, str::FromStr};

use chrono::NaiveDate;
use pest::Parser;
use pest_derive::Parser;
use ratatui::text::Text;

#[derive(Parser)]
#[grammar = "./todo_grammar.pest"]
struct TodoListParser;

#[derive(Debug)]
pub struct TodoList {
    items: Vec<TodoItem>,
}

impl AsRef<[TodoItem]> for TodoList {
    fn as_ref(&self) -> &[TodoItem] {
        self.items.as_slice()
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
            let mut done = false;
            let mut item = TodoItem::default();

            for rule in line.into_inner() {
                match rule.as_rule() {
                    Rule::completed => done = true,
                    Rule::priority => item.priority = Some(rule.as_str().chars().nth(1).unwrap()),
                    Rule::date => {
                        if done && item.completed.is_none() {
                            item.completed = Some(
                                NaiveDate::parse_from_str(rule.as_str(), "%Y-%m-%d").map_err(
                                    |_| {
                                        format!(
                                            "Failed to parse completion date in line {line}",
                                            line = rule.line_col().0
                                        )
                                    },
                                )?,
                            );
                        } else {
                            item.created = Some(
                                NaiveDate::parse_from_str(rule.as_str(), "%Y-%m-%d").map_err(
                                    |_| {
                                        format!(
                                            "Failed to parse creation date in line {line}",
                                            line = rule.line_col().0
                                        )
                                    },
                                )?,
                            );
                        }
                    }
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
                                Rule::project => {
                                    item.projects.push(tag_span);
                                }
                                Rule::context => {
                                    item.contexts.push(tag_span);
                                }
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
                                    item.due = Some(
                                        NaiveDate::parse_from_str(date.as_str(), "%Y-%m-%d")
                                            .unwrap(),
                                    );
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
                                    item.threshhold = Some(
                                        NaiveDate::parse_from_str(date.as_str(), "%Y-%m-%d")
                                            .unwrap(),
                                    );
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
                                    item.priority = Some(tag.as_str().chars().nth(4).unwrap());
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }

            if item.completed.is_some() && item.created.is_none() {
                return Err("Found completion date without creation date".to_owned());
            }

            items.push(item)
        }

        Ok(TodoList { items })
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

#[derive(Default, Debug)]
pub struct TodoItem {
    priority: Option<char>,
    content: String,
    contexts: Vec<Range<usize>>,
    projects: Vec<Range<usize>>,
    created: Option<NaiveDate>,
    completed: Option<NaiveDate>,
    due: Option<NaiveDate>,
    threshhold: Option<NaiveDate>,
    recurring: Option<Recurring>, // TODO: better type
}

impl From<&TodoItem> for Text<'_> {
    fn from(value: &TodoItem) -> Self {
        // TODO: implement styling
        Text::raw(value.to_string())
    }
}

impl Display for TodoItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.completed.is_some() {
            write!(f, "x ")?;
        } else if let Some(prio) = self.priority {
            write!(f, "({prio}) ")?;
        }
        if let Some(date) = self.completed {
            write!(f, "{date} ")?;
        }
        if let Some(date) = self.created {
            write!(f, "{date} ")?;
        }

        write!(f, "{content}", content = self.content)?;

        if let (Some(_), Some(priority)) = (self.completed, self.priority) {
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

#[derive(Debug)]
pub struct Recurring {
    pub amount: u32,
    pub unit: RecurringUnit,
}

#[derive(Debug)]
pub enum RecurringUnit {
    Days,
    Weeks,
    Months,
    Years,
}

impl FromStr for Recurring {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let amount = s[..s.len() - 1].parse()?;
        let unit = match s.chars().last().unwrap() {
            'd' => RecurringUnit::Days,
            'w' => RecurringUnit::Weeks,
            'm' => RecurringUnit::Months,
            'y' => RecurringUnit::Years,
            _ => unreachable!(),
        };

        Ok(Self { amount, unit })
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

        write!(f, "{amount}{unit}", amount = self.amount)
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::todo::TodoList;

    #[test]
    fn test_parsing() {
        let test_input = "\
(A) 2020-02-20 due:2031-07-11 Thank Mom for the meatballs @phone
(B) Schedule Goodwill t:5032-12-09 pickup +GarageSale @phone
x 2000-11-14 3000-11-30 Post rec:1w signs around pri:B the neighborhood +GarageSale
@GroceryStore Eskimo pies
";
        let list = TodoList::from_str(test_input).unwrap();
        println!("{list}");
    }
}
