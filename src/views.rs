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
                if ui::primary(ui, "+ 새 메모").clicked() {
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
    pub fn agenda(&mut self, ui: &mut egui::Ui, mode: usize) {
        if !self.split && mode == 0 {
            self.detail_workspace(ui, mode);
            return;
        }
        let today = Local::now().date_naive();
        self.filters(ui);
        ui.add_space(14.);
        let mut rows = vec![];
        for (i, n) in self.data.notes.iter().enumerate() {
            if !self.matches_search(i)
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
                        self.task_row_on(ui, i, d);
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
                self.task_row_on(ui, i, occurrence);
            }
            ui.add_space(18.);
        }
    }
    pub fn calendar(&mut self, ui: &mut egui::Ui) {
        let today = Local::now().date_naive();
        let tone = ui::t(ui);
        ui.horizontal(|ui| {
            ui.set_min_height(32.);
            ui.label(
                RichText::new(self.month.format("%Y년 %-m월").to_string())
                    .size(18.)
                    .strong(),
            );
            ui.add_space(4.);
            if icons::icon_button(ui, icons::Icon::ChevronLeft, "이전 달").clicked() {
                self.month = (self.month - Duration::days(1)).with_day(1).unwrap();
            }
            if icons::icon_button(ui, icons::Icon::ChevronRight, "다음 달").clicked() {
                self.month = (self.month + Duration::days(32)).with_day(1).unwrap();
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
                    0 => ACCENT,
                    6 => ui::BLUE,
                    _ => tone.faint,
                },
            );
        }
        let offset = self.month.weekday().num_days_from_sunday() as usize;
        let next = (self.month + Duration::days(32)).with_day(1).unwrap();
        let rows = ((next - self.month).num_days() as usize + offset).div_ceil(7);
        let height = if width < 580. { 104. } else { 122. };
        let (all, _) =
            ui.allocate_exact_size(Vec2::new(width, rows as f32 * height), Sense::hover());
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
            let day = self.month + Duration::days(slot as i64 - offset as i64);
            let weekday = day.weekday().num_days_from_sunday();
            if day.month() != self.month.month() {
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
                ACCENT
            } else if weekday == 6 {
                ui::BLUE
            } else {
                tone.ink
            };
            let number = rect.min + Vec2::new(17., 18.);
            if day == today {
                ui.painter().circle_filled(number, 11., ACCENT);
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
            let mut ids: Vec<_> = self
                .data
                .notes
                .iter()
                .enumerate()
                .filter(|(i, n)| self.visible(*i) && self.calendar_matches(n, day))
                .map(|(i, _)| i)
                .collect();
            ids.sort_by_key(|&i| self.data.notes[i].time);
            let count = ids.len();
            let capacity = (((height - 36.) / 22.).floor() as usize).max(1);
            let shown = if count > capacity {
                capacity - 1
            } else {
                count
            };
            for (row, i) in ids.into_iter().take(shown).enumerate() {
                let pill = Rect::from_min_size(
                    rect.min + Vec2::new(5., 33. + row as f32 * 22.),
                    Vec2::new(col - 10., 20.),
                );
                let n = &self.data.notes[i];
                let done = n.done_on(Some(day));
                let base = n.color;
                let fill = if self.dark {
                    Color32::from_rgb(52, 50, 46)
                } else {
                    base
                };
                let response = ui
                    .interact(
                        pill,
                        ui.id().with(("pill", i, day)),
                        Sense::click_and_drag(),
                    )
                    .on_hover_text(&n.title)
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                ui.painter().rect_filled(
                    pill,
                    4.,
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
                        font_id: egui::FontId::proportional(11.5),
                        color: if done {
                            tone.faint
                        } else if self.dark {
                            tone.ink
                        } else {
                            INK
                        },
                        strikethrough: if done {
                            Stroke::new(1.0_f32, tone.faint)
                        } else {
                            Stroke::NONE
                        },
                        ..Default::default()
                    },
                );
                job.wrap = egui::text::TextWrapping::truncate_at_width(pill.width() - 12.);
                let galley = ui.fonts(|f| f.layout_job(job));
                ui.painter().galley(
                    egui::pos2(pill.left() + 6., pill.center().y - galley.size().y / 2.),
                    galley,
                    INK,
                );
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
            if count > shown {
                ui.painter().text(
                    rect.min + Vec2::new(9., 33. + shown as f32 * 22. + 10.),
                    egui::Align2::LEFT_CENTER,
                    format!("+{}개 더보기", count - shown),
                    egui::FontId::proportional(11.),
                    tone.muted,
                );
            }
        }
        ui.add_space(18.);
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
        let ids: Vec<_> = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(i, n)| self.visible(*i) && self.calendar_matches(n, self.day))
            .map(|(i, _)| i)
            .collect();
        if ids.is_empty() {
            ui.add_space(10.);
            ui::hint(ui, "이 날짜에 표시할 메모가 없습니다");
        }
        for i in ids {
            self.task_row_on(ui, i, Some(self.day));
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
                for &(a, b) in &self.data.links {
                    if ids.contains(&a) && ids.contains(&b) {
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
                        if ids.contains(&p) {
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
