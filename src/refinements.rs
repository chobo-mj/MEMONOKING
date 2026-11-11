use crate::*;
use chrono::NaiveDateTime;
pub fn ordered_range(a: NaiveDate, b: NaiveDate) -> (NaiveDate, NaiveDate) {
    (a.min(b), a.max(b))
}
pub fn project_match(filter: Option<usize>, p: Option<usize>) -> bool {
    filter.is_none_or(|f| {
        if f == NO_PROJECT {
            p.is_none()
        } else {
            p == Some(f)
        }
    })
}
pub fn project_label(filter: Option<usize>, names: &[String]) -> &str {
    match filter {
        None => "모든 프로젝트",
        Some(NO_PROJECT) => "프로젝트 없음",
        Some(p) => names.get(p).map(String::as_str).unwrap_or("프로젝트 없음"),
    }
}
impl App {
    pub fn done_at(&mut self, i: usize, day: Option<NaiveDate>, value: bool) {
        let n = &self.data.notes[i];
        if n.duration > 1 && n.date.is_some() {
            let today = Local::now().date_naive();
            let day = day.unwrap_or_else(|| {
                if n.occurs(today) {
                    today
                } else {
                    n.days()
                        .into_iter()
                        .find(|d| !n.occurrence_completed.contains_key(d))
                        .unwrap_or(n.date.unwrap())
                }
            });
            self.extra.completion_scope = Some((i, day, value));
            self.extra.scope_owner = main_viewport();
        } else {
            self.data.complete(i, value, Local::now());
        }
    }
    pub fn complete_span(
        &mut self,
        i: usize,
        date: NaiveDate,
        value: bool,
        future: bool,
        now: NaiveDateTime,
    ) {
        let n = &mut self.data.notes[i];
        for d in n.days() {
            if d == date || (future && d >= date) {
                if value {
                    n.occurrence_completed.entry(d).or_insert(now);
                } else {
                    n.occurrence_completed.remove(&d);
                }
            }
        }
        n.refresh_done();
        if self.active == i {
            self.extra.edit_occurrence = Some(date);
            self.completion_edit = n
                .occurrence_completed
                .get(&date)
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();
        }
    }
    pub fn scope_dialog(&mut self, ctx: &egui::Context) {
        if ctx.viewport_id() != self.extra.scope_owner {
            return;
        }
        if let Some((i, d, value)) = self.extra.completion_scope {
            let mut open = true;
            crate::ui::dialog(
                ctx,
                "span_scope",
                "기간 일정 완료",
                [360., 220.],
                true,
                &mut open,
                |ui| {
                    ui.label(
                        RichText::new(if value {
                            "어느 일정을 완료할까요?"
                        } else {
                            "어느 일정의 완료를 해제할까요?"
                        })
                        .strong()
                        .size(15.),
                    );
                    crate::ui::hint(ui, format!("{} · {}", self.data.notes[i].title, d));
                    ui.add_space(10.);
                    ui.horizontal(|ui| {
                        if ui.button("이 날짜만").clicked() {
                            self.complete_span(i, d, value, false, Local::now().naive_local());
                            self.extra.completion_scope = None;
                        }
                        if ui.button("이 날짜 이후 모두").clicked() {
                            self.complete_span(i, d, value, true, Local::now().naive_local());
                            self.extra.completion_scope = None;
                        }
                    });
                    ui.add_space(6.);
                    if ui.button("취소").clicked() {
                        self.extra.completion_scope = None;
                    }
                },
            );
            if !open {
                self.extra.completion_scope = None;
            }
        }
    }
    pub fn set_completion_time(&mut self, t: NaiveDateTime) {
        if self.editor_completion().is_none() {
            return;
        }
        let i = self.active;
        if self.data.notes[i].duration > 1 {
            if let Some(completed) = self
                .extra
                .edit_occurrence
                .and_then(|d| self.data.notes[i].occurrence_completed.get_mut(&d))
            {
                *completed = t;
            }
            self.data.notes[i].refresh_done();
        } else {
            self.data.notes[i].completed = Some(t);
        }
        self.completion_edit = t.format("%Y-%m-%d %H:%M").to_string();
    }
    pub fn open_note_on(&mut self, i: usize, d: Option<NaiveDate>) {
        self.open_note(i);
        self.extra.edit_occurrence = d.filter(|_| self.data.notes[i].duration > 1);
        if let Some(w) = self.studio.windows.get_mut(&i) {
            w.occurrence = self.extra.edit_occurrence;
        }
        if let Some(d) = self.extra.edit_occurrence {
            self.completion_edit = self.data.notes[i]
                .occurrence_completed
                .get(&d)
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();
        }
    }
    /// Open memo `i` in the memo editor window rather than as a sticker.
    pub fn open_editor_on(&mut self, i: usize, d: Option<NaiveDate>) {
        self.open_note_on(i, d);
        if let Some(w) = self.studio.windows.get_mut(&i) {
            w.edit();
        }
    }
    pub fn drag_source_on(
        &mut self,
        ui: &egui::Ui,
        r: &egui::Response,
        note: usize,
        date: Option<NaiveDate>,
    ) {
        self.drag_source(ui, r, note, false);
        if let Some(s) = self.sources.last_mut() {
            s.occurrence = date;
        }
    }
    pub fn folder_source(&mut self, ui: &egui::Ui, r: &egui::Response, f: usize) {
        self.sources.push(crate::dragging::Source {
            viewport: ui.ctx().viewport_id(),
            layer: ui.layer_id(),
            rect: r.rect.intersect(ui.clip_rect()),
            note: f,
            card: false,
            folder: true,
            occurrence: None,
        });
    }
    pub fn move_folder(&mut self, folder: usize, parent: Option<usize>) -> bool {
        if folder >= self.extra.folders.len()
            || parent.is_some_and(|p| p >= self.extra.folders.len())
        {
            return false;
        }
        let mut cursor = parent;
        let mut visited = std::collections::HashSet::new();
        while let Some(p) = cursor {
            if p == folder || !visited.insert(p) {
                return false;
            }
            cursor = self.extra.folders[p].parent;
        }
        self.extra.folders[folder].parent = parent;
        true
    }
    pub fn create_wiki_folder(&mut self) -> usize {
        let id = self.extra.folders.len();
        self.extra.folders.push(crate::extras::Folder {
            title: format!("새 폴더 {}", id + 1),
            description: String::new(),
            parent: self.extra.folder_selected,
        });
        self.extra.folder_open.insert(id);
        if let Some(p) = self.extra.folder_selected {
            self.extra.folder_open.insert(p);
        }
        id
    }
    pub fn wiki(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new(RichText::new("+  새 폴더").strong()))
                .clicked()
            {
                self.create_wiki_folder();
            }
            if crate::ui::pill(ui, self.extra.wiki_unfiled, "미분류만").clicked() {
                self.extra.wiki_unfiled = !self.extra.wiki_unfiled;
                self.extra.wiki_selected = None;
            }
            crate::ui::hint(
                ui,
                self.extra
                    .folder_selected
                    .map(|f| {
                        format!(
                            "새 폴더는 ‘{}’ 아래에 만들어집니다",
                            self.extra.folders[f].title
                        )
                    })
                    .unwrap_or("위키 메모를 왼쪽 폴더로 끌어 분류하세요".into()),
            );
        });
        ui.add_space(12.);
        let height = ui.available_height().max(320.);
        let left_w = (ui.available_width() * 0.32).clamp(220., 300.);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(left_w, height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_width(left_w);
                    self.wiki_sidebar(ui);
                },
            );
            ui.add_space(18.);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_width(ui.available_width());
                    if let Some(i) = self
                        .extra
                        .wiki_selected
                        .filter(|&i| self.visible(i) && self.data.notes[i].wiki)
                    {
                        self.wiki_article(ui, i);
                    } else {
                        self.wiki_list(ui);
                    }
                },
            );
        });
    }
    fn wiki_sidebar(&mut self, ui: &mut egui::Ui) {
        let count = self.extra.folders.len();
        crate::ui::section_header(ui, crate::icons::Icon::Folder, "폴더", Some(count));
        crate::ui::card(ui, |ui| {
            let all = self
                .data
                .notes
                .iter()
                .enumerate()
                .filter(|(i, n)| self.visible(*i) && n.wiki)
                .count();
            let (root, _) = crate::ui::tree_row(
                ui,
                ui.id().with("wiki_root"),
                self.extra.folder_selected.is_none(),
                None,
                crate::icons::Icon::Book,
                "모든 위키",
                all,
            );
            let root = root.on_hover_text("여기로 놓으면 최상위 / 미분류");
            if root.clicked() {
                self.extra.folder_selected = None;
                self.extra.wiki_selected = None;
            }
            self.wiki_drop(ui, root.rect, None);
            let roots: Vec<_> = self
                .extra
                .folders
                .iter()
                .enumerate()
                .filter(|(_, f)| f.parent.is_none())
                .map(|(i, _)| i)
                .collect();
            if roots.is_empty() {
                crate::ui::empty_state(
                    ui,
                    crate::icons::Icon::Folder,
                    "폴더가 없습니다",
                    "‘+ 새 폴더’로 위키를 묶어 보세요",
                );
            }
            for f in roots {
                self.folder_tree(ui, f);
            }
        });
        if let Some(f) = self.extra.folder_selected {
            ui.add_space(14.);
            crate::ui::section_header(ui, crate::icons::Icon::Edit, "폴더 정보", None);
            crate::ui::card(ui, |ui| {
                crate::ui::caption(ui, "제목");
                ui.add(
                    egui::TextEdit::singleline(&mut self.extra.folders[f].title)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(6.);
                crate::ui::caption(ui, "설명");
                ui.add(
                    egui::TextEdit::multiline(&mut self.extra.folders[f].description)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );
            });
        }
    }
    fn wiki_list(&mut self, ui: &mut egui::Ui) {
        let mut ids: Vec<_> = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(i, n)| {
                self.visible(*i)
                    && n.wiki
                    && (!self.extra.wiki_unfiled || n.folder.is_none())
                    && self
                        .extra
                        .folder_selected
                        .is_none_or(|f| n.folder == Some(f))
            })
            .map(|(i, _)| i)
            .collect();
        ids.sort_by_key(|&i| std::cmp::Reverse(self.data.notes[i].created));
        let title = match self.extra.folder_selected {
            Some(f) => self.extra.folders[f].title.clone(),
            None if self.extra.wiki_unfiled => "미분류 위키".into(),
            None => "모든 위키".into(),
        };
        crate::ui::section_header(ui, crate::icons::Icon::Book, &title, Some(ids.len()));
        if let Some(f) = self.extra.folder_selected {
            let d = &self.extra.folders[f].description;
            if !d.trim().is_empty() {
                crate::ui::hint(ui, d.clone());
                ui.add_space(6.);
            }
        }
        crate::ui::card(ui, |ui| {
            ui.set_width(ui.available_width());
            if ids.is_empty() {
                crate::ui::empty_state(
                    ui,
                    crate::icons::Icon::Book,
                    "위키 메모가 없습니다",
                    "메모의 ‘위키’ 표시를 켜면 여기에 모입니다",
                );
            }
            for i in ids {
                ui.push_id(("wiki_list", i), |ui| {
                    let tone = crate::ui::t(ui);
                    let (rect, r) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 56.),
                        egui::Sense::click_and_drag(),
                    );
                    if r.hovered() {
                        ui.painter().rect_filled(rect, 8., tone.hover);
                    }
                    let n = &self.data.notes[i];
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            rect.left_top() + Vec2::new(8., 12.),
                            Vec2::new(4., rect.height() - 24.),
                        ),
                        2.,
                        n.color,
                    );
                    let x = rect.left() + 22.;
                    let clip = egui::Rect::from_min_max(
                        egui::pos2(x, rect.top()),
                        egui::pos2(rect.right() - 8., rect.bottom()),
                    );
                    let painter = ui.painter().with_clip_rect(clip);
                    painter.text(
                        egui::pos2(x, rect.top() + 9.),
                        egui::Align2::LEFT_TOP,
                        &n.title,
                        egui::FontId::proportional(14.5),
                        tone.ink,
                    );
                    let folder = n
                        .folder
                        .map(|f| self.extra.folders[f].title.as_str())
                        .unwrap_or("미분류");
                    let first = if n.locked {
                        "잠긴 메모"
                    } else {
                        n.body.lines().find(|l| !l.trim().is_empty()).unwrap_or("")
                    };
                    painter.text(
                        egui::pos2(x, rect.bottom() - 9.),
                        egui::Align2::LEFT_BOTTOM,
                        format!(
                            "{folder}  ·  {}  ·  {}",
                            n.created.format("%Y.%m.%d"),
                            crate::shorten(first, 60)
                        ),
                        egui::FontId::proportional(11.5),
                        tone.muted,
                    );
                    let r = r
                        .on_hover_text("클릭: 읽기 · 더블클릭: 메모 열기 · 드래그: 폴더로 분류")
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    self.drag_source(ui, &r, i, false);
                    if r.double_clicked() {
                        self.open_note(i);
                    } else if r.clicked() {
                        self.extra.wiki_selected = Some(i);
                    }
                });
            }
        });
    }
    fn wiki_article(&mut self, ui: &mut egui::Ui, i: usize) {
        let tone = crate::ui::t(ui);
        ui.horizontal(|ui| {
            if crate::icons::small_icon(ui, crate::icons::Icon::ChevronLeft, "목록으로").clicked()
            {
                self.extra.wiki_selected = None;
            }
            crate::ui::hint(ui, "위키 목록");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("메모 열기").clicked() {
                    self.open_note(i);
                }
            });
        });
        ui.add_space(10.);
        let n = &self.data.notes[i];
        ui.label(RichText::new(&n.title).size(22.).strong().color(tone.ink));
        ui.add_space(2.);
        let folder = n
            .folder
            .map(|f| self.extra.folders[f].title.clone())
            .unwrap_or("미분류".into());
        crate::ui::hint(
            ui,
            format!(
                "{folder}  ·  {} 작성  ·  댓글 {}",
                n.created.format("%Y.%m.%d"),
                n.wiki_comments.len()
            ),
        );
        ui.add_space(10.);
        crate::ui::divider(ui);
        ui.add_space(12.);
        if n.locked {
            crate::ui::empty_state(ui, crate::icons::Icon::Lock, "잠긴 메모입니다", "");
        } else {
            let body = n.body.clone();
            ui.label(RichText::new(body).size(14.5).color(tone.ink));
        }
        ui.add_space(20.);
        crate::ui::section_header(ui, crate::icons::Icon::Notes, "댓글", None);
        self.comments(ui, i, 0);
    }
    fn wiki_drop(&mut self, ui: &egui::Ui, rect: egui::Rect, f: Option<usize>) {
        self.targets.push(crate::dragging::Target {
            viewport: ui.ctx().viewport_id(),
            layer: ui.layer_id(),
            rect: rect.intersect(ui.clip_rect()),
            kind: crate::dragging::Destination::WikiFolder(f),
        });
    }
    fn folder_tree(&mut self, ui: &mut egui::Ui, f: usize) {
        let title = self.extra.folders[f].title.clone();
        let open = self.extra.folder_open.contains(&f);
        let ids: Vec<_> = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(i, n)| self.visible(*i) && n.wiki && n.folder == Some(f))
            .map(|(i, _)| i)
            .collect();
        let (r, toggled) = crate::ui::tree_row(
            ui,
            ui.id().with(("wiki_folder_row", f)),
            self.extra.folder_selected == Some(f),
            Some(open),
            crate::icons::Icon::Folder,
            &title,
            ids.len(),
        );
        if toggled {
            if open {
                self.extra.folder_open.remove(&f);
            } else {
                self.extra.folder_open.insert(f);
            }
        } else if r.clicked() {
            self.extra.folder_selected = Some(f);
            self.extra.wiki_selected = None;
        }
        self.folder_source(ui, &r, f);
        self.wiki_drop(ui, r.rect, Some(f));
        if open {
            ui.indent(("wiki_folder", f), |ui| {
                for i in ids {
                    let n = &self.data.notes[i];
                    let meta = if n.wiki_comments.is_empty() {
                        String::new()
                    } else {
                        format!("댓글 {}", n.wiki_comments.len())
                    };
                    let r = crate::ui::list_row(
                        ui,
                        ui.id().with(("wiki_note", i)),
                        self.extra.wiki_selected == Some(i),
                        Some(n.color),
                        &n.title.clone(),
                        &meta,
                        30.,
                    );
                    self.drag_source(ui, &r, i, false);
                    if r.clicked() {
                        self.extra.wiki_selected = Some(i);
                    }
                }
                let children: Vec<_> = self
                    .extra
                    .folders
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| n.parent == Some(f))
                    .map(|(i, _)| i)
                    .collect();
                for c in children {
                    self.folder_tree(ui, c);
                }
            });
        }
    }
}
impl App {
    pub fn journal_v2(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut month = usize::from(self.journal_month);
            if crate::ui::segmented(ui, &mut month, &["날짜별", "월별"]) {
                self.journal_month = month == 1;
            }
            ui.add_space(6.);
            let date = NaiveDate::parse_from_str(&self.journal_date, "%Y-%m-%d").ok();
            let label = date
                .map(|d| {
                    if self.journal_month {
                        d.format("%Y년 %-m월").to_string()
                    } else {
                        format!("{} {}", d.format("%Y.%m.%d"), crate::ui::weekday(d))
                    }
                })
                .unwrap_or("날짜 선택".into());
            if crate::icons::text_button(ui, crate::icons::Icon::Calendar, &label, "날짜 선택")
                .clicked()
            {
                self.extra.picker = Some(crate::extras::Picker::new(
                    1,
                    date.unwrap_or(Local::now().date_naive()),
                    chrono::NaiveTime::MIN,
                ));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("댓글 모두 숨기기").clicked() {
                    self.extra.journal_all = false;
                    self.extra.journal_open.clear();
                }
                if ui.button("댓글 모두 펼치기").clicked() {
                    self.extra.journal_all = true;
                    self.extra.journal_open.clear();
                }
            });
        });
        ui.add_space(10.);
        let Ok(date) = NaiveDate::parse_from_str(&self.journal_date, "%Y-%m-%d") else {
            ui.colored_label(ACCENT, "날짜를 선택하세요");
            return;
        };
        let mut rows = vec![];
        let matches = |t: NaiveDateTime| {
            if self.journal_month {
                t.year() == date.year() && t.month() == date.month()
            } else {
                t.date() == date
            }
        };
        for (i, n) in self.data.notes.iter().enumerate() {
            if !self.visible(i) {
                continue;
            }
            if n.duration > 1 {
                for (&d, &t) in &n.occurrence_completed {
                    if matches(t) {
                        rows.push((i, Some(d), t));
                    }
                }
            } else if n.done {
                if let Some(t) = n.completed.filter(|&t| matches(t)) {
                    rows.push((i, None, t));
                }
            }
        }
        rows.sort_by_key(|r| r.2);
        crate::ui::section_header(ui, crate::icons::Icon::Today, "완료한 일", Some(rows.len()));
        if rows.is_empty() {
            crate::ui::card(ui, |ui| {
                ui.set_width(ui.available_width());
                crate::ui::empty_state(
                    ui,
                    crate::icons::Icon::Today,
                    "이 기간에 완료한 메모가 없습니다",
                    "메모를 완료하면 완료 시각과 함께 여기에 기록됩니다",
                );
            });
        }
        for (i, d, t) in rows {
            ui.push_id(("journal", i, d), |ui| {
                crate::ui::card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let tone = crate::ui::t(ui);
                    ui.horizontal(|ui| {
                        crate::ui::chip(
                            ui,
                            &t.format("%H:%M").to_string(),
                            tone.accent_soft,
                            ACCENT,
                        );
                        ui.add_space(4.);
                        ui.label(RichText::new(&self.data.notes[i].title).strong().size(14.));
                        if let Some(d) = d {
                            crate::ui::hint(ui, format!("일정 {d}"));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if crate::icons::small_icon(
                                ui,
                                crate::icons::Icon::Edit,
                                "내용 / 완료 시각 수정",
                            )
                            .clicked()
                            {
                                self.open_note_on(i, d);
                            }
                            crate::ui::hint(ui, t.format("%Y-%m-%d").to_string());
                        });
                    });
                    ui.add_space(2.);
                    ui.label(
                        RichText::new(if self.data.notes[i].locked {
                            "잠긴 메모".to_owned()
                        } else {
                            crate::composer::strip_pictures(&self.data.notes[i].body)
                        })
                        .size(13.)
                        .color(tone.muted),
                    );
                    ui.add_space(4.);
                    let open = *self
                        .extra
                        .journal_open
                        .get(&(i, d))
                        .unwrap_or(&self.extra.journal_all);
                    if crate::ui::pill(
                        ui,
                        open,
                        if open {
                            "댓글 숨기기"
                        } else {
                            "댓글 펼치기"
                        },
                    )
                    .clicked()
                    {
                        self.extra.journal_open.insert((i, d), !open);
                    }
                    if open {
                        self.extra.comment_occurrence = d;
                        self.comments(ui, i, 1);
                        self.extra.comment_occurrence = None;
                    }
                });
                ui.add_space(8.);
            });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        App::sample(NaiveDate::from_ymd_opt(2026, 11, 9).unwrap())
    }
    #[test]
    fn no_project_filter_is_distinct_from_all() {
        assert!(project_match(None, Some(0)));
        assert!(project_match(Some(NO_PROJECT), None));
        assert!(!project_match(Some(NO_PROJECT), Some(0)));
        assert_eq!(project_label(Some(NO_PROJECT), &[]), "프로젝트 없음");
    }
    #[test]
    fn date_range_normalizes_reverse_dates() {
        let a = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let b = a + chrono::Duration::days(3);
        assert_eq!(ordered_range(b, a), (a, b));
        assert_eq!(ordered_range(a, a), (a, a));
    }
    #[test]
    fn new_folder_keeps_context_for_multiple_siblings() {
        let mut a = app();
        a.extra.folder_selected = None;
        let x = a.create_wiki_folder();
        let y = a.create_wiki_folder();
        assert_eq!(a.extra.folders[x].parent, None);
        assert_eq!(a.extra.folders[y].parent, None);
        a.extra.folder_selected = Some(x);
        let z = a.create_wiki_folder();
        assert_eq!(a.extra.folders[z].parent, Some(x));
        assert!(!a.move_folder(x, Some(z)));
        assert_eq!(a.extra.folders[x].parent, None);
        assert!(a.move_folder(y, Some(x)));
    }
    #[test]
    fn three_day_scope_does_not_complete_without_choice() {
        let mut a = app();
        a.data.notes[0].duration = 3;
        let d = a.data.notes[0].date.unwrap() + chrono::Duration::days(1);
        a.done_at(0, Some(d), true);
        assert_eq!(a.extra.completion_scope, Some((0, d, true)));
        assert!(a.data.notes[0].occurrence_completed.is_empty());
    }
    #[test]
    fn this_day_and_future_scope_preserve_prior_days() {
        let mut a = app();
        let start = a.data.notes[0].date.unwrap();
        a.data.notes[0].duration = 3;
        let time = start.and_hms_opt(17, 0, 0).unwrap();
        a.complete_span(0, start + chrono::Duration::days(1), true, false, time);
        assert!(!a.data.notes[0].done_on(Some(start)));
        assert!(a.data.notes[0].done_on(Some(start + chrono::Duration::days(1))));
        assert!(!a.data.notes[0].done);
        a.complete_span(0, start + chrono::Duration::days(1), true, true, time);
        assert_eq!(a.data.notes[0].occurrence_completed.len(), 2);
        a.complete_span(0, start, true, false, time);
        assert!(a.data.notes[0].done);
        a.complete_span(0, start + chrono::Duration::days(1), false, true, time);
        assert_eq!(a.data.notes[0].occurrence_completed.len(), 1);
    }
    #[test]
    fn span_crosses_month_boundary_without_cloning_note() {
        let mut a = app();
        let count = a.data.notes.len();
        a.data.notes[0].date = NaiveDate::from_ymd_opt(2026, 12, 31);
        a.data.notes[0].duration = 3;
        assert!(a.data.notes[0].occurs(NaiveDate::from_ymd_opt(2027, 1, 2).unwrap()));
        assert!(!a.data.notes[0].occurs(NaiveDate::from_ymd_opt(2027, 1, 3).unwrap()));
        assert_eq!(a.data.notes.len(), count);
    }
    #[test]
    fn rescheduling_period_preserves_completion_and_comment_identity() {
        let mut a = app();
        let start = a.data.notes[0].date.unwrap();
        a.data.notes[0].duration = 3;
        let d = start + chrono::Duration::days(1);
        let t = d.and_hms_opt(10, 0, 0).unwrap();
        a.complete_span(0, d, true, false, t);
        a.data.notes[0]
            .occurrence_comments
            .insert(d, vec!["완료 댓글".into()]);
        a.data.notes[0].move_start(start + chrono::Duration::days(5));
        assert_eq!(
            a.data.notes[0].occurrence_completed[&(d + chrono::Duration::days(5))],
            t
        );
        assert_eq!(
            a.data.notes[0].occurrence_comments[&(d + chrono::Duration::days(5))],
            vec!["완료 댓글"]
        );
    }
}

