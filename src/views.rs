use crate::*;
use chrono::{Duration, NaiveDateTime, NaiveTime};
use egui::{Rect, Sense, Stroke};
impl App {
    pub fn filters(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.recurring, "정기 일정 보기");
            egui::ComboBox::from_id_salt(ui.id().with("projects"))
                .selected_text(crate::refinements::project_label(
                    self.project_filter,
                    &self.data.projects,
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
            if ui.button("+ 할 일 추가").clicked() {
                self.add_note(Some(match self.page {
                    2 => Local::now().date_naive(),
                    3 => Local::now().date_naive() + Duration::days(1),
                    _ => self.day,
                }));
            }
        });
    }
    pub fn task_row_on(&mut self, ui: &mut egui::Ui, i: usize, occurrence: Option<NaiveDate>) {
        ui.push_id((i, occurrence), |ui| {
            ui.horizontal(|ui| {
                let drag = ui
                    .add(egui::Label::new(RichText::new("≡").color(MUTED)).sense(Sense::drag()))
                    .on_hover_text("날짜 칸으로 드래그");
                drag.dnd_set_drag_payload(NoteDrag);
                self.drag_source_on(ui, &drag, i, occurrence);
                let mut d = self.data.notes[i].done_on(occurrence);
                if ui.checkbox(&mut d, "").changed() {
                    self.done_at(i, occurrence, d);
                    self.extra.scope_owner = ui.ctx().viewport_id();
                }
                ui.vertical(|ui| {
                    let n = &self.data.notes[i];
                    let t = if n.done_on(occurrence) {
                        RichText::new(&n.title).strikethrough().color(MUTED)
                    } else {
                        RichText::new(&n.title)
                    };
                    let r = ui.add(egui::Label::new(t).sense(Sense::click_and_drag()));
                    r.dnd_set_drag_payload(NoteDrag);
                    self.drag_source_on(ui, &r, i, occurrence);
                    if r.clicked() {
                        self.open_note_on(i, occurrence);
                    }
                    let n = &self.data.notes[i];
                    let date = occurrence
                        .map(|d| format!("{}  {}", d.format("%m월 %d일"), n.time.format("%H:%M")))
                        .unwrap_or("일정 없음".into());
                    let project = n
                        .project
                        .map(|p| format!(" · {}", self.data.projects[p]))
                        .unwrap_or_default();
                    ui.label(
                        RichText::new(format!(
                            "{date}{} · {}{project}",
                            if n.recurring { "  ↻" } else { "" },
                            LABELS[n.label]
                        ))
                        .small()
                        .color(COLORS[n.label]),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("편집").clicked() {
                        self.open_note_on(i, occurrence);
                    }
                    let n = &mut self.data.notes[i];
                    if ui.small_button(if n.starred { "★" } else { "☆" }).clicked() {
                        n.starred = !n.starred;
                    }
                });
            });
            if self.extra.created_basis {
                ui.label(
                    RichText::new(format!(
                        "생성 {}",
                        self.data.notes[i].created.format("%Y-%m-%d %H:%M")
                    ))
                    .small()
                    .color(MUTED),
                );
            }
            if self.extra.search_mode {
                let d = if self.extra.created_basis {
                    Some(self.data.notes[i].created.date())
                } else {
                    self.data.notes[i].date
                };
                if let Some(d) = d {
                    if ui
                        .link(format!("{} · 달력에서 주변 날짜 보기", d))
                        .clicked()
                    {
                        self.month = d.with_day(1).unwrap();
                        self.day = d;
                        self.split = true;
                    }
                }
            }
            ui.add_space(7.);
            ui.separator();
            ui.add_space(7.);
        });
    }
    pub fn agenda(&mut self, ui: &mut egui::Ui, mode: usize) {
        let today = Local::now().date_naive();
        let name = match mode {
            1 => "나만의 위키",
            2 => "오늘",
            3 => "다음",
            _ => "내 작업실",
        };
        ui.horizontal(|ui| {
            ui.heading(if self.extra.search_mode {
                "검색 결과"
            } else {
                name
            });
            if mode == 0
                && ui
                    .button(if self.split {
                        "캘린더 닫기"
                    } else {
                        "캘린더 나란히"
                    })
                    .clicked()
            {
                self.split = !self.split;
            }
        });
        ui.label(
            RichText::new("제목을 눌러 편집 · 손잡이나 제목을 날짜로 드래그")
                .small()
                .color(MUTED),
        );
        ui.add_space(15.);
        self.filters(ui);
        ui.checkbox(&mut self.extra.created_basis, "생성시간 기준");
        ui.add_space(20.);
        let mut rows = vec![];
        for (i, n) in self.data.notes.iter().enumerate() {
            if !self.matches_search(i)
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
                ui.strong("★ 중요");
                ui.push_id("important", |ui| {
                    for (i, d) in important {
                        self.task_row_on(ui, i, d);
                    }
                });
                ui.add_space(15.);
            }
        }
        let mut group = None;
        if rows.is_empty() {
            ui.label(
                "표시할 메모가 없습니다. 프로젝트 없음 필터로 빠른 메모를 확인할 수 있습니다.",
            );
        }
        for (i, occurrence) in rows {
            let d = if self.extra.created_basis {
                Some(self.data.notes[i].created.date())
            } else {
                occurrence
            };
            if group != Some(d) {
                ui.add_space(12.);
                ui.strong(
                    d.map(|d| {
                        format!(
                            "{}{}",
                            d.format("%m월 %d일"),
                            if d == today { " · 오늘" } else { "" }
                        )
                    })
                    .unwrap_or("일정 없는 메모".into()),
                );
                ui.separator();
                group = Some(d);
            }
            self.task_row_on(ui, i, occurrence);
        }
    }
    pub fn calendar(&mut self, ui: &mut egui::Ui) {
        let today = Local::now().date_naive();
        ui.horizontal(|ui| {
            ui.heading(self.month.format("%Y년 %m월").to_string());
            if ui.small_button("〈").clicked() {
                self.month = (self.month - Duration::days(1)).with_day(1).unwrap();
            }
            if ui.small_button("〉").clicked() {
                self.month = (self.month + Duration::days(32)).with_day(1).unwrap();
            }
            if ui.small_button("오늘").clicked() {
                self.month = today.with_day(1).unwrap();
                self.day = today;
            }
        });
        ui.label(
            RichText::new("날짜로 드래그 · 일정 제목을 눌러 편집")
                .small()
                .color(MUTED),
        );
        ui.add_space(10.);
        let width = ui.available_width().max(280.);
        let col = width / 7.;
        let (header, _) = ui.allocate_exact_size(Vec2::new(width, 24.), Sense::hover());
        for (i, n) in ["일", "월", "화", "수", "목", "금", "토"]
            .iter()
            .enumerate()
        {
            ui.painter().text(
                header.min + Vec2::new(i as f32 * col + 8., 8.),
                egui::Align2::LEFT_CENTER,
                n,
                egui::FontId::proportional(12.),
                MUTED,
            );
        }
        let offset = self.month.weekday().num_days_from_sunday() as usize;
        let next = (self.month + Duration::days(32)).with_day(1).unwrap();
        let days = (next - self.month).num_days() as usize;
        let rows = (days + offset).div_ceil(7);
        let height = if width < 580. { 104. } else { 118. };
        let (all, _) =
            ui.allocate_exact_size(Vec2::new(width, rows as f32 * height), Sense::hover());
        for slot in 0..rows * 7 {
            let rect = Rect::from_min_size(
                all.min + Vec2::new((slot % 7) as f32 * col, (slot / 7) as f32 * height),
                Vec2::new(col, height),
            );
            let day = self.month + Duration::days(slot as i64 - offset as i64);
            let fill = if day == self.day {
                if self.dark {
                    Color32::from_rgb(43, 39, 34)
                } else {
                    Color32::from_rgb(255, 249, 240)
                }
            } else {
                ui.visuals().panel_fill
            };
            ui.painter().rect_filled(rect, 0., fill);
            ui.painter().rect_stroke(
                rect,
                0.,
                Stroke::new(
                    0.6_f32,
                    if self.dark {
                        Color32::from_gray(65)
                    } else {
                        Color32::from_gray(227)
                    },
                ),
                egui::StrokeKind::Inside,
            );
            if day.month() != self.month.month() {
                continue;
            }
            self.targets.push(dragging::Target {
                viewport: ui.ctx().viewport_id(),
                layer: ui.layer_id(),
                rect: rect.intersect(ui.clip_rect()),
                kind: dragging::Destination::Date(day),
            });
            let drop = ui.interact(rect, ui.id().with(("day", day)), Sense::click());
            if drop.clicked() {
                self.day = day;
            }
            if drop.dnd_hover_payload::<NoteDrag>().is_some() {
                ui.painter().rect_stroke(
                    rect.shrink(2.),
                    3.,
                    Stroke::new(2.0_f32, ACCENT),
                    egui::StrokeKind::Inside,
                );
            }

            ui.painter().text(
                rect.min + Vec2::new(8., 10.),
                egui::Align2::LEFT_TOP,
                day.day().to_string(),
                egui::FontId::proportional(12.),
                if day == today { ACCENT } else { MUTED },
            );
            let mut ids: Vec<_> = self
                .data
                .notes
                .iter()
                .enumerate()
                .filter(|(i, n)| self.visible(*i) && n.occurs(day))
                .map(|(i, _)| i)
                .collect();
            ids.sort_by_key(|&i| self.data.notes[i].time);
            let count = ids.len();
            for (row, i) in ids.into_iter().take(3).enumerate() {
                let r = Rect::from_min_size(
                    rect.min + Vec2::new(4., 29. + row as f32 * 24.),
                    Vec2::new(col - 8., 22.),
                );
                overlay(ui, r, i + 1000, |ui| {
                    let n = &self.data.notes[i];
                    let max = ((col - 18.) / 12.) as usize;
                    let title = shorten(&n.title, max.max(2));
                    let t = RichText::new(title).size(12.).color(if self.dark {
                        Color32::from_gray(235)
                    } else {
                        INK
                    });
                    let t = if n.done_on(Some(day)) {
                        t.strikethrough()
                    } else {
                        t
                    };
                    let res = ui
                        .add_sized(
                            r.size(),
                            egui::Button::new(t)
                                .small()
                                .fill(if self.dark {
                                    Color32::from_rgb(48, 51, 46)
                                } else {
                                    Color32::from_rgb(235, 240, 230)
                                })
                                .sense(Sense::click_and_drag()),
                        )
                        .on_hover_text(&n.title);
                    res.dnd_set_drag_payload(NoteDrag);
                    self.drag_source_on(ui, &res, i, Some(day));
                    if res.clicked() {
                        self.open_note_on(i, Some(day));
                    }
                });
            }
            if count > 3 {
                ui.painter().text(
                    rect.left_bottom() + Vec2::new(7., -3.),
                    egui::Align2::LEFT_BOTTOM,
                    format!("+ {}개", count - 3),
                    egui::FontId::proportional(11.),
                    MUTED,
                );
            }
        }
        ui.add_space(10.);
        ui.horizontal(|ui| {
            ui.strong(self.day.format("%m월 %d일").to_string());
            if ui.button("+ 일정 추가").clicked() {
                self.add_note(Some(self.day));
            }
        });
        let ids: Vec<_> = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(i, n)| self.visible(*i) && n.occurs(self.day))
            .map(|(i, _)| i)
            .collect();
        for i in ids {
            self.task_row_on(ui, i, Some(self.day));
        }
    }
    pub fn note_editor(&mut self, ui: &mut egui::Ui) {
        let i = self.active;
        ui.horizontal(|ui| {
            let n = &self.data.notes[i];
            let today = Local::now().date_naive();
            let occurrence = self.extra.edit_occurrence.or_else(|| {
                if n.occurs(today) {
                    Some(today)
                } else {
                    n.date
                }
            });
            let mut d = n.done_on(occurrence);
            if ui.checkbox(&mut d, "완료").changed() {
                self.done_at(i, occurrence, d);
                self.extra.scope_owner = ui.ctx().viewport_id();
                self.completion_edit = self.data.notes[i]
                    .completed
                    .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default();
            }
        });
        if self.data.notes[i].duration > 1 {
            ui.label(format!(
                "기간 {}일 · {}일 완료",
                self.data.notes[i].duration,
                self.data.notes[i].occurrence_completed.len()
            ));
        }
        self.note_options(ui, i);
        self.date_controls(ui, i);
        let can_edit_completion = self.editor_completion().is_some();
        let mut correction = None;
        let n = &mut self.data.notes[i];
        ui.add(
            egui::TextEdit::multiline(&mut n.title)
                .font(egui::FontId::proportional(26.))
                .desired_rows(1)
                .desired_width(f32::INFINITY)
                .frame(false),
        );
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label("날짜");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.date_edit)
                        .hint_text("YYYY-MM-DD")
                        .desired_width(115.),
                )
                .clicked()
            {
                self.extra.picker = Some(crate::extras::Picker::new(
                    0,
                    n.date.unwrap_or(Local::now().date_naive()),
                    n.time,
                ));
            }
            ui.label("시간");
            if ui
                .add(egui::TextEdit::singleline(&mut self.time_edit).desired_width(65.))
                .clicked()
            {
                self.extra.picker = Some(crate::extras::Picker::new(
                    0,
                    n.date.unwrap_or(Local::now().date_naive()),
                    n.time,
                ));
            }
            if ui.button("일정 적용").clicked() {
                let date = if self.date_edit.trim().is_empty() {
                    Ok(None)
                } else {
                    NaiveDate::parse_from_str(&self.date_edit, "%Y-%m-%d").map(Some)
                };
                match (date, NaiveTime::parse_from_str(&self.time_edit, "%H:%M")) {
                    (Ok(d), Ok(t)) => {
                        if let Some(d) = d {
                            n.move_start(d);
                        } else {
                            n.date = None;
                        }
                        n.time = t;
                        self.edit_error.clear();
                    }
                    _ => self.edit_error = "날짜 YYYY-MM-DD, 시간 HH:MM을 확인하세요".into(),
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt(ui.id().with("label"))
                .selected_text(LABELS[n.label])
                .show_ui(ui, |ui| {
                    for (i, t) in LABELS.iter().enumerate() {
                        ui.selectable_value(&mut n.label, i, *t);
                    }
                });
            egui::ComboBox::from_id_salt(ui.id().with("project"))
                .selected_text(
                    n.project
                        .map(|p| self.data.projects[p].as_str())
                        .unwrap_or("프로젝트 없음"),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut n.project, None, "프로젝트 없음");
                    for (i, p) in self.data.projects.iter().enumerate() {
                        if self.extra.hidden_projects.contains(&i)
                            || self.extra.deleted_projects.contains(&i)
                        {
                            continue;
                        }
                        ui.selectable_value(&mut n.project, Some(i), p);
                    }
                });
            ui.checkbox(&mut n.recurring, "정기 일정 표시");
            let old = n.duration;
            egui::ComboBox::from_id_salt("duration_days")
                .selected_text(format!("{}일", n.duration))
                .show_ui(ui, |ui| {
                    for days in 1..=5 {
                        ui.selectable_value(&mut n.duration, days, format!("{days}일"));
                    }
                });
            if old != n.duration {
                n.refresh_done();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut n.starred, "중요");
            ui.checkbox(&mut n.wiki, "위키");
            if n.wiki {
                egui::ComboBox::from_id_salt("wiki_folder_assignment")
                    .selected_text(
                        n.folder
                            .map(|f| self.extra.folders[f].title.as_str())
                            .unwrap_or("미분류"),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut n.folder, None, "미분류");
                        for (f, folder) in self.extra.folders.iter().enumerate() {
                            ui.selectable_value(&mut n.folder, Some(f), &folder.title);
                        }
                    });
            }
        });
        ui.label(
            RichText::new("정기 일정 표시는 필터 시연용이며 반복 계산은 후속 단계입니다.")
                .small()
                .color(MUTED),
        );
        ui.add_space(12.);
        crate::refinements::body_input(ui, &mut n.body, 9, "메모 내용을 입력하세요");
        if can_edit_completion {
            ui.separator();
            ui.label("완료 시각 · 실제 완료한 시각으로 수정 가능");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.completion_edit).desired_width(180.));
                if ui.button("완료 시각 적용").clicked() {
                    match NaiveDateTime::parse_from_str(&self.completion_edit, "%Y-%m-%d %H:%M") {
                        Ok(t) => {
                            correction = Some(t);
                            self.edit_error.clear();
                        }
                        Err(_) => self.edit_error = "YYYY-MM-DD HH:MM 형식을 입력하세요".into(),
                    }
                }
            });
        }
        if !self.edit_error.is_empty() {
            ui.colored_label(ACCENT, &self.edit_error);
        }
        ui.separator();
        if let Some(t) = correction {
            self.set_completion_time(t);
        }
        self.consume_project_header(i);
        ui.strong("연결된 메모 / 하위 메모");
        let ids: Vec<_> = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(j, n)| {
                self.visible(*j)
                    && (n.parents.contains(&i) || self.data.links.contains(&(i.min(*j), i.max(*j))))
            })
            .map(|(j, _)| j)
            .collect();
        for j in ids {
            if ui.link(&self.data.notes[j].title).clicked() {
                self.open_note(j);
            }
        }
    }
    pub fn journal(&mut self, ui: &mut egui::Ui) {
        self.journal_v2(ui);
    }
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("화면과 단축키");
        ui.checkbox(&mut self.dark, "다크 모드");
        ui.separator();
        ui.strong("전역 단축키");
        ui.label("다른 Windows 프로그램 사용 중에도 동작합니다. 앱은 실행 중이어야 합니다.");
        for (i, name) in ["빠른 메모", "분리 캘린더"].iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(*name);
                ui.text_edit_singleline(&mut self.shortcuts[i]);
            });
        }
        if ui.button("단축키 적용").clicked() {
            self.apply_shortcuts();
        }
        ui.label(&self.hotkey_status);
        ui.label(
            RichText::new("예: Ctrl+Alt+N / Ctrl+Shift+F8 · 충돌하면 기존 설정 복구")
                .small()
                .color(MUTED),
        );
        ui.separator();
        ui.label("화면 안 단축키: Escape = 대화창 닫기");
        ui.label("모든 변경은 실행 중에만 유지됩니다.");
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
                    for (c, t) in COLORS.iter().zip(["영감", "기록", "생각"]) {
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
        ui.heading("수집함");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.extra.board_tab, 0, "관계");
            ui.selectable_value(&mut self.extra.board_tab, 1, "트리보기");
        });
        if self.extra.board_tab == 1 {
            self.relation_tree(ui);
            return;
        }
        ui.checkbox(&mut self.extra.created_basis, "생성시간 표시");
        ui.horizontal(|ui| {
            ui.label("Ctrl + 휠 확대/축소");
            if ui
                .add(egui::Slider::new(&mut self.extra.zoom, 0.4..=1.8).text("확대"))
                .changed()
            {
                let center = self.extra.scene.center();
                self.extra.scene =
                    Rect::from_center_size(center, self.extra.scene_size / self.extra.zoom);
            }
            if ui.button("메모 자동정렬").clicked() {
                self.auto_layout();
            }
        });
        ui.label(
            RichText::new(
                "카드 제목은 이동 · ≡를 다른 카드로 끌면 종속 · 선택 후 연결 / 프로젝트 묶기",
            )
            .small()
            .color(MUTED),
        );
        ui.horizontal_wrapped(|ui| {
            for (i, t) in ["모든 메모", "선택한 메모", "라벨별 보기"]
                .iter()
                .enumerate()
            {
                ui.selectable_value(&mut self.board_filter, i, *t);
            }
            if self.board_filter == 2 {
                egui::ComboBox::from_id_salt("board_label")
                    .selected_text(LABELS[self.label_filter])
                    .show_ui(ui, |ui| {
                        for (i, t) in LABELS.iter().enumerate() {
                            ui.selectable_value(&mut self.label_filter, i, *t);
                        }
                    });
            }
            if ui.button("+ 메모").clicked() {
                self.add_note(None);
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{}개 선택", self.picked.len()));
            if ui.button("선택 연결").clicked() {
                if self.picked.len() == 2 {
                    self.data.connect(self.picked[0], self.picked[1]);
                } else {
                    self.toast = "연결할 메모 2개를 선택하세요".into();
                }
            }
            if ui.button("선택 연결 해제").clicked() && self.picked.len() == 2 {
                let (a, b) = (self.picked[0], self.picked[1]);
                self.data.links.retain(|e| *e != (a.min(b), a.max(b)));
            }
            if ui.button("첫 선택 아래로 종속").clicked() && self.picked.len() >= 2 {
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
            if ui.button("선택 해제").clicked() {
                self.picked.clear();
                self.link_source = None;
            }
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.project_name)
                    .desired_width(150.)
                    .hint_text("프로젝트 이름"),
            );
            if ui.button("선택 메모를 프로젝트로 묶기").clicked() {
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
        });
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
                                2 => self.data.notes[i].label == self.label_filter,
                                _ => true,
                            }
                    })
                    .collect();
                let size = Vec2::new(254., if self.extra.created_basis { 220. } else { 184. });
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
                            ui.painter()
                                .arrow(a, b - a, Stroke::new(2.0_f32, COLORS[1]));
                        }
                    }
                }
                for i in ids {
                    let rect = rects[i];
                    let selected = self.picked.contains(&i);
                    let fill = if self.dark {
                        Color32::from_rgb(39, 40, 38)
                    } else {
                        Color32::WHITE
                    };
                    ui.painter().rect(
                        rect,
                        8.,
                        fill,
                        Stroke::new(
                            if selected { 2.0_f32 } else { 1.0_f32 },
                            if selected {
                                ACCENT
                            } else {
                                Color32::from_gray(195)
                            },
                        ),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().rect_filled(
                        Rect::from_min_size(rect.min, Vec2::new(size.x, 5.)),
                        3.,
                        COLORS[self.data.notes[i].label],
                    );
                    overlay(ui, rect.shrink(11.), i + 2000, |ui| {
                        ui.set_clip_rect(ui.clip_rect().intersect(rect));
                        ui.horizontal(|ui| {
                            let mut s = self.picked.contains(&i);
                            if ui.checkbox(&mut s, "").changed() {
                                if s {
                                    self.picked.push(i)
                                } else {
                                    self.picked.retain(|&j| j != i)
                                }
                            }
                            let drag = ui
                                .add(egui::Label::new("≡").sense(Sense::drag()))
                                .on_hover_text("다른 카드로 끌면 자식 메모가 됩니다");
                            drag.dnd_set_drag_payload(NoteDrag);
                            self.drag_source(ui, &drag, i, false);
                            let grip = ui.add_sized(
                                [130., 24.],
                                egui::Label::new(if self.data.notes[i].done {
                                    RichText::new(shorten(&self.data.notes[i].title, 9))
                                        .strong()
                                        .strikethrough()
                                } else {
                                    RichText::new(shorten(&self.data.notes[i].title, 9)).strong()
                                })
                                .sense(Sense::click_and_drag()),
                            );
                            self.drag_source(ui, &grip, i, true);
                            if grip.double_clicked() {
                                self.open_note(i);
                            }
                            let n = &mut self.data.notes[i];
                            if ui.small_button(if n.starred { "★" } else { "☆" }).clicked() {
                                n.starred = !n.starred;
                            }
                        });
                        ui.label(
                            RichText::new(shorten(&self.data.notes[i].body, 35))
                                .small()
                                .color(MUTED),
                        );
                        let n = &self.data.notes[i];
                        ui.label(
                            RichText::new(format!(
                                "{} · {}",
                                LABELS[n.label],
                                n.project
                                    .map(|p| self.data.projects[p].as_str())
                                    .unwrap_or("프로젝트 없음")
                            ))
                            .small()
                            .color(COLORS[n.label]),
                        );
                        if let Some(&p) = n.parents.first() {
                            ui.label(
                                RichText::new(format!(
                                    "상위: {}",
                                    shorten(&self.data.notes[p].title, 12)
                                ))
                                .small(),
                            );
                        } else {
                            ui.label(RichText::new("독립 메모").small().color(MUTED));
                        }
                        if self.extra.created_basis {
                            ui.label(
                                RichText::new(format!(
                                    "생성 {}",
                                    self.data.notes[i].created.format("%m-%d %H:%M")
                                ))
                                .small()
                                .color(MUTED),
                            );
                        }
                        ui.horizontal(|ui| {
                            let mut d = self.data.notes[i].done;
                            if ui.checkbox(&mut d, "완료").changed() {
                                self.done(i, d);
                                self.extra.scope_owner = ui.ctx().viewport_id();
                            }
                            if ui.small_button("열기").clicked() {
                                self.open_note(i);
                            }
                            if ui
                                .small_button(if self.link_source == Some(i) {
                                    "대상 선택 중"
                                } else {
                                    "연결"
                                })
                                .clicked()
                            {
                                if let Some(s) = self.link_source.take() {
                                    self.data.connect(s, i);
                                } else {
                                    self.link_source = Some(i);
                                    self.toast = "다른 카드의 연결 버튼을 누르세요".into();
                                }
                            }
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
