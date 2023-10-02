use std::io;

use chrono::{Local, NaiveDate};
use ratatui::{
    prelude::{Constraint, CrosstermBackend, Direction, Layout, Margin, Rect, Style},
    style::{Color, Modifier},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, HighlightSpacing, List, ListItem, Padding, Paragraph},
    Frame,
};
use tui_input::Input;

use crate::{
    config::Config,
    todo::{self, Filter, TodoItem},
    widgets::{CalendarPicker, RecurrencePicker, ScrollBar},
    CompletionPopup, FocusState, State,
};

/// Draw the application
pub fn draw(
    frame: &mut Frame<CrosstermBackend<io::Stdout>>,
    focus_state: &mut FocusState,
    state: &mut State,
    config: &Config,
) {
    // Line + border
    const FILTER_SIZE: u16 = 3;

    // Line + border
    const HINT_SIZE: u16 = 3;

    // Vertical layouy search -> list -> help
    let vertical = Layout::new()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(FILTER_SIZE),
            Constraint::Min(0),
            Constraint::Length(if config.general.show_help {
                HINT_SIZE
            } else {
                0
            }),
        ])
        .split(frame.size());

    // default block around everything
    let default_block = Block::new()
        .borders(Borders::all())
        .border_type(config.display.border_type)
        .style(config.display.border_style);

    let (filter_preamble_size, filter_line) =
        display_filter(state.todo_list.filter(), state.filter.value(), config);

    // Draw filter
    frame.render_widget(
        Paragraph::new(filter_line.clone())
            .block(default_block.clone().padding(Padding::horizontal(1))),
        vertical[0],
    );

    // Draw list
    frame.render_stateful_widget(
        List::new(
            state
                .todo_list
                .iter_filtered()
                .map(|(_, item)| ListItem::new(display_item(item, config, false).1))
                .collect::<Vec<_>>(),
        )
        .block(default_block.clone())
        .highlight_symbol(config.display.selection_text.as_str())
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_style(config.display.selection_style),
        vertical[1],
        &mut state.todo_list_view,
    );

    frame.render_widget(
        ScrollBar {
            pos: state.todo_list_view.offset(),
            total: state.todo_list.filter_count(),
            view: vertical[1].height as usize - 2,
        },
        vertical[1],
    );

    // Display the helpline at the bottom
    macro_rules! keys_help {
        ($($($bind:tt):+),+) => {
            {
                let mut spans = vec![];
                let sep_style = Style {
                    fg: config.display.help_style.bg,
                    bg: config.display.default_style.bg,
                    ..config.display.help_style
                };
                $(
                    spans.push(Span::styled("", sep_style));
                    keys_help!(@push_help spans, sep_style, $($bind):+);
                    spans.push(Span::styled("", sep_style));
                    spans.push(Span::styled(&config.display.seperator_text, config.display.seperator_style));
                )+
                Paragraph::new(Line {
                    spans,
                    alignment: None,
                }).block(default_block.clone())
            }
        };
        // TODO: refactor this
        (@push_help $spans:expr, $sep_style:expr, $bind:ident) => {
            $spans.push(Span::styled(format!("{bind} {action}", bind = config.keys.$bind, action = stringify!($bind)), config.display.help_style));
        };
        (@push_help $spans:expr, $sep_style:expr, $bind:ident : $action:literal) => {
            $spans.push(Span::styled(format!("{bind} {action}", bind = config.keys.$bind, action = $action), config.display.help_style));
        };
        (@push_help $spans:expr, $sep_style:expr, $bind:literal : $action:literal) => {
            $spans.push(Span::styled(format!("{bind} {action}", bind = $bind, action = $action), config.display.help_style));
        };
    }

    // Draw help
    if config.general.show_help {
        frame.render_widget(
            match focus_state {
                FocusState::Browsing => {
                    keys_help!(
                        quit,
                        up,
                        down,
                        toggle_done: "completion",
                        edit_priority: "priority",
                        edit_threshhold: "threshhold",
                        edit_recurrence: "repeat",
                        edit_due: "due",
                        add,
                        edit,
                        remove,
                        remove_completed: "remove completed",
                        filter,
                        clear_filter: "reset filter"
                    )
                }
                FocusState::FilterTyping { .. } => keys_help!(
                    confirm,
                    cancel,
                    typing_toggle_ignore_case: "toggle ignore case",
                    typing_toggle_done: "completion",
                    typing_edit_priority: "priority"
                ),
                FocusState::ItemTyping { .. } => {
                    keys_help!(
                        confirm, 
                        cancel, 
                        typing_toggle_done: "completion", 
                        typing_edit_priority: "priority", 
                        typing_edit_threshhold: "threshhold", 
                        typing_edit_due: "due", 
                        typing_edit_recurrence: "repeat"
                    )
                }
                FocusState::PriorityPicking { .. } => keys_help!(confirm, cancel, up, down, "Space": "select none", "a-z": "select A-Z"),
                FocusState::DatePicking { .. } => keys_help!(
                    confirm, 
                    cancel, 
                    up, 
                    down, 
                    left, 
                    right, 
                    "C": "Clear selection", 
                    "Space": "Select date", 
                    "T": "Jump to today", 
                    "S": "Jump to selection"),
                FocusState::RecurrencePicking { .. } => keys_help!(confirm, cancel, up, down, left, right, "C": "Clear"),
                FocusState::Exiting => unreachable!()
            },
            vertical[2],
        );
    }

    // Draw specific to focus_state elements
    match focus_state {
        FocusState::FilterTyping { popup, .. } => {
            let cursor_pos = (
                filter_preamble_size as u16 + 2 + state.filter.visual_cursor() as u16,
                FILTER_SIZE / 2,
            );
            // Draw completion popup
            render_completion_popup(
                frame,
                popup,
                cursor_pos.0,
                cursor_pos.1,
                default_block.clone(),
                config,
            );
            // Draw cursor
            frame.set_cursor(cursor_pos.0, cursor_pos.1);
        }
        FocusState::Browsing => {}
        FocusState::ItemTyping {
            input, item, popup, ..
        } => {
            // Render input field
            let (_, y, cursor_x) = render_item_typing(
                frame,
                vertical[1],
                input,
                item,
                popup,
                default_block.clone(),
                config,
            );
            // Draw cursor
            frame.set_cursor(cursor_x, y + 1);
        }
        FocusState::PriorityPicking {
            previous_state,
            view,
        } => {
            let mut min_width = 3;
            // Start with no priority
            let mut priorities = vec![ListItem::new(Span::styled(
                &config.display.no_priority_text,
                config.display.no_priority_style,
            ))];
            // Add priorities
            priorities.extend(('A'..='Z').map(|p| {
                let (text, style) = config.priority_look(&p.to_string());
                let span = Span::styled(text, style);
                min_width = min_width.max(span.width());
                ListItem::new(span)
            }));

            let selection_width = Text::raw(&config.display.selection_text).width() as u16;
            // selection_text + min_width + padding + border
            let popup_width = min_width as u16 + selection_width + 3;
            let popup_height = config.general.completion_window_height + 2;

            // change popup position depending on previous focus state (e.g. filter or editing etc.)
            let popup_rect = match &mut **previous_state {
                FocusState::Browsing => {
                    let selected = state
                        .todo_list
                        .iter_filtered()
                        .nth(state.todo_list_view.selected().unwrap())
                        .unwrap();
                    // offset for the priority
                    let x = display_item(selected.1, config, false)
                        .1
                        .spans
                        .iter()
                        .take(2)
                        .map(|span| span.width() as u16)
                        .sum::<u16>();
                    // calculate real position of selected entry: selection - offset + filter area
                    // + top list border + one line below
                    let y = (state.todo_list_view.selected().unwrap()
                        - state.todo_list_view.offset()) as u16
                        + FILTER_SIZE
                        + 2;

                    rect_in_bounds(frame.size(), x, y, popup_width, popup_height)
                }
                FocusState::FilterTyping { .. } => {
                    // calculate priority offset: offset + border - selection_text
                    let x = filter_line
                        .spans
                        .iter()
                        .take(4)
                        .map(|span| span.width() as u16)
                        .sum::<u16>()
                        + 1
                        - selection_width;
                    let y = 2;

                    rect_in_bounds(frame.size(), x, y, popup_width, popup_height)
                }
                FocusState::ItemTyping {
                    input, item, popup, ..
                } => {
                    let (px, py, _) = render_item_typing(
                        frame,
                        vertical[1],
                        input,
                        item,
                        popup,
                        default_block.clone(),
                        config,
                    );
                    let x = px - selection_width
                        + display_item(item, config, false)
                            .1
                            .spans
                            .iter()
                            .take(2)
                            .map(|span| span.width() as u16)
                            .sum::<u16>();
                    let y = py + 2;

                    rect_in_bounds(frame.size(), x, y, popup_width, popup_height)
                }
                _ => {
                    unreachable!()
                }
            };

            frame.render_widget(Clear, popup_rect);
            frame.render_stateful_widget(
                List::new(priorities)
                    .block(default_block.clone())
                    .highlight_symbol(&config.display.selection_text)
                    .highlight_style(config.display.selection_style),
                popup_rect,
                view,
            );
            frame.render_widget(
                ScrollBar {
                    pos: view.offset(),
                    total: 27,
                    view: popup_rect.height as usize - 2,
                },
                popup_rect,
            );
        }
        FocusState::DatePicking {
            calendar_view,
            date,
            previous_state,
        } => {
            if let FocusState::ItemTyping {
                input, item, popup, ..
            } = &mut **previous_state
            {
                render_item_typing(
                    frame,
                    vertical[1],
                    input,
                    item,
                    popup,
                    default_block.clone(),
                    config,
                );
            }

            let popup_rect = floating_rect(
                vertical[1],
                CalendarPicker::size().0 + 2,
                CalendarPicker::size().1 + 2,
            );
            frame.render_widget(Clear, popup_rect);
            frame.render_widget(default_block.clone(), popup_rect);
            frame.render_stateful_widget(
                CalendarPicker {
                    title_style: config.display.default_style.add_modifier(Modifier::BOLD),
                    line_style: config.display.border_style,
                    line_type: config.display.border_type,
                    week_day_style: config.display.recurrence_style,
                    normal_style: config.display.default_style.fg(Color::Gray),
                    selection_style: config.display.completed_style.add_modifier(Modifier::BOLD),
                    locked_in_style: match date {
                        crate::EditDate::Due => {
                            config.display.due_style.add_modifier(Modifier::REVERSED)
                        }
                        crate::EditDate::Threshhold => config
                            .display
                            .threshhold_style
                            .add_modifier(Modifier::REVERSED),
                    },
                    today_style: config
                        .display
                        .default_priority_style
                        .add_modifier(Modifier::REVERSED),
                },
                popup_rect.inner(&Margin {
                    horizontal: 1,
                    vertical: 1,
                }),
                calendar_view,
            );
        }
        FocusState::RecurrencePicking {
            previous_state,
            picker_state,
        } => {
            if let FocusState::ItemTyping {
                input, item, popup, ..
            } = &mut **previous_state
            {
                render_item_typing(
                    frame,
                    vertical[1],
                    input,
                    item,
                    popup,
                    default_block.clone(),
                    config,
                );
            }

            let popup_rect = floating_rect(
                vertical[1],
                picker_state.size().0 + 2,
                picker_state.size().1 + 2,
            );
            frame.render_widget(Clear, popup_rect);
            frame.render_widget(default_block.clone(), popup_rect);
            frame.render_stateful_widget(
                RecurrencePicker {
                    normal_style: config.display.recurrence_style,
                    selection_style: config.display.selection_style,
                    arrow_style: config.display.default_style,
                },
                popup_rect.inner(&Margin {
                    horizontal: 1,
                    vertical: 1,
                }),
                picker_state,
            );
        }
        FocusState::Exiting => unreachable!(),
    }
}

