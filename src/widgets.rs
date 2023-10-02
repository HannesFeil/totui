use chrono::{Datelike, Days, Duration, Local, NaiveDate};
use ratatui::{
    prelude::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, StatefulWidget, Widget},
};

use crate::todo::{Recurring, RecurringUnit};

pub struct ScrollBar {
    pub pos: usize,
    pub total: usize,
    pub view: usize,
}

impl Widget for ScrollBar {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        if self.pos == 0 && self.total <= self.view {
            return;
        }

        let possible_scroll = (self.total - self.view) as u16;
        let bar_height = (area.height - 2).saturating_sub(possible_scroll).max(1);
        let bar_pos = (self.pos as f32 / possible_scroll as f32
            * (area.height - 2 - bar_height) as f32) as u16;

        for i in 1..area.height - 1 {
            buf.get_mut(area.right() - 1, area.y + i).set_char(
                if i > bar_pos && i < bar_pos + 1 + bar_height {
                    '▊'
                } else {
                    '║'
                },
            );
        }
    }
}

pub struct RecurrencePicker {
    pub normal_style: Style,
    pub arrow_style: Style,
    pub selection_style: Style,
}

#[derive(Debug, Clone)]
pub struct RecurrencePickerState {
    rec: Recurring,
    selected: u8,
}

impl RecurrencePickerState {
    pub fn new(recurring: Option<Recurring>) -> Self {
        Self {
            rec: recurring.unwrap_or(Recurring {
                relative: false,
                amount: 0,
                unit: RecurringUnit::Days,
            }),
            selected: 0,
        }
    }

