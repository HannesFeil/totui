use std::io;

use ratatui::{
    prelude::{Constraint, CrosstermBackend, Direction, Layout},
    text::Text,
    widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph},
    Frame,
};
use tui_input::Input;

use crate::{config::Config, todo::TodoItem, FocusState};

/// Draw the application
pub fn draw(
    frame: &mut Frame<CrosstermBackend<io::Stdout>>,
    filtered: &[&TodoItem],
    list_state: &mut ListState,
    focus_state: &FocusState,
    filter: &Input,
    config: &Config,
) {
    // Line + border
    let search_size = 3;

    // Vertical layouy search -> list
    let vertical = Layout::new()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(search_size), Constraint::Min(0)])
        .split(frame.size());

    // Draw search
    frame.render_widget(
        Paragraph::new(Text::styled(filter.value(), config.styles.default_style)).block(
            Block::new()
                .borders(Borders::all())
                .border_type(config.styles.border_type.into())
                .style(config.styles.border_style),
        ),
        vertical[0],
    );

    // Draw list
    frame.render_stateful_widget(
        List::new(
            filtered
                .iter()
                .map(|item| display_item(item))
                .collect::<Vec<_>>(),
        )
        .block(
            Block::new()
                .borders(Borders::all())
                .border_type(config.styles.border_type.into())
                .style(config.styles.border_style),
        )
        .highlight_symbol(config.styles.selection_symbol.as_str())
        .highlight_spacing(HighlightSpacing::Always)
        .style(config.styles.selection_symbol_style),
        vertical[1],
        list_state,
    );

    match focus_state {
        // Draw cursor
        FocusState::FilterTyping { .. } => {
            frame.set_cursor(1 + filter.visual_cursor() as u16, search_size / 2)
        }
        FocusState::Browsing => {}
    }
}

fn display_item(item: &TodoItem) -> ListItem {
    ListItem::new(Text::raw(item.to_string()))
}