/// Render a floating item editing window
///
/// # Return
/// The x and y coordinate of the top left corner and the global cursor position
fn render_item_typing(
    frame: &mut Frame<CrosstermBackend<io::Stdout>>,
    rect: Rect,
    input: &Input,
    item: &TodoItem,
    popup: &mut CompletionPopup,
    block: Block,
    config: &Config,
) -> (u16, u16, u16) {
    let popup_width = (0.7 * frame.size().width as f32) as u16;
    let popup_height = 3;
    let popup_rect = floating_rect(rect, popup_width, popup_height);
    let (preamble_size, item_line) = display_item(item, config, true);
    frame.render_widget(Clear, popup_rect);
    frame.render_widget(Paragraph::new(item_line).block(block.clone()), popup_rect);

    let cursor_pos = (
        popup_rect.x + 1 + preamble_size as u16 + input.visual_cursor() as u16,
        popup_rect.y + 1,
    );

    render_completion_popup(frame, popup, cursor_pos.0, cursor_pos.1, block, config);

    (popup_rect.x, popup_rect.y, cursor_pos.0)
}

/// Render a completion window popup at the given cursor position
fn render_completion_popup(
    frame: &mut Frame<CrosstermBackend<io::Stdout>>,
    popup: &mut CompletionPopup,
    cursor_x: u16,
    cursor_y: u16,
    default_block: Block,
    config: &Config,
) {
    if popup.visible() {
        let mut min_width = 0;
        let mut vals = popup
            .options
            .iter()
            .map(|item| {
                let (text, style) = if item.starts_with('@') {
                    config.context_look(item)
                } else if item.starts_with('+') {
                    config.project_look(item)
                } else {
                    (item.as_str(), Style::new())
                };
                (text, style)
            })
            .collect::<Vec<_>>();
        vals.sort_unstable_by_key(|v| v.0.to_lowercase());
        let total = vals.len();
        let list = List::new(
            vals.into_iter()
                .map(|(text, style)| Span::styled(text, style))
                .inspect(|t| min_width = min_width.max(t.width()))
                .map(ListItem::new)
                .collect::<Vec<_>>(),
        )
        .block(default_block.padding(Padding::horizontal(1)))
        .style(config.display.default_style)
        .highlight_style(config.display.selection_style);

        let popup_rect = rect_in_bounds(
            frame.size(),
            cursor_x,
            cursor_y + 1,
            config.general.completion_window_width.min(min_width as u16) + 4,
            config
                .general
                .completion_window_height
                .min(popup.options.len() as u16)
                + 2,
        );

        frame.render_widget(Clear, popup_rect);
        frame.render_stateful_widget(list, popup_rect, &mut popup.state);
        frame.render_widget(
            ScrollBar {
                pos: popup.state.offset(),
                total,
                view: popup_rect.height as usize - 2,
            },
            popup_rect,
        );
    }
}

