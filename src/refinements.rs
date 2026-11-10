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
            self.extra.scope_owner = egui::ViewportId::ROOT;
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
            egui::Window::new("기간 일정 완료 범위")
                .id(egui::Id::new("span_scope"))
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(format!("{} · {}", self.data.notes[i].title, d));
                    ui.label(if value {
                        "어느 일정을 완료할까요?"
                    } else {
                        "어느 일정의 완료를 해제할까요?"
                    });
                    if ui.button("이 날짜만").clicked() {
                        self.complete_span(i, d, value, false, Local::now().naive_local());
                        self.extra.completion_scope = None;
                    }
                    if ui.button("이 날짜 이후 모두").clicked() {
                        self.complete_span(i, d, value, true, Local::now().naive_local());
                        self.extra.completion_scope = None;
                    }
                    if ui.button("취소").clicked() {
                        self.extra.completion_scope = None;
                    }
                });
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
        ui.heading("나만의 위키");
        ui.label(
            RichText::new("오른쪽 위키 메모를 왼쪽 폴더로 드래그해 분류하세요.")
                .small()
                .color(MUTED),
        );
        ui.horizontal_wrapped(|ui| {
            if ui.button("+ 새 폴더").clicked() {
                self.create_wiki_folder();
            }
            if ui.button("폴더 선택 해제 · 최상위").clicked() {
                self.extra.folder_selected = None;
            }
            ui.label(
                self.extra
                    .folder_selected
                    .map(|f| format!("새 폴더 위치: {} 아래", self.extra.folders[f].title))
                    .unwrap_or("새 폴더 위치: 최상위".into()),
            );
            if ui
                .checkbox(&mut self.extra.wiki_unfiled, "미분류만 보기")
                .changed()
            {
                self.extra.wiki_selected = None;
            }
            if ui.button("메모 목록").clicked() {
                self.extra.wiki_selected = None;
            }
        });
        ui.separator();
        ui.columns(2, |cols| {
            let root = cols[0].add(
                egui::Label::new("폴더 트리 · 여기로 드롭하면 최상위 / 미분류")
                    .sense(egui::Sense::click()),
            );
            if root.clicked() {
                self.extra.folder_selected = None;
            }
            self.wiki_drop(&cols[0], root.rect, None);
            let roots: Vec<_> = self
                .extra
                .folders
                .iter()
                .enumerate()
                .filter(|(_, f)| f.parent.is_none())
                .map(|(i, _)| i)
                .collect();
            for f in roots {
                self.folder_tree(&mut cols[0], f);
            }
            if let Some(f) = self.extra.folder_selected {
                cols[0].separator();
                cols[0].label("폴더 제목");
                cols[0].text_edit_singleline(&mut self.extra.folders[f].title);
                cols[0].label("폴더 설명");
                cols[0].text_edit_multiline(&mut self.extra.folders[f].description);
            }
            if let Some(i) = self
                .extra
                .wiki_selected
                .filter(|&i| self.visible(i) && self.data.notes[i].wiki)
            {
                cols[1].heading(&self.data.notes[i].title);
                cols[1].separator();
                cols[1].label(if self.data.notes[i].locked {
                    "잠긴 메모"
                } else {
                    &self.data.notes[i].body
                });
                cols[1].add_space(15.);
                cols[1].strong("댓글");
                self.comments(&mut cols[1], i, 0);
            } else {
                cols[1].heading(if self.extra.wiki_unfiled {
                    "미분류 위키 메모"
                } else {
                    "위키 메모 목록"
                });
                let mut ids: Vec<_> = self
                    .data
                    .notes
                    .iter()
                    .enumerate()
                    .filter(|(i, n)| {
                        self.visible(*i)
                            && n.wiki
                            && (!self.extra.wiki_unfiled || n.folder.is_none())
                    })
                    .map(|(i, _)| i)
                    .collect();
                ids.sort_by_key(|&i| self.data.notes[i].created);
                for i in ids {
                    cols[1].push_id(("wiki_list", i), |ui| {
                        let r = ui
                            .add(
                                egui::Label::new(&self.data.notes[i].title)
                                    .sense(egui::Sense::click_and_drag()),
                            )
                            .on_hover_text("클릭: 독립 편집창 · 드래그: 왼쪽 폴더로 분류");
                        self.drag_source(ui, &r, i, false);
                        if r.clicked() {
                            self.open_note(i);
                        }
                        ui.label(
                            RichText::new(
                                self.data.notes[i]
                                    .folder
                                    .map(|f| self.extra.folders[f].title.as_str())
                                    .unwrap_or("미분류"),
                            )
                            .small()
                            .color(MUTED),
                        );
                        ui.separator();
                    });
                }
            }
        });
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
        let row = ui.horizontal(|ui| {
            if ui.small_button(if open { "▼" } else { "▶" }).clicked() {
                if open {
                    self.extra.folder_open.remove(&f);
                } else {
                    self.extra.folder_open.insert(f);
                }
            }
            let r = ui.add(
                egui::Label::new(RichText::new(title).strong().color(
                    if self.extra.folder_selected == Some(f) {
                        ACCENT
                    } else {
                        ui.visuals().text_color()
                    },
                ))
                .sense(egui::Sense::click_and_drag()),
            );
            if r.clicked() {
                self.extra.folder_selected = Some(f);
            }
            self.folder_source(ui, &r, f);
        });
        self.wiki_drop(ui, row.response.rect, Some(f));
        if open {
            ui.indent(("wiki_folder", f), |ui| {
                let ids: Vec<_> = self
                    .data
                    .notes
                    .iter()
                    .enumerate()
                    .filter(|(i, n)| self.visible(*i) && n.wiki && n.folder == Some(f))
                    .map(|(i, _)| i)
                    .collect();
                for i in ids {
                    let r = ui.add(
                        egui::Label::new(&self.data.notes[i].title)
                            .sense(egui::Sense::click_and_drag()),
                    );
                    self.drag_source(ui, &r, i, false);
                    if r.clicked() {
                        self.extra.wiki_selected = Some(i);
                    }
                    if !self.data.notes[i].locked && !self.data.notes[i].wiki_comments.is_empty() {
                        egui::CollapsingHeader::new(format!(
                            "댓글 {}개",
                            self.data.notes[i].wiki_comments.len()
                        ))
                        .id_salt(("wiki_comments", f, i))
                        .show(ui, |ui| {
                            for c in &self.data.notes[i].wiki_comments {
                                ui.label(c);
                            }
                        });
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
        ui.heading("업무일지");
        ui.horizontal_wrapped(|ui| {
            if ui.button("댓글 모두 펼치기").clicked() {
                self.extra.journal_all = true;
                self.extra.journal_open.clear();
            }
            if ui.button("댓글 모두 숨기기").clicked() {
                self.extra.journal_all = false;
                self.extra.journal_open.clear();
            }
            self.date_filter_button(
                ui,
                1,
                "날짜 선택",
                NaiveDate::parse_from_str(&self.journal_date, "%Y-%m-%d").ok(),
            );
        });
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.journal_month, false, "날짜별");
            ui.selectable_value(&mut self.journal_month, true, "월별");
            if ui
                .add(egui::TextEdit::singleline(&mut self.journal_date).desired_width(120.))
                .clicked()
            {
                self.extra.picker = Some(crate::extras::Picker::new(
                    1,
                    NaiveDate::parse_from_str(&self.journal_date, "%Y-%m-%d")
                        .unwrap_or(Local::now().date_naive()),
                    chrono::NaiveTime::MIN,
                ));
            }
        });
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
        ui.strong(format!("{}개 완료", rows.len()));
        for (i, d, t) in rows {
            ui.push_id(("journal", i, d), |ui| {
                ui.separator();
                ui.strong(&self.data.notes[i].title);
                if let Some(d) = d {
                    ui.label(RichText::new(format!("일정 날짜 {d}")).small().color(MUTED));
                }
                ui.label(if self.data.notes[i].locked {
                    "잠긴 메모"
                } else {
                    &self.data.notes[i].body
                });
                ui.horizontal(|ui| {
                    ui.label(t.format("%Y-%m-%d %H:%M").to_string());
                    if ui.button("내용 / 완료 시각 수정").clicked() {
                        self.open_note_on(i, d);
                    }
                });
                let open = *self
                    .extra
                    .journal_open
                    .get(&(i, d))
                    .unwrap_or(&self.extra.journal_all);
                if ui
                    .button(if open {
                        "댓글 숨기기"
                    } else {
                        "댓글 펼치기"
                    })
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
