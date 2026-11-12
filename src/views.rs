use crate::*;
use chrono::Duration;
use egui::{Rect, Sense, Stroke};
impl App {
    pub fn filters(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.;
            egui::ComboBox::from_id_salt(ui.id().with("projects"))
                .width(150.)
                .selected_text(format!(
                    "# {}",
                    crate::refinements::project_label(self.project_filter, &self.data.projects)
                ))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.project_filter, None, "모든 프로젝트");
                    ui.selectable_value(
                        &mut self.project_filter,
                        Some(NO_PROJECT),
                        "프로젝트 없음",
                    );
                    for (i, p) in self.data.projects.iter().enumerate() {
                        if self.extra.hidden_projects.contains(&i)
                            || self.extra.deleted_projects.contains(&i)
                        {
                            continue;
                        }
                        ui.selectable_value(&mut self.project_filter, Some(i), p);
                    }
                });
            if ui::pill(ui, self.recurring, "정기 일정 포함").clicked() {
                self.recurring = !self.recurring;
            }
            if ui::pill(ui, self.extra.created_basis, "생성일 기준").clicked() {
                self.extra.created_basis = !self.extra.created_basis;
            }
            if ui::pill(ui, self.extra.show_done, "완료 메모 보기").clicked() {
                self.extra.show_done = !self.extra.show_done;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui::primary(ui, "+ 새 일정").clicked() {
                    self.add_note(Some(match self.page {
                        2 => Local::now().date_naive(),
                        3 => Local::now().date_naive() + Duration::days(1),
                        _ => self.day,
                    }));
                }
            });
        });
    }
    pub fn task_row_on(&mut self, ui: &mut egui::Ui, i: usize, occurrence: Option<NaiveDate>) {
        let tone = ui::t(ui);
        let height = match self.desktop.row_spacing {
            0 => 40.,
            2 => 54.,
            _ => 46.,
        };
        // On 오늘의 할일 / 다음에 할일 a click shows the memo in the preview
        // pane; the eye button beside the check opens it as a sticker.
        let preview = matches!(self.page, 2 | 3);
        let lead = if preview { 26. } else { 0. };
        ui.push_id((i, occurrence), |ui| {
            let (rect, row) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), height),
                Sense::click_and_drag(),
            );
            let hovered = ui.rect_contains_pointer(rect);
            let selected = preview && self.catalog.selected == Some(i);
            if selected {
                ui.painter().rect_filled(rect, 6., tone.accent_soft);
            } else if hovered {
                ui.painter()
                    .rect_filled(rect, 6., tone.hover.gamma_multiply(0.6));
            }
            ui.painter().line_segment(
                [rect.left_bottom() + Vec2::new(40., 0.), rect.right_bottom()],
                Stroke::new(1.0_f32, tone.line),
            );
            row.dnd_set_drag_payload(NoteDrag);
            self.drag_source_on(ui, &row, i, occurrence);
            if hovered {
                for (dx, dy) in [(0., -4.), (0., 0.), (0., 4.), (4., -4.), (4., 0.), (4., 4.)] {
                    ui.painter().circle_filled(
                        egui::pos2(rect.left() + 8. + dx, rect.center().y + dy),
                        1.1,
                        tone.faint,
                    );
                }
            }
            let mut toggle_sticker = false;
            if preview {
                let open = self.studio.windows.get(&i).is_some_and(|w| w.open);
                let eye = Rect::from_center_size(
                    egui::pos2(rect.left() + 30., rect.top() + 15.),
                    Vec2::splat(24.),
                );
                toggle_sticker = ui
                    .put(eye, |ui: &mut egui::Ui| {
                        icons::toggle_icon(ui, icons::Icon::Eye, "스티커 표시 / 숨기기", open)
                    })
                    .clicked();
            }
            let mut d = self.data.notes[i].done_on(occurrence);
            let check = Rect::from_center_size(
                egui::pos2(rect.left() + 30. + lead, rect.top() + 15.),
                Vec2::splat(20.),
            );
            if ui
                .put(check, |ui: &mut egui::Ui| ui::round_check(ui, &mut d, None))
                .changed()
            {
                self.done_at(i, occurrence, d);
                self.extra.scope_owner = ui.ctx().viewport_id();
            }
            let n = &self.data.notes[i];
            let title = if n.title.trim().is_empty() {
                "제목 없음".to_owned()
            } else {
                n.title.clone()
            };
            let text_left = rect.left() + 48. + lead;
            let actions = if hovered || n.starred { 64. } else { 0. };
            let title_font = egui::TextStyle::Body.resolve(ui.style());
            let mut job = egui::text::LayoutJob::single_section(
                title,
                egui::TextFormat {
                    font_id: title_font,
                    color: if n.done_on(occurrence) {
                        tone.faint
                    } else {
                        tone.ink
                    },
                    strikethrough: if n.done_on(occurrence) {
                        Stroke::new(1.0_f32, tone.faint)
                    } else {
                        Stroke::NONE
                    },
                    ..Default::default()
                },
            );
            job.wrap = egui::text::TextWrapping::truncate_at_width(
                (rect.right() - text_left - actions - 8.).max(40.),
            );
            let galley = ui.fonts(|f| f.layout_job(job));
            ui.painter()
                .galley(egui::pos2(text_left, rect.top() + 6.), galley, tone.ink);
            let mut meta = vec![];
            if self.extra.created_basis {
                meta.push(format!("생성 {}", n.created.format("%m.%d %H:%M")));
            } else if let Some(d) = occurrence {
                meta.push(format!(
                    "{} {}",
                    d.format("%-m월 %-d일"),
                    n.time.format("%H:%M")
                ));
            }
            if n.recurring {
                meta.push("↻ 정기".into());
            }
            if let Some(t) = n.reminder {
                meta.push(format!("⏰ {}", t.format("%m/%d %H:%M")));
            }
            if let Some(p) = n.project {
                meta.push(format!("# {}", self.data.projects[p]));
            }
            if self.desktop.show_preview && self.split && !n.locked {
                let body = crate::composer::strip_pictures(&n.body).replace('\n', " ");
                if !body.trim().is_empty() {
                    meta.push(body);
                }
            }
            if !meta.is_empty() {
                let mut job = egui::text::LayoutJob::single_section(
                    meta.join("  ·  "),
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(12.),
                        color: tone.muted,
                        ..Default::default()
                    },
                );
                job.wrap = egui::text::TextWrapping::truncate_at_width(
                    (rect.right() - text_left - actions - 8.).max(40.),
                );
                let galley = ui.fonts(|f| f.layout_job(job));
                ui.painter().galley(
                    egui::pos2(text_left, rect.top() + height - 20.),
                    galley,
                    tone.muted,
                );
            }
            let mut toggled_star = false;
            let mut edit = false;
            if hovered || self.data.notes[i].starred {
                let mut x = rect.right() - 28.;
                if hovered {
                    let r =
                        Rect::from_min_size(egui::pos2(x, rect.center().y - 14.), Vec2::splat(28.));
                    edit = ui
                        .put(r, |ui: &mut egui::Ui| {
                            icons::icon_button(ui, icons::Icon::Edit, "편집창 열기")
                        })
                        .clicked();
                    x -= 30.;
                }
                let starred = self.data.notes[i].starred;
                let r = Rect::from_min_size(egui::pos2(x, rect.center().y - 14.), Vec2::splat(28.));
                toggled_star = ui
                    .put(r, |ui: &mut egui::Ui| {
                        icons::toggle_icon(
                            ui,
                            icons::Icon::Star,
                            if starred {
                                "중요 해제"
                            } else {
                                "중요 표시"
                            },
                            starred,
                        )
                    })
                    .clicked();
            }
            if toggled_star {
                let n = &mut self.data.notes[i];
                n.starred = !n.starred;
            }
            if toggle_sticker {
                self.toggle_sticker(i);
            } else if edit {
                self.open_editor_on(i, occurrence);
            } else if row.clicked() {
                if preview {
                    self.catalog.selected = Some(i);
                    self.catalog.selected_day = occurrence;
                } else {
                    self.open_note_on(i, occurrence);
                }
            }
            row.context_menu(|ui| {
                if ui.button("편집창 열기").clicked() {
                    self.open_editor_on(i, occurrence);
                    ui.close_menu();
                }
                if ui.button("스티커로 꺼내기").clicked() {
                    self.open_sticker(i, occurrence);
                    ui.close_menu();
                }
                if ui.button("알람 설정").clicked() {
                    self.open_reminder(i);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("휴지통으로 이동").clicked() {
                    self.trash_note(i);
                    ui.close_menu();
                }
            });
            if self.extra.search_mode {
                let d = if self.extra.created_basis {
                    Some(self.data.notes[i].created.date())
                } else {
                    self.data.notes[i].date
                };
                if let Some(d) = d {
                    ui.horizontal(|ui| {
                        ui.add_space(48.);
                        if ui
                            .link(RichText::new(format!("{d} · 달력에서 보기")).small())
                            .clicked()
                        {
                            self.month = d.with_day(1).unwrap();
                            self.day = d;
                            self.split = true;
                        }
                    });
                }
            }
        });
    }
    /// 오늘의 할일 / 다음에 할일: the memo list with the preview pane on the
    /// right, where the selected memo is read and edited.
    pub fn agenda_preview(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let height = rect.height();
        let preview_max = (rect.width() - 360.).max(260.);
        let preview_width = self
            .catalog
            .preview_width
            .max(380.)
            .clamp(260., preview_max);
        let list_rect = Rect::from_min_size(
            rect.min,
            Vec2::new((rect.width() - preview_width - 16.).max(200.), height),
        );
        let preview_rect = Rect::from_min_size(
            egui::pos2(list_rect.right() + 16., rect.top()),
            Vec2::new(preview_width, height),
        );
        let mut list = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("agenda_list_pane")
                .max_rect(list_rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        list.set_clip_rect(ui.clip_rect().intersect(list_rect));
        egui::ScrollArea::vertical()
            .id_salt(("page_scroll", self.page))
            .auto_shrink([false, false])
            .show(&mut list, |ui| {
                ui.set_min_width((ui.available_width() - 8.).max(200.));
                self.agenda(ui, self.page);
            });
        let handle = Rect::from_min_max(
            egui::pos2(list_rect.right() + 2., rect.top()),
            egui::pos2(list_rect.right() + 14., rect.bottom()),
        );
        let drag = ui
            .interact(handle, ui.id().with("agenda_splitter"), Sense::drag())
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if drag.hovered() || drag.dragged() {
            ui.painter().line_segment(
                [handle.center_top(), handle.center_bottom()],
                Stroke::new(2.0_f32, ACCENT.gamma_multiply(0.6)),
            );
        }
        if drag.dragged() {
            self.catalog.preview_width =
                (preview_width - drag.drag_delta().x).clamp(260., preview_max);
        }
        let mut preview = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("agenda_preview_pane")
                .max_rect(preview_rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        preview.set_clip_rect(ui.clip_rect().intersect(preview_rect));
        self.catalog_preview(&mut preview, height);
        ui.allocate_rect(rect, Sense::hover());
    }
    /// Memo rows of the agenda (each day of a multi-day memo is a row),
    /// filtered and sorted.
    fn agenda_rows(&self, mode: usize, today: NaiveDate) -> Vec<(usize, Option<NaiveDate>)> {
        let mut rows = vec![];
        for (i, n) in self.data.notes.iter().enumerate() {
            if !self.matches_search(i)
                || self.is_guide(i)
                || (self.desktop.hide_locked && n.locked)
                || (self.desktop.hide_completed
                    && n.done
                    && !self.extra.search_mode
                    && !self.extra.show_done)
                || (!self.recurring && n.recurring)
                || !crate::refinements::project_match(self.project_filter, n.project)
            {
                continue;
            }
            let days = if n.date.is_some() {
                n.days().into_iter().map(Some).collect()
            } else {
                vec![None]
            };
            for d in days {
                if match mode {
                    2 => d == Some(today),
                    3 => d.is_some_and(|d| d > today),
                    _ => true,
                } {
                    rows.push((i, d));
                }
            }
        }
        rows.sort_by_key(|&(i, d)| {
            let n = &self.data.notes[i];
            if self.extra.created_basis {
                (n.created.date(), n.created.time(), i, d)
            } else {
                (d.unwrap_or(NaiveDate::MAX), n.time, i, d)
            }
        });
        rows
    }
    pub fn agenda(&mut self, ui: &mut egui::Ui, mode: usize) {
        if !self.split && mode == 0 {
            self.detail_workspace(ui, mode);
            return;
        }
        let today = Local::now().date_naive();
        self.filters(ui);
        ui.add_space(14.);
        let mut cache = std::mem::take(&mut self.lists);
        let key = crate::lists::key((mode, today, self.data.notes.len()));
        let rows = cache.get("agenda", key, || self.agenda_rows(mode, today));
        self.lists = cache;
        let rows = (*rows).clone();
        if mode == 2 || mode == 3 {
            let mut seen = std::collections::HashSet::new();
            let important: Vec<_> = rows
                .iter()
                .copied()
                .filter(|&(i, _)| self.data.notes[i].starred && seen.insert(i))
                .collect();
            if !important.is_empty() {
                group_header(ui, "중요", important.len());
                ui.push_id("important", |ui| {
                    for (i, d) in important {
                        crate::lists::lazy_row(ui, (i, d), 48., false, |ui| {
                            self.task_row_on(ui, i, d)
                        });
                    }
                });
                ui.add_space(18.);
            }
        }
        if rows.is_empty() {
            ui.add_space(40.);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("표시할 메모가 없습니다")
                        .size(15.)
                        .color(ui::t(ui).muted),
                );
                ui::hint(ui, "필터를 바꾸거나 새 메모를 추가해 보세요");
            });
        }
        type Row = (usize, Option<NaiveDate>);
        let mut groups: Vec<(Option<NaiveDate>, Vec<Row>)> = vec![];
        for (i, occurrence) in rows {
            let d = if self.extra.created_basis {
                Some(self.data.notes[i].created.date())
            } else {
                occurrence
            };
            match groups.last_mut() {
                Some((g, items)) if *g == d => items.push((i, occurrence)),
                _ => groups.push((d, vec![(i, occurrence)])),
            }
        }
        for (d, items) in groups {
            let label = d
                .map(|d| {
                    let rel = match (d - today).num_days() {
                        0 => " · 오늘",
                        1 => " · 내일",
                        -1 => " · 어제",
                        _ => "",
                    };
                    format!("{} {}{rel}", d.format("%-m월 %-d일"), ui::weekday(d))
                })
                .unwrap_or("일정 없는 메모".into());
            group_header(ui, &label, items.len());
            for (i, occurrence) in items {
                crate::lists::lazy_row(ui, (i, occurrence), 48., false, |ui| {
                    self.task_row_on(ui, i, occurrence)
                });
            }
            ui.add_space(18.);
        }
    }
    pub fn calendar(&mut self, ui: &mut egui::Ui) {
        self.calendar_grid(ui);
        self.calendar_day_detail(ui);
    }
    /// The month/week grid with its navigation header and memo bands.
    pub fn calendar_grid(&mut self, ui: &mut egui::Ui) {
        let today = Local::now().date_naive();
        let (tone, sun, sat, today_mark) = self.calendar_tone(ui);
        ui.horizontal(|ui| {
            ui.set_min_height(32.);
            ui.label(
                RichText::new(self.month.format("%Y년 %-m월").to_string())
                    .size(18.)
                    .strong(),
            );
            ui.add_space(4.);
            let weeks = self.prefs.cal_weeks;
            let (back, forward) = if weeks == 0 {
                ("이전 달", "다음 달")
            } else {
                ("이전 주", "다음 주")
            };
            if icons::icon_button(ui, icons::Icon::ChevronLeft, back).clicked() {
                if weeks == 0 {
                    self.month = (self.month - Duration::days(1)).with_day(1).unwrap();
                } else {
                    self.day -= Duration::days(7 * weeks as i64);
                    self.month = self.day.with_day(1).unwrap();
                }
            }
            if icons::icon_button(ui, icons::Icon::ChevronRight, forward).clicked() {
                if weeks == 0 {
                    self.month = (self.month + Duration::days(32)).with_day(1).unwrap();
                } else {
                    self.day += Duration::days(7 * weeks as i64);
                    self.month = self.day.with_day(1).unwrap();
                }
            }
            if ui.button("오늘").clicked() {
                self.month = today.with_day(1).unwrap();
                self.day = today;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui::segmented(
                    ui,
                    &mut self.desktop.calendar_basis,
                    &["작성일", "일정", "알람"],
                );
                // Will open the linked Excel file of who is on duty; not
                // connected yet.
                let _ = ui
                    .button("근무자 보기")
                    .on_hover_text("연결된 엑셀 파일의 근무자 보기 · 준비 중");
            });
        });
        ui.add_space(10.);
        let width = ui.available_width().max(280.);
        let col = width / 7.;
        let (header, _) = ui.allocate_exact_size(Vec2::new(width, 26.), Sense::hover());
        for (i, label) in ["일", "월", "화", "수", "목", "금", "토"]
            .iter()
            .enumerate()
        {
            ui.painter().text(
                header.min + Vec2::new(i as f32 * col + 10., 13.),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(12.),
                match i {
                    0 => sun,
                    6 => sat,
                    _ => tone.faint,
                },
            );
        }
        let weeks = self.prefs.cal_weeks;
        // Month view starts on the Sunday before the 1st; week views on the
        // Sunday of the selected day's week.
        let (first, rows) = if weeks == 0 {
            let offset = self.month.weekday().num_days_from_sunday() as i64;
            let next = (self.month + Duration::days(32)).with_day(1).unwrap();
            let days = (next - self.month).num_days() + offset;
            (
                self.month - Duration::days(offset),
                (days as usize).div_ceil(7),
            )
        } else {
            let offset = self.day.weekday().num_days_from_sunday() as i64;
            (self.day - Duration::days(offset), weeks)
        };
        let height = match (weeks, width < 580.) {
            (1, _) => 300.,
            (2, _) => 220.,
            (_, true) => 104.,
            _ => 122.,
        };
        let (all, _) =
            ui.allocate_exact_size(Vec2::new(width, rows as f32 * height), Sense::hover());
        if let Some(theme) = self.cal.active() {
            ui.painter().rect_filled(all, 8., theme.card);
        }
        ui.painter().rect_stroke(
            all,
            8.,
            Stroke::new(1.0_f32, tone.line),
            egui::StrokeKind::Inside,
        );
        for r in 1..rows {
            let y = all.top() + r as f32 * height;
            ui.painter().line_segment(
                [egui::pos2(all.left(), y), egui::pos2(all.right(), y)],
                Stroke::new(1.0_f32, tone.line),
            );
        }
        for c in 1..7 {
            let x = all.left() + c as f32 * col;
            ui.painter().line_segment(
                [egui::pos2(x, all.top()), egui::pos2(x, all.bottom())],
                Stroke::new(1.0_f32, tone.line),
            );
        }
        for slot in 0..rows * 7 {
            let rect = Rect::from_min_size(
                all.min + Vec2::new((slot % 7) as f32 * col, (slot / 7) as f32 * height),
                Vec2::new(col, height),
            );
            let day = first + Duration::days(slot as i64);
            let weekday = day.weekday().num_days_from_sunday();
            if weeks == 0 && day.month() != self.month.month() {
                ui.painter().text(
                    rect.min + Vec2::new(10., 10.),
                    egui::Align2::LEFT_TOP,
                    day.day().to_string(),
                    egui::FontId::proportional(12.),
                    tone.faint.gamma_multiply(0.6),
                );
                continue;
            }
            if day == self.day {
                ui.painter().rect_filled(
                    rect.shrink(1.),
                    if slot == 0 { 7. } else { 0. },
                    tone.accent_soft.gamma_multiply(0.55),
                );
            }
            self.targets.push(dragging::Target {
                viewport: ui.ctx().viewport_id(),
                layer: ui.layer_id(),
                rect: rect.intersect(ui.clip_rect()),
                kind: dragging::Destination::Date(day),
            });
            let r = ui.interact(rect, ui.id().with(("day", day)), Sense::click());
            if r.clicked() {
                self.day = day;
            }
            if r.dnd_hover_payload::<NoteDrag>().is_some() {
                ui.painter().rect_stroke(
                    rect.shrink(2.),
                    4.,
                    Stroke::new(2.0_f32, ACCENT),
                    egui::StrokeKind::Inside,
                );
            }
            let holiday = matches!(
                (day.month(), day.day()),
                (1, 1) | (3, 1) | (5, 5) | (6, 6) | (8, 15) | (10, 3) | (10, 9) | (12, 25)
            );
            let datecolor = if holiday || weekday == 0 {
                sun
            } else if weekday == 6 {
                sat
            } else {
                tone.ink
            };
            let number = rect.min + Vec2::new(17., 18.);
            if day == today {
                ui.painter().circle_filled(number, 11., today_mark);
            }
            ui.painter().text(
                number,
                egui::Align2::CENTER_CENTER,
                day.day().to_string(),
                egui::FontId::proportional(12.5),
                if day == today {
                    Color32::WHITE
                } else {
                    datecolor
                },
            );
            if ui.rect_contains_pointer(rect) {
                let plus =
                    Rect::from_min_size(rect.right_top() + Vec2::new(-26., 5.), Vec2::splat(22.));
                overlay(ui, plus, slot + 25000, |ui| {
                    if icons::small_icon(ui, icons::Icon::Plus, "이 날짜에 새 스티커").clicked()
                    {
                        let i = self.new_sticker();
                        self.data.notes[i].date = Some(day);
                        self.day = day;
                    }
                });
            }
        }
        self.calendar_bands(ui, all, first, rows, col, height);
    }
    /// The selected day's title bar and its list of memos.
    pub fn calendar_day_detail(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} {}",
                    self.day.format("%-m월 %-d일"),
                    ui::weekday(self.day)
                ))
                .strong()
                .size(14.),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ 이 날짜에 스티커").clicked() {
                    let i = self.new_sticker();
                    self.data.notes[i].date = Some(self.day);
                }
            });
        });
        ui.add_space(4.);
        ui::divider(ui);
        let mut cache = std::mem::take(&mut self.lists);
        let key = crate::lists::key((self.day, self.data.notes.len()));
        let ids = cache.get("calendar_day", key, || {
            self.data
                .notes
                .iter()
                .enumerate()
                .filter(|(i, n)| {
                    self.visible(*i) && !self.is_guide(*i) && self.calendar_matches(n, self.day)
                })
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        });
        self.lists = cache;
        let ids = (*ids).clone();
        if ids.is_empty() {
            ui.add_space(10.);
            ui::hint(ui, "이 날짜에 표시할 메모가 없습니다");
        }
        for i in ids {
            self.task_row_on(ui, i, Some(self.day));
        }
    }
    /// Memo bands of the month grid. A memo shown on several days in a row
    /// is one band across those days (split only at week rows), placed in
    /// lanes so bands never overlap.
    fn calendar_bands(
        &mut self,
        ui: &mut egui::Ui,
        all: Rect,
        first: NaiveDate,
        rows: usize,
        col: f32,
        height: f32,
    ) {
        let (tone, ..) = self.calendar_tone(ui);
        let month = self.month.month();
        let whole_weeks = self.prefs.cal_weeks > 0;
        let hide_done = self.prefs.cal_hide_done;
        let band = (self.prefs.cal_font + 8.5).round();
        let step = band + 2.;
        let capacity = (((height - 36.) / step).floor() as usize).max(1);
        // Bands and lanes of every week row, reused until something changes.
        let mut cache = std::mem::take(&mut self.lists);
        let key = crate::lists::key((
            first,
            rows,
            month,
            whole_weeks,
            hide_done,
            self.data.notes.len(),
        ));
        let weeks = cache.get("calendar_bands", key, || {
            (0..rows)
                .map(|row| {
                    let start = first + Duration::days((row * 7) as i64);
                    let days: Vec<NaiveDate> = (0..7).map(|c| start + Duration::days(c)).collect();
                    let shown_day = |c: usize| whole_weeks || days[c].month() == month;
                    let mut segments = band_segments(
                        &self.data.notes,
                        |i| self.visible(i) && !self.is_guide(i),
                        |n, d| self.calendar_matches(n, d) && !(hide_done && n.done_on(Some(d))),
                        &days,
                        shown_day,
                    );
                    segments.sort_by_key(|s| {
                        (
                            s.first,
                            std::cmp::Reverse(s.last - s.first),
                            self.data.notes[s.note].time,
                        )
                    });
                    let lanes = assign_lanes(&segments);
                    (segments, lanes)
                })
                .collect::<Vec<_>>()
        });
        self.lists = cache;
        for (row, (segments, lanes)) in weeks.iter().enumerate() {
            let start = first + Duration::days((row * 7) as i64);
            let days: Vec<NaiveDate> = (0..7).map(|c| start + Duration::days(c)).collect();
            let shown_day = |c: usize| whole_weeks || days[c].month() == month;
            let mut covering = [0usize; 7];
            for s in segments {
                for c in &mut covering[s.first..=s.last] {
                    *c += 1;
                }
            }
            let visible_lanes: Vec<usize> = covering
                .iter()
                .map(|&n| if n > capacity { capacity - 1 } else { capacity })
                .collect();
            let row_top = all.top() + row as f32 * height;
            for (s, &lane) in segments.iter().zip(lanes) {
                // Draw the parts of the band whose days have room for its lane.
                let mut c = s.first;
                while c <= s.last {
                    if lane >= visible_lanes[c] {
                        c += 1;
                        continue;
                    }
                    let from = c;
                    while c < s.last && lane < visible_lanes[c + 1] {
                        c += 1;
                    }
                    let open_left = from == s.first && s.starts_here;
                    let open_right = c == s.last && s.ends_here;
                    let rect = Rect::from_min_max(
                        egui::pos2(
                            all.left() + from as f32 * col + if open_left { 5. } else { 0. },
                            row_top + 33. + lane as f32 * step,
                        ),
                        egui::pos2(
                            all.left() + (c + 1) as f32 * col - if open_right { 5. } else { 0. },
                            row_top + 33. + lane as f32 * step + band,
                        ),
                    );
                    self.calendar_band(
                        ui,
                        &tone,
                        s.note,
                        rect,
                        &days[from..=c],
                        open_left,
                        open_right,
                    );
                    c += 1;
                }
            }
            for (c, &n) in covering.iter().enumerate() {
                if n > capacity && shown_day(c) {
                    let shown = capacity - 1;
                    ui.painter().text(
                        egui::pos2(
                            all.left() + c as f32 * col + 9.,
                            row_top + 33. + shown as f32 * step + band / 2.,
                        ),
                        egui::Align2::LEFT_CENTER,
                        format!("+{}개 더보기", n - shown),
                        egui::FontId::proportional(11.),
                        tone.muted,
                    );
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn calendar_band(
        &mut self,
        ui: &mut egui::Ui,
        tone: &ui::Tone,
        i: usize,
        rect: Rect,
        days: &[NaiveDate],
        open_left: bool,
        open_right: bool,
    ) {
        let n = &self.data.notes[i];
        let done = days.iter().all(|&d| n.done_on(Some(d)));
        let fill = if self.dark {
            Color32::from_rgb(52, 50, 46)
        } else {
            n.color
        };
        let response = ui
            .interact(
                rect,
                ui.id().with(("band", i, days[0])),
                Sense::click_and_drag(),
            )
            .on_hover_text(&n.title)
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let r = |open: bool| if open { 4 } else { 0 };
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius {
                nw: r(open_left),
                sw: r(open_left),
                ne: r(open_right),
                se: r(open_right),
            },
            if response.hovered() {
                ui::lerp_color(fill, Color32::BLACK, 0.06)
            } else {
                fill
            },
        );
        let label = if n.title.trim().is_empty() {
            "제목 없음".to_owned()
        } else {
            n.title.clone()
        };
        let mut job = egui::text::LayoutJob::single_section(
            label,
            egui::TextFormat {
                font_id: egui::FontId::proportional(self.prefs.cal_font),
                color: if done {
                    tone.faint
                } else if self.dark {
                    tone.ink
                } else {
                    INK
                },
                strikethrough: if done && self.prefs.cal_strike {
                    Stroke::new(1.0_f32, tone.faint)
                } else {
                    Stroke::NONE
                },
                ..Default::default()
            },
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(rect.width() - 12.);
        let galley = ui.fonts(|f| f.layout_job(job));
        ui.painter().galley(
            egui::pos2(rect.left() + 6., rect.center().y - galley.size().y / 2.),
            galley,
            INK,
        );
        // The day under the pointer is the occurrence a click or drag uses.
        let day = response
            .interact_pointer_pos()
            .or(response.hover_pos())
            .map(|p| {
                let k = ((p.x - rect.left()) / (rect.width() / days.len() as f32)).floor();
                days[(k.max(0.) as usize).min(days.len() - 1)]
            })
            .unwrap_or(days[0]);
        response.dnd_set_drag_payload(NoteDrag);
        let occurrence = if self.desktop.calendar_basis == 1 {
            Some(day)
        } else {
            None
        };
        self.drag_source_on(ui, &response, i, occurrence);
        if response.clicked() {
            self.open_sticker(i, occurrence);
        }
    }
    /// Colors of the month grid: the app's own, or the calendar window's
    /// theme. Returns the tone and the Sunday, Saturday and today marks.
    fn calendar_tone(&self, ui: &egui::Ui) -> (ui::Tone, Color32, Color32, Color32) {
        let mut tone = ui::t(ui);
        match self.cal.active() {
            Some(t) => {
                tone.ink = t.ink;
                tone.muted = t.muted;
                tone.faint = t.faint;
                tone.line = t.line;
                tone.accent_soft = t.select;
                (tone, t.sun, t.sat, t.today)
            }
            None => (tone, ACCENT, ui::BLUE, ACCENT),
        }
    }
    fn calendar_matches(&self, n: &Note, day: NaiveDate) -> bool {
        match self.desktop.calendar_basis {
            0 => n.created.date() == day,
            2 => n.reminder.is_some_and(|t| t.date() == day),
            _ => n.occurs(day),
        }
    }
    pub fn note_editor(&mut self, ui: &mut egui::Ui) {
        let i = self.active;
        ui.horizontal(|ui| {
            self.note_options(ui, i);
            let n = &self.data.notes[i];
            let today = Local::now().date_naive();
            let occurrence = self.extra.edit_occurrence.or_else(|| {
                if n.occurs(today) {
                    Some(today)
                } else {
                    n.date
                }
            });
            let mut done = n.done_on(occurrence);
            if ui
                .add_sized([24., 32.], |ui: &mut egui::Ui| {
                    ui::round_check(ui, &mut done, None)
                })
                .on_hover_text("완료")
                .changed()
            {
                self.done_at(i, occurrence, done);
                self.extra.scope_owner = ui.ctx().viewport_id();
            }
            let width = (ui.available_width() - 34.).max(60.);
            ui.add_sized(
                [width, 34.],
                egui::TextEdit::singleline(&mut self.data.notes[i].title)
                    .font(egui::FontId::proportional(20.))
                    .frame(false)
                    .hint_text("제목"),
            );
            let starred = self.data.notes[i].starred;
            if icons::toggle_icon(
                ui,
                icons::Icon::Star,
                if starred {
                    "중요 표시 해제"
                } else {
                    "중요 표시"
                },
                starred,
            )
            .clicked()
            {
                self.data.notes[i].starred = !starred;
            }
        });
        ui.add_space(6.);
        self.date_controls(ui, i);
    }
    pub fn journal(&mut self, ui: &mut egui::Ui) {
        self.journal_v2(ui);
    }
    pub fn collections(&mut self, ui: &mut egui::Ui) {
        match self.page {
            7 => {
                ui.heading("표 모아보기");
                egui::Grid::new("table").striped(true).show(ui, |ui| {
                    for row in [
                        ["작업", "상태", "일정"],
                        ["화면 설계", "진행 중", "오늘"],
                        ["프로토타입 검토", "할 일", "내일"],
                    ] {
                        for c in row {
                            ui.label(c);
                        }
                        ui.end_row();
                    }
                });
            }
            8 => {
                ui.heading("이미지 모아보기");
                ui.horizontal_wrapped(|ui| {
                    for (c, t) in [PALETTE[0], PALETTE[1], PALETTE[2]]
                        .iter()
                        .zip(["영감", "기록", "생각"])
                    {
                        egui::Frame::NONE.fill(*c).inner_margin(35.).show(ui, |ui| {
                            ui.set_min_size(Vec2::new(150., 100.));
                            ui.label(RichText::new(t).color(Color32::WHITE));
                        });
                    }
                });
            }
            _ => {
                ui.heading("링크 · 즐겨찾기");
                for t in ["프로젝트 참고 자료", "사내 업무 포털", "디자인 아이디어"]
                {
                    if ui.button(format!("↗ {t}")).clicked() {
                        self.toast = "링크 실행은 후속 기능입니다".into();
                    }
                }
            }
        }
    }
}
impl App {
    pub fn board(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui::segmented(ui, &mut self.extra.board_tab, &["관계", "트리"]);
            if self.extra.board_tab == 0 {
                ui.add_space(6.);
                ui::segmented(ui, &mut self.board_filter, &["모든 메모", "선택한 메모"]);
                if ui::pill(ui, self.extra.created_basis, "생성시간 표시").clicked() {
                    self.extra.created_basis = !self.extra.created_basis;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.extra.board_tab == 0 {
                    if ui.button("+ 메모").clicked() {
                        self.add_note(None);
                    }
                    if ui.button("자동 정렬").clicked() {
                        self.auto_layout();
                    }
                    if ui
                        .add(
                            egui::Slider::new(&mut self.extra.zoom, 0.4..=1.8)
                                .show_value(false)
                                .text("배율"),
                        )
                        .on_hover_text("Ctrl + 휠로도 확대/축소")
                        .changed()
                    {
                        let center = self.extra.scene.center();
                        self.extra.scene =
                            Rect::from_center_size(center, self.extra.scene_size / self.extra.zoom);
                    }
                }
            });
        });
        if self.extra.board_tab == 1 {
            ui.add_space(10.);
            self.relation_tree(ui);
            return;
        }
        ui.add_space(6.);
        ui.horizontal_wrapped(|ui| {
            let tone = ui::t(ui);
            ui.label(
                RichText::new(format!("{}개 선택", self.picked.len())).color(
                    if self.picked.is_empty() {
                        tone.faint
                    } else {
                        ACCENT
                    },
                ),
            );
            let enabled = !self.picked.is_empty();
            ui.add_enabled_ui(enabled, |ui| {
                if ui
                    .button("연결")
                    .on_hover_text("선택한 두 메모를 연결")
                    .clicked()
                {
                    if self.picked.len() == 2 {
                        self.data.connect(self.picked[0], self.picked[1]);
                    } else {
                        self.toast = "연결할 메모 2개를 선택하세요".into();
                    }
                }
                if ui.button("연결 해제").clicked() && self.picked.len() == 2 {
                    let (a, b) = (self.picked[0], self.picked[1]);
                    self.data.links.retain(|e| *e != (a.min(b), a.max(b)));
                }
                if ui
                    .button("첫 메모 아래로 종속")
                    .on_hover_text("처음 선택한 메모 아래로 나머지를 묶습니다")
                    .clicked()
                    && self.picked.len() >= 2
                {
                    let p = self.picked[0];
                    for c in self.picked[1..].iter().copied() {
                        if !self.data.reparent(c, p) {
                            self.toast = "자기 자신 또는 순환 관계는 만들 수 없습니다".into();
                        }
                    }
                }
                if ui.button("종속 해제").clicked() {
                    for &i in &self.picked {
                        self.data.notes[i].parents.clear();
                    }
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.project_name)
                        .desired_width(120.)
                        .hint_text("프로젝트 이름"),
                );
                if ui.button("프로젝트로 묶기").clicked() {
                    if self.picked.is_empty() || self.project_name.trim().is_empty() {
                        self.toast = "메모와 프로젝트 이름을 선택하세요".into();
                    } else {
                        let p = self
                            .data
                            .projects
                            .iter()
                            .position(|s| s == self.project_name.trim())
                            .unwrap_or_else(|| {
                                self.data.projects.push(self.project_name.trim().into());
                                self.data.projects.len() - 1
                            });
                        for &i in &self.picked {
                            self.data.notes[i].project = Some(p);
                        }
                    }
                }
                if ui.button("선택 해제").clicked() {
                    self.picked.clear();
                    self.link_source = None;
                }
            });
        });
        ui::hint(
            ui,
            "카드 제목을 끌어 이동 · ≡ 를 다른 카드로 끌면 종속 · 카드의 연결 버튼으로 관계 연결",
        );
        ui.add_space(8.);
        self.extra.scene_size = ui.available_size();
        if !self.extra.scene_initialized {
            self.extra.scene = Rect::from_min_size(egui::Pos2::ZERO, self.extra.scene_size);
            self.extra.scene_initialized = true;
        }
        let mut scene = self.extra.scene;
        egui::Scene::new()
            .zoom_range(0.4..=1.8)
            .max_inner_size([5000., 5000.])
            .show(ui, &mut scene, |ui| {
                let (canvas, _) = ui.allocate_exact_size(Vec2::new(1450., 950.), Sense::hover());
                let ids: Vec<_> = (0..self.data.notes.len())
                    .filter(|&i| {
                        self.visible(i)
                            && match self.board_filter {
                                1 => self.picked.contains(&i),
                                _ => true,
                            }
                    })
                    .collect();
                let size = Vec2::new(254., if self.extra.created_basis { 170. } else { 152. });
                let rects: Vec<_> = self
                    .data
                    .notes
                    .iter()
                    .map(|n| Rect::from_min_size(canvas.min + n.pos, size))
                    .collect();
                let mut shown = vec![false; self.data.notes.len()];
                for &i in &ids {
                    shown[i] = true;
                }
                for &(a, b) in &self.data.links {
                    if shown[a] && shown[b] {
                        edge(
                            ui.painter(),
                            rects[a],
                            rects[b],
                            Color32::from_rgb(161, 167, 153),
                            true,
                        );
                    }
                }
                for &i in &ids {
                    for &p in &self.data.notes[i].parents {
                        if shown.get(p).copied().unwrap_or(false) {
                            let a = rects[p].center_bottom();
                            let b = rects[i].center_top();
                            ui.painter().arrow(
                                a,
                                b - a,
                                Stroke::new(2.0_f32, Color32::from_rgb(133, 121, 177)),
                            );
                        }
                    }
                }
                for i in ids {
                    let rect = rects[i];
                    // Cards outside the view are not drawn.
                    if !ui.is_rect_visible(rect) {
                        continue;
                    }
                    let selected = self.picked.contains(&i);
                    let tone = ui::t(ui);
                    let fill = if self.dark {
                        Color32::from_rgb(39, 40, 38)
                    } else {
                        Color32::WHITE
                    };
                    ui.painter().rect(
                        rect,
                        10.,
                        fill,
                        Stroke::new(
                            if selected { 2.0_f32 } else { 1.0_f32 },
                            if selected { ACCENT } else { tone.line_strong },
                        ),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().rect_filled(
                        Rect::from_min_size(
                            rect.min + Vec2::new(12., 0.),
                            Vec2::new(size.x - 24., 3.),
                        ),
                        2.,
                        self.data.notes[i].color,
                    );
                    overlay(ui, rect.shrink2(Vec2::new(12., 12.)), i + 2000, |ui| {
                        ui.set_clip_rect(ui.clip_rect().intersect(rect));
                        ui.spacing_mut().item_spacing.y = 5.;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.;
                            let mut s = self.picked.contains(&i);
                            if ui
                                .checkbox(&mut s, "")
                                .on_hover_text("여러 카드 선택")
                                .changed()
                            {
                                if s {
                                    self.picked.push(i)
                                } else {
                                    self.picked.retain(|&j| j != i)
                                }
                            }
                            let (handle, drag) =
                                ui.allocate_exact_size(Vec2::new(16., 22.), Sense::drag());
                            crate::icons::paint_icon(
                                ui.painter(),
                                handle.shrink2(Vec2::new(1., 4.)),
                                crate::icons::Icon::Menu,
                                tone.faint,
                            );
                            let drag = drag
                                .on_hover_text("다른 카드로 끌면 자식 메모가 됩니다")
                                .on_hover_cursor(egui::CursorIcon::Grab);
                            drag.dnd_set_drag_payload(NoteDrag);
                            self.drag_source(ui, &drag, i, false);
                            let title = shorten(&self.data.notes[i].title, 14);
                            let mut text = RichText::new(title).strong().size(13.5);
                            if self.data.notes[i].done {
                                text = text.strikethrough().color(tone.muted);
                            }
                            let grip = ui
                                .allocate_ui_with_layout(
                                    Vec2::new((ui.available_width() - 26.).max(40.), 22.),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.set_min_width(ui.available_width());
                                        ui.add(
                                            egui::Label::new(text)
                                                .truncate()
                                                .sense(Sense::click_and_drag()),
                                        )
                                    },
                                )
                                .inner;
                            self.drag_source(ui, &grip, i, true);
                            if grip.double_clicked() {
                                self.open_note(i);
                            }
                            let n = &mut self.data.notes[i];
                            let star = crate::icons::small_icon(
                                ui,
                                crate::icons::Icon::Star,
                                if n.starred {
                                    "중요 해제"
                                } else {
                                    "중요 표시"
                                },
                            );
                            if n.starred {
                                crate::icons::paint_icon(
                                    ui.painter(),
                                    star.rect.shrink(5.),
                                    crate::icons::Icon::Star,
                                    ACCENT,
                                );
                            }
                            if star.clicked() {
                                n.starred = !n.starred;
                            }
                        });
                        ui.add(
                            egui::Label::new(
                                RichText::new(if self.data.notes[i].locked {
                                    "잠긴 메모".into()
                                } else {
                                    shorten(&self.data.notes[i].body, 40)
                                })
                                .size(12.)
                                .color(tone.muted),
                            )
                            .wrap(),
                        );
                        let n = &self.data.notes[i];
                        let mut meta = vec![n
                            .project
                            .map(|p| format!("# {}", self.data.projects[p]))
                            .unwrap_or("프로젝트 없음".into())];
                        meta.push(match n.parents.first() {
                            Some(&p) => {
                                format!("상위 · {}", shorten(&self.data.notes[p].title, 10))
                            }
                            None => "독립 메모".into(),
                        });
                        if self.extra.created_basis {
                            meta.push(format!("생성 {}", n.created.format("%m-%d %H:%M")));
                        }
                        ui.label(
                            RichText::new(meta.join("  ·  "))
                                .size(11.)
                                .color(tone.faint),
                        );
                        ui.add_space(2.);
                        ui.horizontal(|ui| {
                            let mut d = self.data.notes[i].done;
                            if ui::round_check(ui, &mut d, None).changed() {
                                self.done(i, d);
                                self.extra.scope_owner = ui.ctx().viewport_id();
                            }
                            ui.label(RichText::new("완료").size(12.).color(tone.muted));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let linking = self.link_source == Some(i);
                                    if ui::pill(
                                        ui,
                                        linking,
                                        if linking {
                                            "대상 선택 중"
                                        } else {
                                            "연결"
                                        },
                                    )
                                    .on_hover_text("두 카드를 차례로 눌러 연결합니다")
                                    .clicked()
                                    {
                                        if let Some(s) = self.link_source.take() {
                                            self.data.connect(s, i);
                                        } else {
                                            self.link_source = Some(i);
                                            self.toast = "다른 카드의 연결 버튼을 누르세요".into();
                                        }
                                    }
                                    if ui::pill(ui, false, "열기").clicked() {
                                        self.open_note(i);
                                    }
                                },
                            );
                        });
                    });
                    self.targets.push(dragging::Target {
                        viewport: ui.ctx().viewport_id(),
                        layer: ui.layer_id(),
                        rect: rect.intersect(ui.clip_rect()),
                        kind: dragging::Destination::Parent(i),
                    });
                    let _drop = ui.interact(rect, ui.id().with(("board_drop", i)), Sense::hover());
                }
            });
        self.extra.scene = scene;
        if let Some(t) = ui.ctx().layer_transform_to_global(egui::LayerId::new(
            ui.layer_id().order,
            ui.id().with("scene_area"),
        )) {
            self.extra.zoom = t.scaling;
        }
    }
}