/// Return a `Rect` with its top left corner anchored at `(x, y)`
/// If the rect wouldn't fit inside `container`, anchors the rect on a different corner
fn rect_in_bounds(container: Rect, mut x: u16, mut y: u16, width: u16, height: u16) -> Rect {
    if x + width > container.x + container.width {
        x = container.x.max(x.saturating_sub(width - 1));
    }

    if y + height > container.y + container.height {
        y = container.y.max(y.saturating_sub(height + 1));
    }

    Rect::new(x, y, width, height)
}

/// Return a `Rect` with the given `width` and `height`, centered in `container`
fn floating_rect(container: Rect, width: u16, height: u16) -> Rect {
    Rect::new(
        container.x + (container.width - width) / 2,
        container.y + (container.height - height) / 2,
        width,
        height,
    )
}

/// Display the filter line
fn display_filter<'filter>(
    filter: &'filter Filter,
    words: &'filter str,
    config: &'filter Config,
) -> (usize, Line<'filter>) {
    let sep = Span::styled(
        &config.display.seperator_text,
        config.display.seperator_style,
    );

    let mut spans = vec![];

    let ignore_case = if filter.ignore_case {
        Span::styled(
            &config.display.ignore_case_text,
            config.display.ignore_case_style,
        )
    } else {
        Span::styled(
            &config.display.sensitive_case_text,
            config.display.sensitive_case_style,
        )
    };

    if ignore_case.width() != 0 {
        spans.push(ignore_case);
        spans.push(sep.clone());
    }

    let completed = filter
        .completed
        .map(|c| {
            if c {
                Span::styled(
                    &config.display.completed_text,
                    config.display.completed_style,
                )
            } else {
                Span::styled(
                    &config.display.uncompleted_text,
                    config.display.uncompleted_style,
                )
            }
        })
        .unwrap_or(Span::styled(
            &config.display.ignore_completed_text,
            config.display.ignore_completed_style,
        ));

    if completed.width() != 0 {
        spans.push(completed);
        spans.push(sep.clone());
    }

    let priority = filter
        .priority
        .map(|p| match p {
            None => Span::styled(
                &config.display.no_priority_text,
                config.display.no_priority_style,
            ),
            Some(p) => {
                let (text, style) = config.priority_look(&p.0.to_string());
                Span::styled(text, style)
            }
        })
        .unwrap_or(Span::styled(
            &config.display.ignore_priority_text,
            config.display.ignore_priority_style,
        ));

    if priority.width() != 0 {
        spans.push(priority);
        spans.push(sep);
    }

    let preamble = spans.iter().map(|s| s.width()).sum();

    let mut item = TodoItem::default();
    item.set_content(words.to_owned());
    spans.extend(item.iter_content_parts().map(|p| match p {
        todo::ContentPart::Normal(p) => Span::styled(p.to_owned(), config.display.default_style),
        todo::ContentPart::Context(p) => Span::styled(p.to_owned(), config.display.context_style),
        todo::ContentPart::Project(p) => Span::styled(p.to_owned(), config.display.project_style),
        todo::ContentPart::Recurrence(p)
        | todo::ContentPart::Due(p)
        | todo::ContentPart::Priority(p)
        | todo::ContentPart::Threshhold(p) => {
            Span::styled(p.to_owned(), config.display.default_style)
        }
    }));

    (
        preamble,
        Line {
            spans,
            alignment: None,
        },
    )
}