impl App {
    pub fn editor_completion(&self) -> Option<NaiveDateTime> {
        let n = &self.data.notes[self.active];
        if n.duration > 1 {
            self.extra
                .edit_occurrence
                .and_then(|d| n.occurrence_completed.get(&d).copied())
        } else if n.done {
            n.completed
        } else {
            None
        }
    }
}
#[cfg(test)]
mod completion_editor_tests {
    use super::*;
    #[test]
    fn correction_is_scoped_to_selected_completed_day() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let mut a = App::sample(d);
        a.data.notes[0].duration = 3;
        let t0 = d.and_hms_opt(9, 0, 0).unwrap();
        let d1 = d + chrono::Duration::days(1);
        let t1 = d1.and_hms_opt(10, 0, 0).unwrap();
        a.complete_span(0, d, true, false, t0);
        a.complete_span(0, d1, true, false, t1);
        a.open_note_on(0, Some(d));
        assert_eq!(a.editor_completion(), Some(t0));
        a.open_note_on(0, Some(d + chrono::Duration::days(2)));
        assert!(a.editor_completion().is_none());
    }
    #[test]
    fn stale_time_correction_cannot_complete_an_unchecked_note_or_day() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let t = d.and_hms_opt(9, 0, 0).unwrap();
        let corrected = d.and_hms_opt(10, 0, 0).unwrap();
        let mut a = App::sample(d);
        a.data.notes[0].done = true;
        a.data.notes[0].completed = Some(t);
        a.open_note_on(0, None);
        a.data.complete(0, false, Local::now());
        a.set_completion_time(corrected);
        assert!(!a.data.notes[0].done);
        assert!(a.data.notes[0].completed.is_none());

        a.data.notes[0].date = Some(d);
        a.data.notes[0].duration = 3;
        a.complete_span(0, d, true, false, t);
        a.open_note_on(0, Some(d));
        a.complete_span(0, d, false, false, t);
        a.set_completion_time(corrected);
        assert!(!a.data.notes[0].done_on(Some(d)));
        assert!(!a.data.notes[0].occurrence_completed.contains_key(&d));

        a.complete_span(0, d, true, false, t);
        a.set_completion_time(corrected);
        assert_eq!(
            a.data.notes[0].occurrence_completed.get(&d),
            Some(&corrected)
        );
    }
}