    pub fn size(&self) -> (u16, u16) {
        (3 + self.rec.amount.to_string().len() as u16 + 3, 3)
    }

    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1) % 3
    }

    pub fn select_previous(&mut self) {
        self.selected = (self.selected + 2) % 3
    }

    pub fn increase(&mut self) {
        match self.selected {
            0 => self.rec.relative = true,
            1 => self.rec.amount += 1,
            2 => {
                self.rec.unit = match self.rec.unit {
                    RecurringUnit::Days => RecurringUnit::Weeks,
                    RecurringUnit::Weeks => RecurringUnit::Months,
                    RecurringUnit::Months => RecurringUnit::Years,
                    RecurringUnit::Years => RecurringUnit::Years,
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn decrease(&mut self) {
        match self.selected {
            0 => self.rec.relative = false,
            1 => self.rec.amount = self.rec.amount.saturating_sub(1),
            2 => {
                self.rec.unit = match self.rec.unit {
                    RecurringUnit::Days => RecurringUnit::Days,
                    RecurringUnit::Weeks => RecurringUnit::Days,
                    RecurringUnit::Months => RecurringUnit::Weeks,
                    RecurringUnit::Years => RecurringUnit::Months,
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn get_recurrence(&self) -> Option<Recurring> {
        (self.rec.amount != 0).then_some(self.rec)
    }

    pub fn reset(&mut self) {
        self.rec = Recurring {
            relative: false,
            amount: 0,
            unit: RecurringUnit::Days,
        }
    }
}

impl StatefulWidget for RecurrencePicker {
    type State = RecurrencePickerState;

    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &mut Self::State) {
        const UP: char = '▲';
        const DOWN: char = '▼';

        if state.rec.relative {
            buf.get_mut(area.x + 1, area.y + 1)
                .set_char('+')
                .set_style(if state.selected == 0 {
                    self.normal_style.patch(self.selection_style)
                } else {
                    self.normal_style
                });
            buf.get_mut(area.x + 1, area.y + 2)
                .set_char(DOWN)
                .set_style(self.arrow_style);
        } else {
            buf.get_mut(area.x + 1, area.y + 1)
                .set_char(' ')
                .set_style(if state.selected == 0 {
                    self.normal_style.patch(self.selection_style)
                } else {
                    self.normal_style
                });
            buf.get_mut(area.x + 1, area.y)
                .set_char(UP)
                .set_style(self.arrow_style);
        }

        let num = state.rec.amount.to_string();

        buf.set_string(
            area.x + 3,
            area.y + 1,
            &num,
            if state.selected == 1 {
                self.normal_style.patch(self.selection_style)
            } else {
                self.normal_style
            },
        );

        if state.rec.amount > 0 {
            buf.get_mut(area.x + 3, area.y + 2)
                .set_char(DOWN)
                .set_style(self.arrow_style);
        }
        buf.get_mut(area.x + 3, area.y)
            .set_char(UP)
            .set_style(self.arrow_style);

        buf.set_string(
            area.x + 4 + num.len() as u16,
            area.y + 1,
            match state.rec.unit {
                RecurringUnit::Days => "d",
                RecurringUnit::Weeks => "w",
                RecurringUnit::Months => "m",
                RecurringUnit::Years => "y",
            },
            if state.selected == 2 {
                self.normal_style.patch(self.selection_style)
            } else {
                self.normal_style
            },
        );

        if !matches!(state.rec.unit, RecurringUnit::Days) {
            buf.get_mut(area.x + 4 + num.len() as u16, area.y + 2)
                .set_char(DOWN)
                .set_style(self.arrow_style);
        }

        if !matches!(state.rec.unit, RecurringUnit::Years) {
            buf.get_mut(area.x + 4 + num.len() as u16, area.y)
                .set_char(UP)
                .set_style(self.arrow_style);
        }
    }
}

pub struct CalendarPicker {
    pub title_style: Style,
    pub line_style: Style,
    pub week_day_style: Style,
    pub line_type: BorderType,
    pub normal_style: Style,
    pub selection_style: Style,
    pub locked_in_style: Style,
    pub today_style: Style,
}

impl CalendarPicker {
    pub fn size() -> (u16, u16) {
        (1 + 3 * 7, 6 + 3)
    }
}

#[derive(Debug, Clone)]
pub struct CalendarPicerState {
    current: NaiveDate,
    selected: NaiveDate,
    locked_in: Option<NaiveDate>,
}

impl CalendarPicerState {
    pub fn new(selected: Option<NaiveDate>) -> CalendarPicerState {
        let current = Local::now().date_naive();
        Self {
            current,
            selected: selected.unwrap_or(current),
            locked_in: selected,
        }
    }

    pub fn locked(&self) -> Option<NaiveDate> {
        self.locked_in
    }

    pub fn select_today(&mut self) {
        self.selected = self.current;
    }

    pub fn select_locked(&mut self) {
        if let Some(locked) = self.locked_in {
            self.selected = locked;
        }
    }

    pub fn lock_selected(&mut self) {
        self.locked_in = Some(self.selected);
    }

    pub fn clear_locked(&mut self) {
        self.locked_in = None;
    }

    pub fn select_next(&mut self) {
        self.selected = self
            .selected
            .succ_opt()
            .expect("Should not be an extreme date");
    }

    pub fn select_previous(&mut self) {
        self.selected = self
            .selected
            .pred_opt()
            .expect("Should not be an extreme date");
    }

    pub fn select_previous_week(&mut self) {
        self.selected = self
            .selected
            .checked_sub_signed(Duration::weeks(1))
            .expect("Should not be an extreme date");
    }

    pub fn select_next_week(&mut self) {
        self.selected = self
            .selected
            .checked_add_signed(Duration::weeks(1))
            .expect("Should not be an extreme date");
    }
}

impl StatefulWidget for CalendarPicker {
    type State = CalendarPicerState;

    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &mut Self::State) {
        let month = state.selected.month0();
        let last_day = NaiveDate::from_ymd_opt(
            state
                .selected
                .year_ce()
                .1
                .try_into()
                .expect("Should not be an extreme case"),
            (month + 1) % 12 + 1,
            1,
        )
        .expect("Should not be an extreme date")
        .pred_opt()
        .expect("Should not be an extreme date")
        .day0() as u16;

        let first_weekday = state
            .selected
            .checked_sub_days(Days::new(state.selected.day0().into()))
            .unwrap()
            .weekday()
            .num_days_from_monday() as u16;

        let line = Line {
            spans: vec![Span::styled(
                state.selected.format("%B").to_string(),
                self.title_style,
            )],
            alignment: Some(Alignment::Center),
        };

        buf.set_line(
            area.x + (area.width - line.width() as u16) / 2,
            area.y,
            &line,
            line.width() as u16,
        );
        Block::new()
            .style(self.line_style)
            .border_type(self.line_type)
            .borders(Borders::BOTTOM)
            .render(Rect::new(area.x, area.y, area.width, 2), buf);
        let week_days = Line {
            spans: vec![
                " ".into(),
                Span::styled("Mo", self.week_day_style),
                " ".into(),
                Span::styled("Tu", self.week_day_style),
                " ".into(),
                Span::styled("We", self.week_day_style),
                " ".into(),
                Span::styled("Th", self.week_day_style),
                " ".into(),
                Span::styled("Fr", self.week_day_style),
                " ".into(),
                Span::styled("Sa", self.week_day_style),
                " ".into(),
                Span::styled("Su", self.week_day_style),
            ],
            alignment: None,
        };
        buf.set_line(area.x, area.y + 2, &week_days, week_days.width() as u16);

        for day in 0..=last_day {
            let y = (first_weekday + day) / 7;
            let x = 1 + ((first_weekday + day) % 7) * 3;

            let mut style = if state.locked_in.is_some()
                && day == state.locked_in.unwrap().day0() as u16
                && state.selected.month0() == state.locked_in.unwrap().month0()
                && state.selected.year_ce() == state.locked_in.unwrap().year_ce()
            {
                self.locked_in_style
            } else if day == state.current.day0() as u16
                && state.current.month0() == state.selected.month0()
                && state.current.year_ce() == state.selected.year_ce()
            {
                self.today_style
            } else {
                self.normal_style
            };

            if day == state.selected.day0() as u16 {
                style = style.patch(self.selection_style);
            }

            buf.set_string(
                area.x + x,
                area.y + 3 + y,
                format!("{day:>2}", day = day + 1),
                style,
            );
        }
    }
}
