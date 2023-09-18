use std::{cmp, fmt::Display, num::ParseIntError, ops::Range, str::FromStr};

use chrono::NaiveDate;
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
}

impl TodoList {
    /// Return a slice of references to `TodoItem`s which match the given filter
    pub fn filtered(&self, filter: &str, ignore_case: bool) -> Box<[&TodoItem]> {
        let mut search_words = Vec::new();
        let mut priority = None;
        let mut completed = None;

        for word in filter.split_whitespace() {
            if word.starts_with("prio:") && priority.is_none() {
                priority = Some(match word.chars().nth(5) {
                    Some(c) if c.is_ascii_alphabetic() => Some(c),
                    _ => None,
                });
            } else if word == "done:x" && completed.is_none() {
                completed = Some(true);
            } else if word == "done:" && completed.is_none() {
                completed = Some(false);
            } else {
                search_words.push(if ignore_case {
                    word.to_lowercase()
                } else {
                    word.to_string()
                });
            }
        }

        self.iter()
            .filter(move |item| {
                // Filter out priority
                if let Some(priority) = priority {
                    if item.priority != priority {
                        return false;
                    }
                }

                // Filter out completed
                if let Some(completed) = completed {
                    if item.completed != completed {
                        return false;
                    }
                }

                // Filter words
                let lower = if ignore_case {
                    item.content.to_lowercase()
                } else {
                    item.content.to_owned()
                };

                for word in &search_words {
                    if !lower.contains(word) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Iterate over all `TodoItem`s by reference
    pub fn iter(&self) -> impl Iterator<Item = &TodoItem> {
        self.items.iter()
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
                    Rule::priority => item.priority = Some(rule.as_str().chars().nth(1).unwrap()),
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
                                Rule::project => {
                                    item.projects.push(tag_start..tag_end);
                                }
                                Rule::context => {
                                    item.contexts.push(tag_start..tag_end);
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
                                    item.priority = Some(tag.as_str().chars().nth(4).unwrap());
                                }
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

/// A single todo item
#[derive(Default, Debug, PartialEq, Eq)]
pub struct TodoItem {
    /// If it has been completed
    completed: bool,
    /// The priority
    priority: Option<char>,
    /// The content including contexts and projects but excluding 
    /// `label:value` tags, `priority` and `completion`
    content: String,
    /// Ranges of contexts in `content`
    contexts: Vec<Range<usize>>,
    /// Ranges of projects in `content`
    projects: Vec<Range<usize>>,
    /// The creation date
    creation_date: Option<NaiveDate>,
    /// The completion date
    completion_date: Option<NaiveDate>,
    /// The due date
    due: Option<NaiveDate>,
    /// The threshhold date
    threshhold: Option<NaiveDate>,
    /// Recurrence
    recurring: Option<Recurring>,
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
                    .unwrap_or(char::MAX)
                    .cmp(&other.priority.unwrap_or(char::MAX)),
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
        } else if let Some(prio) = self.priority {
            write!(f, "({prio}) ")?;
        }
        if let Some(date) = self.completion_date {
            write!(f, "{date} ")?;
        }
        if let Some(date) = self.creation_date {
            write!(f, "{date} ")?;
        }

        write!(f, "{content}", content = self.content)?;

        if let (true, Some(priority)) = (self.completed, self.priority) {
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

/// A recurrence pattern for todo items
#[derive(Debug, PartialEq, Eq)]
pub struct Recurring {
    /// The amount
    pub amount: u32,
    /// The unit of recurrence
    pub unit: RecurringUnit,
}

/// Unit of a recurrence pattern
#[derive(Debug, PartialEq, Eq)]
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

    const TEST_INPUT: &str = "\
(A) 2020-02-20 due:2031-07-11 Thank Mom for the meatballs @phone
(B) Schedule Goodwill t:5032-12-09 pickup +GarageSale @phone
(C) Bernard Goodwill thinkgs @beans are awesome?
x 2000-11-14 3000-11-30 Post rec:1w signs around pri:B the neighborhood +GarageSale
@GroceryStore Eskimo pies
x Completed task +nice
";

    #[test]
    fn test_parsing() {
        let list = TodoList::from_str(TEST_INPUT);
        assert!(dbg!(&list).is_ok());
        let list = list.unwrap();
        print!("{}", list);
        assert_eq!(list, list.to_string().parse().unwrap());
    }

    #[test]
    fn test_filter_basic() {
        let list = TodoList::from_str(TEST_INPUT).unwrap();
        assert_eq!(dbg!(list.filtered("prio:A", true)).len(), 1);
        assert_eq!(dbg!(list.filtered("prio:B", true)).len(), 2);
        assert_eq!(dbg!(list.filtered("prio:", true)).len(), 2);
        assert_eq!(dbg!(list.filtered("done:", true)).len(), 4);
        assert_eq!(dbg!(list.filtered("done:x", true)).len(), 2);
    }

    #[test]
    fn test_filter_words() {
        let list = TodoList::from_str(TEST_INPUT).unwrap();
        assert_eq!(dbg!(list.filtered("@phone", true)).len(), 2);
        assert_eq!(dbg!(list.filtered("@nix", true)).len(), 0);
        assert_eq!(dbg!(list.filtered("+GarageSale", true)).len(), 2);
        assert_eq!(dbg!(list.filtered("around Post", true)).len(), 1);
        assert_eq!(dbg!(list.filtered("the", true)).len(), 2);
    }

    #[test]
    fn test_filter_simple_combinations() {
        let list = TodoList::from_str(TEST_INPUT).unwrap();
        assert_eq!(dbg!(list.filtered("prio:A @phone done:", true)).len(), 1);
        assert_eq!(dbg!(list.filtered("done:x prio:", true)).len(), 1);
        assert_eq!(dbg!(list.filtered("+GarageSale prio:C", true)).len(), 0);
    }
}
