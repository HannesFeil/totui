use ratatui::{
    layout::{Constraint, Layout, Margin},
    text::{Span, Text},
    widgets::{Row, Table},
    Frame,
};

use crate::{
    app::App,
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

    const TABLE_WIDTHS: [Constraint; 3] = [
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(20),
    ];
    const NUM_ROWS: usize = TABLE_WIDTHS.len();

    let content_width = Layout::horizontal(TABLE_WIDTHS)
        .spacing(1)
        .areas::<NUM_ROWS>(mid.inner(Margin::new(1, 0)))[2]
        .width as usize;
    let items = app.todo_list.items();
    let rows = items
        .iter()
        .copied()
        .map(|item| render_item_row(item, content_width, &app.config));
    frame.render_stateful_widget(
        Table::new(rows, TABLE_WIDTHS).block(app.config.default_block()),
        mid,
        &mut app.state.todo_table_state,
    )
}

fn render_item_row<'a>(item: &'a TodoItem, max_width: usize, config: &'a Config) -> Row<'a> {
    let completion = if item.completion_date.is_some() {
        config.item_complete_mark()
    } else {
        config.item_incomplete_mark()
    };

    let priority = match item.priority() {
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
            Content::Rec { .. } | Content::Due(_) | Content::T(_) | Content::Pri(_) => continue,
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

    let t = item.t_date().map(|date| config.item_t_date(*date));
    let due = item.due_date().map(|date| config.item_due_date(*date));
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

    Row::new([completion, priority, content]).height(height)
}
