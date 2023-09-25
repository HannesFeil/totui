use std::io;

use ratatui::{
    prelude::{Constraint, CrosstermBackend, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, HighlightSpacing, List, ListItem, Paragraph},
    Frame,
};

use crate::{
    config::{Config, Item},
    todo::{self, Filter, TodoItem},
    FocusState, State,
};

/// Draw the application
pub fn draw(
    frame: &mut Frame<CrosstermBackend<io::Stdout>>,
    focus_state: &mut FocusState,
    state: &mut State,
    config: &Config,
) {
    // TODO: fix styling (default style)

    // Line + border
    let search_size = 3;

    // Line + border
    let hint_size = if config.show_help { 3 } else { 0 };

    // Vertical layouy search -> list
    let vertical = Layout::new()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(search_size),
            Constraint::Min(0),
            Constraint::Length(hint_size),
        ])
        .split(frame.size());

    let default_block = Block::new()
        .borders(Borders::all())
        .border_type(config.styles.border_type.into())
        .style(config.styles.border_style);

    let (filter_preamble_size, filter_line) =
        display_filter(state.todo_list.filter(), state.filter.value(), config);

    // Draw search
    // TODO: make better
    frame.render_widget(
        Paragraph::new(filter_line).block(default_block.clone()),
        vertical[0],
    );

    // TODO: switch to table?
    // Draw list
    frame.render_stateful_widget(
        List::new(
            state
                .todo_list
                .iter_filtered()
                .map(|(_, item)| ListItem::new(display_item(item, config)))
                .collect::<Vec<_>>(),
        )
        .block(default_block.clone())
        .highlight_symbol(config.styles.selection_symbol.as_str())
        .highlight_spacing(HighlightSpacing::Always)
        .style(config.styles.selection_symbol_style),
        vertical[1],
        &mut state.todo_list_view,
    );

    macro_rules! keys_help {
        ($($($bind:tt):+),+) => {
            {
                let mut spans = vec![];
                let sep_style = Style {
                    fg: config.styles.help_style.bg,
                    bg: config.styles.default_style.bg,
                    ..config.styles.help_style
                };
                $(
                    keys_help!(@push_help spans, sep_style, $($bind):+);
                )+
                Paragraph::new(Line {
                    spans,
                    alignment: None,
                }).block(default_block.clone())
            }
        };
        (@push_help $spans:expr, $sep_style:expr, $bind:ident) => {
            $spans.push(Span::styled(&config.styles.left_help_seperator_symbol, $sep_style));
            $spans.push(Span::styled(format!("{bind} {action}", bind = config.keys.$bind, action = stringify!($bind)), config.styles.help_style));
            $spans.push(Span::styled(&config.styles.right_help_seperator_symbol, $sep_style));
        };
        (@push_help $spans:expr, $sep_style:expr, $bind:literal : $action:literal) => {
            $spans.push(Span::styled(&config.styles.left_help_seperator_symbol, $sep_style));
            $spans.push(Span::styled(format!("{bind} {action}", bind = $bind, action = $action), config.styles.help_style));
            $spans.push(Span::styled(&config.styles.right_help_seperator_symbol, $sep_style));
        };
    }

    // Draw help
    if config.show_help {
        frame.render_widget(
            match focus_state {
                FocusState::Browsing => {
                    keys_help!(
                        quit,
                        up,
                        down,
                        toggle_done,
                        edit_priority,
                        add,
                        edit,
                        filter,
                        clear_filter
                    )
                }
                FocusState::FilterTyping { .. } => keys_help!(
                    confirm,
                    cancel,
                    input_toggle_ignore_case,
                    input_toggle_done,
                    input_edit_priority
                ),
                FocusState::ItemTyping { .. } => {
                    keys_help!(confirm, cancel, input_toggle_done, input_edit_priority)
                }
                FocusState::PriorityPicking {
                    previous_state,
                    view,
                } => keys_help!(confirm, cancel),
            },
            vertical[2],
        );
    }

    match focus_state {
        FocusState::FilterTyping { popup, .. } => {
            let cursor_pos = (
                filter_preamble_size as u16 + 1 + state.filter.visual_cursor() as u16,
                search_size / 2,
            );
            // Draw popup
            if popup.visible() {
                let popup_rect = Rect::new(
                    cursor_pos.0,
                    cursor_pos.1 + 1,
                    config.completion_size.0,
                    config.completion_size.1.min(popup.options.len() as u16 + 2),
                );
                frame.render_widget(Clear, popup_rect);
                frame.render_stateful_widget(
                    List::new(
                        popup
                            .options
                            .iter()
                            .map(|item| {
                                Text::styled(
                                    item,
                                    if item.starts_with('@') {
                                        config.styles.item.context_style
                                    } else if item.starts_with('+') {
                                        config.styles.item.project_style
                                    } else {
                                        config.styles.default_style
                                    },
                                )
                            })
                            .map(ListItem::new)
                            .collect::<Vec<_>>(),
                    )
                    .block(default_block.clone())
                    .highlight_style(Style::new().bg(Color::DarkGray)),
                    popup_rect,
                    &mut popup.state,
                )
            }
            // Draw cursor
            frame.set_cursor(cursor_pos.0, cursor_pos.1);
        }
        FocusState::Browsing => {}
        FocusState::ItemTyping { input, item, .. } => {
            let popup_width = (0.7 * frame.size().width as f32) as u16;
            let popup_height = 3;
            let popup_rect = floating_rect(vertical[1], popup_width, popup_height);
            frame.render_widget(Clear, popup_rect);
            frame.render_widget(
                Paragraph::new(display_item(item, config)).block(default_block.clone()),
                popup_rect,
            );
            frame.set_cursor(
                popup_rect.x + 1 + 8 + input.visual_cursor() as u16,
                popup_rect.y + 1,
            );
        }
        FocusState::PriorityPicking {
            previous_state,
            view,
        } => todo!(),
    }
}

