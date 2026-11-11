//! The sticker's schedule popup: start, end and how the schedule repeats.

use crate::repeat::{bit, is_last_of_month, nth_of_month, ordinal, Repeat, WEEKDAYS};
use crate::App;
use chrono::{Datelike, Duration, Local, NaiveDate, Timelike, Weekday};
use eframe::egui::{self, RichText};

pub struct ScheduleEdit {
    note: usize,
    start: NaiveDate,
    start_hm: (u32, u32),
    end: NaiveDate,
    end_hm: (u32, u32),
    /// Which date the month grid edits: 0 start, 1 end.
    field: usize,
    month: NaiveDate,
    /// 0 none, 1 daily, 2 weekly, 3 every other week, 4 monthly.
    kind: usize,
    days: u8,
    by_weekday: bool,
    nth: i8,
    weekday: Weekday,
}
impl ScheduleEdit {
    fn new(n: &crate::model::Note, note: usize, today: NaiveDate) -> Self {
        let start = n.date.unwrap_or(today);
        let end = start + Duration::days(n.duration.max(1) as i64 - 1);
        let end_time = n.end_time.unwrap_or_else(|| n.time + Duration::hours(1));
        let mut s = Self {
            note,
            start,
            start_hm: (n.time.hour(), n.time.minute()),
            end,
            end_hm: (end_time.hour(), end_time.minute()),
            field: 0,
            month: start.with_day(1).unwrap(),
            kind: 0,
            days: bit(start.weekday()),
            by_weekday: false,
            nth: nth_of_month(start).min(4),
            weekday: start.weekday(),
        };
        match n.repeat {
            Repeat::None => {}
            Repeat::Daily => s.kind = 1,
            Repeat::Weekly { every, days } => {
                s.kind = if every >= 2 { 3 } else { 2 };
                s.days = days;
            }
            Repeat::MonthlyDay => s.kind = 4,
            Repeat::MonthlyNth { nth, weekday } => {
                s.kind = 4;
                s.by_weekday = true;
                s.nth = nth;
                s.weekday = weekday;
            }
        }
        s
    }
    fn rule(&self) -> Repeat {
        match self.kind {
            1 => Repeat::Daily,
            2 | 3 => Repeat::Weekly {
                every: if self.kind == 3 { 2 } else { 1 },
                days: if self.days == 0 {
                    bit(self.start.weekday())
                } else {
                    self.days
                },
            },
            4 if self.by_weekday => Repeat::MonthlyNth {
                nth: self.nth,
                weekday: self.weekday,
            },
            4 => Repeat::MonthlyDay,
            _ => Repeat::None,
        }
    }
}
fn weekday_at(i: usize) -> Weekday {
    Weekday::try_from(i as u8).unwrap_or(Weekday::Mon)
}
fn date_time_row(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    date: NaiveDate,
    hm: &mut (u32, u32),
) -> bool {
    let mut picked = false;
    ui.horizontal(|ui| {
        let muted = crate::ui::t(ui).muted;
        ui.add_sized(
            [40., 26.],
            egui::Label::new(RichText::new(label).color(muted)),
        );
        picked = crate::ui::pill(
            ui,
            selected,
            &format!(
                "{}  ({})",
                date.format("%Y.%m.%d"),
                crate::ui::weekday(date)
            ),
        )
        .on_hover_text("달력에서 날짜 선택")
        .clicked();
        ui.add_space(6.);
        crate::ui::time_input(ui, label, &mut hm.0, &mut hm.1);
    });
    picked
}
impl App {
    pub fn open_schedule(&mut self, i: usize) {
        self.desktop.schedule = Some(ScheduleEdit::new(
            &self.data.notes[i],
            i,
            Local::now().date_naive(),
        ));
        self.desktop.focus_dialog = true;
    }
    pub fn schedule_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut s) = self.desktop.schedule.take() else {
            return;
        };
        if s.note >= self.data.notes.len() {
            return;
        }
        let mut open = true;
        let mut apply = false;
        let mut clear = false;
        let mut cancel = false;
        let title = self.data.notes[s.note].title.clone();
        let has_date = self.data.notes[s.note].date.is_some();
        crate::ui::dialog(
            ctx,
            "schedule_dialog",
            "일정 설정",
            [400., 700.],
            true,
            &mut open,
            |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            crate::icons::paint_icon(
                                ui.painter(),
                                egui::Rect::from_min_size(ui.cursor().min, egui::Vec2::splat(20.)),
                                crate::icons::Icon::Calendar,
                                crate::ui::ACCENT,
                            );
                            ui.add_space(26.);
                            ui.label(
                                RichText::new(if title.is_empty() {
                                    "새 메모"
                                } else {
                                    &title
                                })
                                .strong()
                                .size(15.),
                            );
                        });
                        ui.add_space(10.);
                        if date_time_row(ui, "시작", s.field == 0, s.start, &mut s.start_hm) {
                            s.field = 0;
                            s.month = s.start.with_day(1).unwrap();
                        }
                        if date_time_row(ui, "종료", s.field == 1, s.end, &mut s.end_hm) {
                            s.field = 1;
                            s.month = s.end.with_day(1).unwrap();
                        }
                        ui.add_space(8.);
                        let before = s.start;
                        if s.field == 0 {
                            crate::ui::month_grid(ui, &mut s.month, &mut s.start);
                            // Moving the start keeps the length of the schedule.
                            s.end += s.start - before;
                        } else {
                            crate::ui::month_grid(ui, &mut s.month, &mut s.end);
                            s.end = s.end.max(s.start);
                        }
                        if s.start != before {
                            s.days = bit(s.start.weekday());
                            s.weekday = s.start.weekday();
                            s.nth = if is_last_of_month(s.start) && nth_of_month(s.start) > 4 {
                                -1
                            } else {
                                nth_of_month(s.start).min(4)
                            };
                        }
                        ui.add_space(10.);
                        crate::ui::caption(ui, "반복");
                        ui.add_space(4.);
                        crate::ui::segmented(
                            ui,
                            &mut s.kind,
                            &["안 함", "매일", "매주", "격주", "매월"],
                        );
                        ui.add_space(6.);
                        if s.kind == 2 || s.kind == 3 {
                            ui.horizontal(|ui| {
                                for (i, name) in WEEKDAYS.iter().enumerate() {
                                    let b = 1u8 << i;
                                    if crate::ui::pill(ui, s.days & b != 0, name).clicked() {
                                        s.days ^= b;
                                        if s.days == 0 {
                                            s.days = b;
                                        }
                                    }
                                }
                            });
                        }
                        if s.kind == 4 {
                            let mut basis = usize::from(s.by_weekday);
                            let by_day = format!("매월 {}일", s.start.day());
                            let by_week = format!(
                                "{} 주 {}요일",
                                ordinal(s.nth),
                                WEEKDAYS[s.weekday.num_days_from_monday() as usize]
                            );
                            if crate::ui::segmented(ui, &mut basis, &[&by_day, &by_week]) {
                                s.by_weekday = basis == 1;
                            }
                            if s.by_weekday {
                                ui.add_space(6.);
                                ui.horizontal(|ui| {
                                    for nth in [1, 2, 3, 4, -1] {
                                        if crate::ui::pill(ui, s.nth == nth, ordinal(nth)).clicked()
                                        {
                                            s.nth = nth;
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    for (i, name) in WEEKDAYS.iter().enumerate() {
                                        let w = weekday_at(i);
                                        if crate::ui::pill(ui, s.weekday == w, name).clicked() {
                                            s.weekday = w;
                                        }
                                    }
                                });
                            }
                        }
                        ui.add_space(10.);
                        crate::ui::hint(
                            ui,
                            format!(
                                "{} · {:02}:{:02} – {:02}:{:02}{}",
                                s.rule().describe(s.start),
                                s.start_hm.0,
                                s.start_hm.1,
                                s.end_hm.0,
                                s.end_hm.1,
                                if s.end > s.start {
                                    format!(" · {}일간", (s.end - s.start).num_days() + 1)
                                } else {
                                    String::new()
                                }
                            ),
                        );
                        ui.add_space(8.);
                        ui.horizontal(|ui| {
                            if has_date && ui.button("일정 지우기").clicked() {
                                clear = true;
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    apply = crate::ui::primary(ui, "적용").clicked();
                                    cancel = ui.button("취소").clicked();
                                },
                            );
                        });
                    });
            },
        );
        let n = &mut self.data.notes[s.note];
        open &= !cancel;
        if apply {
            let at = |d: NaiveDate, hm: (u32, u32)| d.and_hms_opt(hm.0, hm.1, 0);
            if let (Some(first), Some(last)) = (at(s.start, s.start_hm), at(s.end, s.end_hm)) {
                n.set_span(first, last);
            }
            n.repeat = s.rule();
            n.recurring = !n.repeat.is_none();
            open = false;
        }
        if clear {
            n.date = None;
            n.duration = 1;
            n.end_time = None;
            n.repeat = Repeat::None;
            n.recurring = false;
            open = false;
        }
        if open {
            self.desktop.schedule = Some(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editing_round_trips_the_rule_of_a_note() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
        let mut n = crate::model::Note::new("회의", Some(today), 0);
        n.repeat = Repeat::MonthlyNth {
            nth: 2,
            weekday: Weekday::Wed,
        };
        let s = ScheduleEdit::new(&n, 0, today);
        assert_eq!(s.kind, 4);
        assert_eq!(s.rule(), n.repeat);
        n.repeat = Repeat::Weekly {
            every: 2,
            days: bit(Weekday::Thu),
        };
        let s = ScheduleEdit::new(&n, 0, today);
        assert_eq!(s.kind, 3);
        assert_eq!(s.rule(), n.repeat);
    }
}
