use std::io;

use ratatui::{
    prelude::{Constraint, CrosstermBackend, Direction, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::{config::Config, State};

pub fn draw(frame: &mut Frame<CrosstermBackend<io::Stdout>>, state: &mut State, config: &Config) {
    // Line + border
    let search_size = 3;

    let vertical = Layout::new()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(search_size), Constraint::Min(0)])
        .split(frame.size());

    frame.render_widget(
        Paragraph::new(Text::styled(
            "Tets paragraph",
            Style::new().fg(Color::Green),
        ))
        .block(
            Block::new()
                .borders(Borders::all())
                .border_type(config.styles.border_type.into())
                .style(config.styles.border_style),
        ),
        vertical[0],
    );

    frame.render_stateful_widget(
        List::new(
            state
                .todo_list
                .as_ref()
                .iter()
                .map(ListItem::new)
                .collect::<Vec<_>>(),
        )
        .block(
            Block::new()
                .borders(Borders::all())
                .border_type(config.styles.border_type.into())
                .style(config.styles.border_style),
        ),
        vertical[1],
        &mut state.todo_list_state,
    );
}