fn floating_rect(container: Rect, width: u16, height: u16) -> Rect {
    Rect::new(
        container.x + (container.width - width) / 2,
        container.y + (container.height - height) / 2,
        width,
        height,
    )
}

fn display_filter<'filter>(
    filter: &'filter Filter,
    words: &'filter str,
    config: &'filter Config,
) -> (usize, Line<'filter>) {
    let ignore_case = if filter.ignore_case {
        Span::styled(
            &config.styles.ignore_case_symbol,
            config.styles.ignore_case_style,
        )
    } else {
        Span::styled(
            &config.styles.sensitive_case_symbol,
            config.styles.sensitive_case_style,
        )
    };

    let completed = filter
        .completed
        .map(|c| {
            if c {
                Span::styled(
                    &config.styles.item.complete_symbol,
                    config.styles.item.complete_style,
                )
            } else {
                Span::styled(
                    &config.styles.item.incomplete_symbol,
                    config.styles.item.incomplete_style,
                )
            }
        })
        .unwrap_or(Span::styled(
            &config.styles.item.ignore_complete_symbol,
            config.styles.item.ignore_complete_style,
        ));

    let priority = filter
        .priority
        .map(|p| match p {
            None => Span::styled(
                &config.styles.item.no_priority_symbol,
                config.styles.item.no_priority_style,
            ),
            Some(p) => Span::styled(
                config
                    .styles
                    .item
                    .default_priority_symbol
                    .replace("%p", &p.0.to_string()),
                config.styles.item.default_priority_style,
            ),
        })
        .unwrap_or(Span::styled(
            &config.styles.item.ignore_priority_symbol,
            config.styles.item.ignore_priority_style,
        ));

    let mut spans = vec![ignore_case, completed, priority];
    let preamble = spans.iter().map(|s| s.width()).sum();

    let mut item = TodoItem::default();
    item.set_content(words.to_owned());
    spans.extend(item.iter_content_parts().map(|p| match p {
        todo::ContentPart::Normal(p) => Span::styled(p.to_owned(), config.styles.default_style),
        todo::ContentPart::Context(p) => {
            Span::styled(p.to_owned(), config.styles.item.context_style)
        }
        todo::ContentPart::Project(p) => {
            Span::styled(p.to_owned(), config.styles.item.project_style)
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

fn display_item<'item>(item: &'item TodoItem, config: &'item Config) -> Line<'item> {
    // TODO: Better display (integrate config)
    let mut spans = vec![];

    if item.completed() {
        spans.push("[󰸞] ".fg(Color::LightGreen))
    } else {
        spans.push("[ ] ".fg(Color::Gray));
    }

    spans.push(match item.priority {
        Some(prio) => Span::styled(format!("({}) ", prio.0), Style::new().red()),
        None => "    ".into(),
    });

    for part in item.iter_content_parts() {
        match part {
            todo::ContentPart::Normal(c) => spans.push(c.into()),
            todo::ContentPart::Project(c) => spans.push(c.fg(Color::Cyan)),
            todo::ContentPart::Context(c) => spans.push(c.fg(Color::Green)),
        }
    }

    Line {
        spans,
        alignment: None,
    }
}