fn group_header(ui: &mut egui::Ui, label: &str, count: usize) {
    let tone = ui::t(ui);
    ui.horizontal(|ui| {
        ui.add_space(4.);
        ui.label(RichText::new(label).strong().size(13.5));
        ui.label(RichText::new(count.to_string()).size(12.).color(tone.faint));
    });
    ui.add_space(2.);
    ui::divider(ui);
}
fn overlay(ui: &mut egui::Ui, rect: Rect, id: usize, f: impl FnOnce(&mut egui::Ui)) {
    let mut child = ui.new_child(egui::UiBuilder::new().id_salt(id).max_rect(rect));
    f(&mut child);
}
fn edge(p: &egui::Painter, a: Rect, b: Rect, color: Color32, both: bool) {
    let points = |r: Rect| {
        [
            r.center_top(),
            r.center_bottom(),
            r.left_center(),
            r.right_center(),
        ]
    };
    let mut best = (a.center(), b.center());
    let mut dist = f32::INFINITY;
    for x in points(a) {
        for y in points(b) {
            let d = x.distance_sq(y);
            if d < dist {
                best = (x, y);
                dist = d;
            }
        }
    }
    let (x, y) = best;
    if x.distance(y) < 1. {
        return;
    }
    p.line_segment([x, y], Stroke::new(1.5_f32, color));
    let head = |tip: egui::Pos2, other: egui::Pos2| {
        let u = (tip - other).normalized();
        let back = tip - u * 9.;
        let n = Vec2::new(-u.y, u.x) * 4.;
        p.line_segment([tip, back + n], Stroke::new(1.5_f32, color));
        p.line_segment([tip, back - n], Stroke::new(1.5_f32, color));
    };
    head(y, x);
    if both {
        head(x, y);
    }
}
/// A run of consecutive days of one week row on which memo `note` shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandSegment {
    pub note: usize,
    pub first: usize,
    pub last: usize,
    /// The memo does not show the day before `first` (round the left end).
    pub starts_here: bool,
    /// The memo does not show the day after `last` (round the right end).
    pub ends_here: bool,
}
/// Runs of consecutive `days` (one week row) on which each memo shows.
pub fn band_segments(
    notes: &[Note],
    visible: impl Fn(usize) -> bool,
    matches: impl Fn(&Note, NaiveDate) -> bool,
    days: &[NaiveDate],
    shown_day: impl Fn(usize) -> bool,
) -> Vec<BandSegment> {
    let mut out = vec![];
    for (i, n) in notes.iter().enumerate() {
        if !visible(i) {
            continue;
        }
        let on: Vec<bool> = days
            .iter()
            .enumerate()
            .map(|(c, &d)| shown_day(c) && matches(n, d))
            .collect();
        let mut c = 0;
        while c < on.len() {
            if !on[c] {
                c += 1;
                continue;
            }
            let first = c;
            while c + 1 < on.len() && on[c + 1] {
                c += 1;
            }
            out.push(BandSegment {
                note: i,
                first,
                last: c,
                starts_here: !(first == 0 && matches(n, days[0] - Duration::days(1))),
                ends_here: !(c == on.len() - 1 && matches(n, days[c] + Duration::days(1))),
            });
            c += 1;
        }
    }
    out
}
/// The lowest free lane for each segment, in order.
pub fn assign_lanes(segments: &[BandSegment]) -> Vec<usize> {
    let mut used: Vec<[bool; 7]> = vec![];
    segments
        .iter()
        .map(|s| {
            let lane = (0..)
                .find(|&l| {
                    used.get(l)
                        .is_none_or(|u| u[s.first..=s.last].iter().all(|x| !x))
                })
                .unwrap();
            if used.len() <= lane {
                used.resize(lane + 1, [false; 7]);
            }
            for x in &mut used[lane][s.first..=s.last] {
                *x = true;
            }
            lane
        })
        .collect()
}
#[cfg(test)]
mod band_tests {
    use super::*;
    #[test]
    fn a_multi_day_memo_is_one_band_split_only_by_week_rows() {
        let d = |day: u32| NaiveDate::from_ymd_opt(2026, 11, day).unwrap();
        let mut long = Note::new("화면 흐름 검토", Some(d(17)), 0);
        long.duration = 4;
        let mut wrap = Note::new("주말 걸침", Some(d(20)), 1);
        wrap.duration = 3;
        let one = Note::new("하루", Some(d(18)), 2);
        let notes = vec![long, wrap, one];
        // Week row Sun 15 .. Sat 21.
        let days: Vec<_> = (15..=21).map(d).collect();
        let segs = band_segments(&notes, |_| true, |n, day| n.occurs(day), &days, |_| true);
        assert_eq!(
            segs[0],
            BandSegment {
                note: 0,
                first: 2,
                last: 5,
                starts_here: true,
                ends_here: true
            }
        );
        // Runs on into the next week: the right end stays open.
        assert_eq!(
            segs[1],
            BandSegment {
                note: 1,
                first: 5,
                last: 6,
                starts_here: true,
                ends_here: false
            }
        );
        assert_eq!(assign_lanes(&segs), vec![0, 1, 1]);
    }
}
#[cfg(test)]
mod schedule_button_tests {
    use super::*;
    #[test]
    fn new_schedule_button_creates_a_dated_note_and_opens_its_editor() {
        let today = Local::now().date_naive();
        let mut a = App::sample(today);
        let before = a.data.notes.len();
        a.add_note(Some(today));
        assert_eq!(a.data.notes.len(), before + 1);
        let i = before;
        assert_eq!(a.data.notes[i].date, Some(today));
        // "+ 새 일정" opens the memo editor window, not just a bare sticker.
        assert!(a.editor);
        assert_eq!(a.active, i);
        assert!(a.studio.windows.contains_key(&i));
    }
}
