use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Paragraph, Row, Table},
    Frame,
};

use crate::{
    app::{App, FocusState, TodoListFilter},
    config::Config,
    todo::{Content, TodoItem},
};

/// Renders the user interface widgets.
pub fn render(app: &mut App, frame: &mut Frame) {
    // This is where you add new widgets.
    // See the following resources:
    let [top, mid, bot] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .areas(frame.size());

    render_sortfilter(
        frame,
        top,
        app.todo_list.filter(),
        &app.config,
        matches!(app.state, FocusState::FilterFocus {}),
    );

    const NUM_COLS: usize = 3;
    const MIN_CONTENT_WIDTH: u16 = 20;
    let table_widths: [Constraint; NUM_COLS] = [
        Constraint::Length(app.config.completion_width() as u16),
        Constraint::Length(app.config.priority_width() as u16),
        Constraint::Min(MIN_CONTENT_WIDTH),
    ];

    let content_width = Layout::horizontal(table_widths)
        .spacing(1)
        .areas::<NUM_COLS>(mid.inner(Margin::new(1, 0)))[2]
        .width as usize;
    let items = app.todo_list.items();
    let rows = items.map(|item| render_item_row(item, content_width, &app.config));
    frame.render_stateful_widget(
        Table::new(rows, table_widths)
            .highlight_symbol(app.config.item_selection_mark())
            .block(app.config.default_block()),
        mid,
        &mut *app.todo_list.table_state_mut(),
    )
}

fn render_sortfilter(
    frame: &mut Frame,
    area: Rect,
    filter: &TodoListFilter,
    config: &Config,
    focused: bool,
) {
    let completion = match filter.completion {
        Some(true) => config.item_complete_mark(),
        Some(false) => config.item_incomplete_mark(),
        None => config.filter_completion_disabled(),
    };
    let priority = match filter.priority {
        Some(Some(priority)) => config.item_priority_mark(priority),
        Some(None) => config.item_no_priority_mark(),
        None => config.filter_priority_disabled(),
    };
    let t = if filter.t {
        config.filter_t_enabled()
    } else {
        config.filter_t_disabled()
    };
    let input = filter.input_field.value();

    frame.render_widget(config.default_block(), area);
    let [completion_area, priority_area, t_area, input_area] = Layout::horizontal([
        Constraint::Length(config.completion_width() as u16),
        Constraint::Length(config.priority_width() as u16),
        Constraint::Length(config.t_width() as u16),
        Constraint::Min(10),
    ])
    .spacing(1)
    .areas(area.inner(Margin::new(1, 1)));
    frame.render_widget(Paragraph::new(completion), completion_area);
    frame.render_widget(Paragraph::new(priority), priority_area);
    frame.render_widget(Paragraph::new(t), t_area);
    frame.render_widget(Paragraph::new(input), input_area);

    if focused {
        frame.set_cursor(
            input_area.x + filter.input_field.visual_cursor() as u16,
            input_area.y,
        );
    }
}

fn render_item_row<'a>(item: &'a TodoItem, max_width: usize, config: &'a Config) -> Row<'a> {
    let completion = if item.completion_date.is_some() {
        config.item_complete_mark()
    } else {
        config.item_incomplete_mark()
    };

    let priority = match item.priority {
        Some(p) => config.item_priority_mark(p),
        None => config.item_no_priority_mark(),
    };

    let mut spans = vec![];
    let mut line_width = 0;
    let mut lines = vec![];
    let mut first = true;

    for part in item.content_parts() {
        let span = match &part.content {
            Content::Word(word) => config.item_word(word),
            Content::Context(context) => config.item_context(context),
            Content::Project(project) => config.item_project(project),
        };
        let space = if first {
            first = false;
            Span::raw("")
        } else {
            config.item_space(&part.space)
        };

        line_width += span.width();
        line_width += space.width();

        if line_width > max_width {
            line_width = span.width();
            lines.push(std::mem::take(&mut spans));
            spans.push(span);
        } else {
            spans.push(space);
            spans.push(span);
        }
    }

    let t = item.t.map(|date| config.item_t_date(date));
    let due = item.due.map(|date| config.item_due_date(date));
    for span in [t, due].into_iter().flatten() {
        line_width += span.width() + 1;

        if line_width > max_width {
            line_width = 0;
            lines.push(std::mem::take(&mut spans));
            spans.push(span);
        } else {
            spans.push(Span::raw(" "));
            spans.push(span);
        }
    }

    if !spans.is_empty() {
        lines.push(spans);
    }

    let content = Text::from_iter(lines);
    let height = content.height() as u16;

    Row::new([completion.into(), priority.into(), content]).height(height)
}