/// Displays the `item` in a line
fn display_item<'item>(
    item: &'item TodoItem,
    config: &'item Config,
    raw: bool,
) -> (usize, Line<'item>) {
    let mut spans = vec![];

    let sep = Span::styled(
        &config.display.seperator_text,
        config.display.seperator_style,
    );

    let completed = if item.completed() {
        Span::styled(
            &config.display.completed_text,
            config.display.completed_style,
        )
    } else {
        Span::styled(
            &config.display.uncompleted_text,
            config.display.uncompleted_style,
        )
    };

    if completed.width() != 0 {
        spans.push(completed);
        spans.push(sep.clone());
    }

    let priority = match item.priority {
        Some(prio) => {
            let (text, style) = config.priority_look(&prio.0.to_string());
            Span::styled(text, style)
        }
        None => Span::styled(
            &config.display.no_priority_text,
            config.display.no_priority_style,
        ),
    };

    if priority.width() != 0 {
        spans.push(priority);
        spans.push(sep.clone());
    }

    let preamble_size = spans.iter().map(|span| span.width()).sum();

    for part in item.iter_content_parts() {
        match part {
            todo::ContentPart::Normal(c) => {
                spans.push(Span::styled(c, config.display.default_style))
            }
            todo::ContentPart::Project(c) => {
                let (text, style) = config.project_look(c);
                spans.push(Span::styled(if raw { c } else { text }, style))
            }
            todo::ContentPart::Context(c) => {
                let (text, style) = config.context_look(c);
                spans.push(Span::styled(if raw { c } else { text }, style))
            }
            todo::ContentPart::Recurrence(c)
            | todo::ContentPart::Due(c)
            | todo::ContentPart::Priority(c)
            | todo::ContentPart::Threshhold(c) => {
                spans.push(Span::styled(c, config.display.error_style))
            }
        }
    }

    let now = Local::now().date_naive();

    if let Some(date) = item.threshhold {
        spans.push(sep.clone());
        spans.push(Span::styled(
            config
                .display
                .threshhold_text
                .replace("%d", &display_duration(now, date, config)),
            config.display.threshhold_style,
        ));
    }

    if let Some(date) = item.due {
        spans.push(sep.clone());
        spans.push(Span::styled(
            config
                .display
                .due_text
                .replace("%d", &display_duration(now, date, config)),
            config.display.due_style,
        ));
    }

    if let Some(rec) = item.recurring {
        spans.push(sep.clone());
        spans.push(Span::styled(
            config
                .display
                .recurrence_text
                .replace("%r", &rec.to_string()),
            config.display.recurrence_style,
        ));
    }

    (
        preamble_size,
        Line {
            spans,
            alignment: None,
        },
    )
}

/// Display duration used for threshhold and due dates
fn display_duration(now: NaiveDate, date: NaiveDate, config: &Config) -> String {
    let duration = date.signed_duration_since(now);
    match duration.num_days() {
        -6..=-2 => format!("Since {week_day}", week_day = date.format("%A")),
        -1 => "Yesterday".to_owned(),
        0 => "Today".to_owned(),
        1 => "Tomorrow".to_owned(),
        2..=6 => date.format("%A").to_string(),
        _ => match duration.num_weeks() {
            n @ -4..=-2 => format!("{x} weeks ago", x = -n),
            -1 => "A week ago".to_owned(),
            0 => unreachable!(),
            1 => "In a week".to_owned(),
            n @ 2..=4 => format!("In {x} weeks", x = n),
            _ => date.format(&config.display.date_format).to_string(),
        },
    }
}
