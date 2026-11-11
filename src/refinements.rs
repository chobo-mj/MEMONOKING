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
/// A large, clickable choice row: icon badge, title and one line of detail.
fn scope_card(
    ui: &mut egui::Ui,
    icon: crate::icons::Icon,
    title: &str,
    detail: &str,
    primary: bool,
) -> egui::Response {
    let tone = crate::ui::t(ui);
    let (rect, r) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 62.), egui::Sense::click());
    let fill = match (primary, r.hovered()) {
        (true, true) => crate::ui::lerp_color(ACCENT, Color32::BLACK, 0.08),
        (true, false) => ACCENT,
        (false, true) => tone.hover,
        (false, false) => tone.sidebar,
    };
    ui.painter().rect_filled(rect, 12., fill);
    if !primary {
        ui.painter().rect_stroke(
            rect,
            12.,
            egui::Stroke::new(1.0_f32, tone.line),
            egui::StrokeKind::Inside,
        );
    }
    let (ink, sub, badge_fill, mark) = if primary {
        (
            Color32::WHITE,
            Color32::from_white_alpha(215),
            Color32::from_white_alpha(40),
            Color32::WHITE,
        )
    } else {
        (tone.ink, tone.muted, tone.accent_soft, ACCENT)
    };
    let badge = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 32., rect.center().y),
        egui::vec2(34., 34.),
    );
    ui.painter().rect_filled(badge, 10., badge_fill);
    crate::icons::paint_icon(ui.painter(), badge.shrink(8.), icon, mark);
    let x = badge.right() + 14.;
    ui.painter().text(
        egui::pos2(x, rect.center().y - 9.),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(15.),
        ink,
    );
    ui.painter().text(
        egui::pos2(x, rect.center().y + 11.),
        egui::Align2::LEFT_CENTER,
        detail,
        egui::FontId::proportional(12.),
        sub,
    );
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
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
            let n = &self.data.notes[i];
            let first = n.date.unwrap_or(d);
            let last = first + chrono::Duration::days(i64::from(n.duration.max(1)) - 1);
            let title = n.title.clone();
            let color = n.color;
            let (verb, heading) = if value {
                ("완료", "어디까지 완료할까요?")
            } else {
                ("완료 해제", "어디까지 완료를 해제할까요?")
            };
            let span = format!(
                "{} ~ {}",
                first.format("%-m월 %-d일"),
                last.format("%-m월 %-d일")
            );
            let mut choice = None;
            let mut cancel = false;
            crate::ui::dialog(
                ctx,
                "span_scope",
                "기간 일정 완료",
                [400., 290.],
                true,
                &mut open,
                |ui| {
                    let tone = crate::ui::t(ui);
                    ui.horizontal(|ui| {
                        let (bar, _) =
                            ui.allocate_exact_size(egui::vec2(5., 38.), egui::Sense::hover());
                        ui.painter().rect_filled(bar, 3., color);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(heading).strong().size(16.).color(tone.ink));
                            crate::ui::hint(ui, format!("{title}  ·  {span}"));
                        });
                    });
                    ui.add_space(14.);
                    let one = format!("{} 하루만 {verb}합니다", d.format("%-m월 %-d일"));
                    if scope_card(ui, crate::icons::Icon::Today, "이 날짜만", &one, false).clicked()
                    {
                        choice = Some(false);
                    }
                    ui.add_space(8.);
                    let all = format!("{span} 모두 {verb}합니다");
                    if scope_card(ui, crate::icons::Icon::Calendar, "전체 일정", &all, true)
                        .clicked()
                    {
                        choice = Some(true);
                    }
                    ui.add_space(10.);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        cancel = ui.button("취소").clicked();
                    });
                },
            );
            if let Some(all) = choice {
                let now = Local::now().naive_local();
                if all {
                    self.complete_span(i, first, value, true, now);
                    if self.active == i {
                        self.extra.edit_occurrence = Some(d);
                    }
                } else {
                    self.complete_span(i, d, value, false, now);
                }
                self.extra.completion_scope = None;
            }
            if cancel {
                self.extra.completion_scope = None;
            }
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
        if let Some(p) = parent {
            self.set_folder_tab(folder, self.extra.folders[p].tab);
        }
        true
    }
    /// Move a folder (and everything in it) to the root of wiki `tab`.
    pub fn move_folder_to_tab(&mut self, folder: usize, tab: usize) {
        if folder < self.extra.folders.len() {
            self.extra.folders[folder].parent = None;
            self.set_folder_tab(folder, tab);
        }
    }
    fn set_folder_tab(&mut self, folder: usize, tab: usize) {
        self.extra.folders[folder].tab = tab;
        for n in &mut self.data.notes {
            if n.folder == Some(folder) {
                n.wiki_tab = Some(tab);
            }
        }
        let children: Vec<_> = (0..self.extra.folders.len())
            .filter(|&c| self.extra.folders[c].parent == Some(folder))
            .collect();
        for c in children {
            self.set_folder_tab(c, tab);
        }
    }
    /// A new folder in the current wiki tab, under `parent` when that folder
    /// belongs to this tab.
    pub fn create_wiki_folder(&mut self, parent: Option<usize>) -> usize {
        let id = self.extra.folders.len();
        let tab = self.extra.wiki_tab;
        let parent = parent.filter(|&p| self.extra.folders[p].tab == tab);
        self.extra.folders.push(crate::extras::Folder {
            title: format!("새 폴더 {}", id + 1),
            description: String::new(),
            parent,
            tab,
        });
        self.extra.folder_open.insert(id);
        if let Some(p) = parent {
            self.extra.folder_open.insert(p);
        }
        id
    }
    /// My wiki: tabs on top; on the left the tab's folder tree (①) over the
    /// unfiled wiki memos (②); on the right two areas that show memos side
    /// by side for comparison.
    pub fn wiki(&mut self, ui: &mut egui::Ui) {
        if self.extra.wiki_tab >= self.extra.wiki_tabs.len() {
            self.extra.wiki_tab = 0;
        }
        for pane in &mut self.extra.wiki_panes {
            if pane.is_some_and(|i| i >= self.data.notes.len() || self.data.notes[i].deleted) {
                *pane = None;
            }
        }
        if self.extra.wiki_panes[1].is_none() {
            self.extra.wiki_blank = false;
        }
        self.wiki_tabs_bar(ui);
        ui.add_space(6.);
        let tone = crate::ui::t(ui);
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect(
            rect,
            12.,
            tone.bg,
            egui::Stroke::new(1.0_f32, tone.line),
            egui::StrokeKind::Inside,
        );
        let inner = rect.shrink(8.);
        let left_w = (inner.width() * 0.22).clamp(210., 280.);
        let left = egui::Rect::from_min_size(inner.min, Vec2::new(left_w, inner.height()));
        let tree_h = (left.height() * 0.58).round();
        let tree_rect = egui::Rect::from_min_size(left.min, Vec2::new(left_w, tree_h));
        let unfiled_rect =
            egui::Rect::from_min_max(egui::pos2(left.left(), left.top() + tree_h + 1.), left.max);
        ui.painter().line_segment(
            [
                egui::pos2(left.left() + 6., left.top() + tree_h),
                egui::pos2(left.right() - 6., left.top() + tree_h),
            ],
            egui::Stroke::new(1.0_f32, tone.line),
        );
        ui.painter().line_segment(
            [
                egui::pos2(left.right() + 4., rect.top()),
                egui::pos2(left.right() + 4., rect.bottom()),
            ],
            egui::Stroke::new(1.0_f32, tone.line),
        );
        self.in_rect(ui, tree_rect, "wiki_tree_area", |a, ui| a.wiki_tree(ui));
        self.in_rect(ui, unfiled_rect, "wiki_unfiled_area", |a, ui| {
            a.wiki_unfiled(ui)
        });
        let right =
            egui::Rect::from_min_max(egui::pos2(left.right() + 12., inner.top()), inner.max);
        let gap = 10.;
        let pane_w = (right.width() - gap) / 2.;
        for k in 0..2 {
            let pane = egui::Rect::from_min_size(
                egui::pos2(right.left() + k as f32 * (pane_w + gap), right.top()),
                Vec2::new(pane_w, right.height()),
            );
            self.in_rect(ui, pane, ("wiki_pane", k), |a, ui| a.wiki_pane(ui, k));
        }
        ui.allocate_rect(rect, egui::Sense::hover());
    }
    /// Run `add` in a child ui confined to `rect`.
    fn in_rect(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        id: impl std::hash::Hash,
        add: impl FnOnce(&mut Self, &mut egui::Ui),
    ) {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(id)
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        child.set_clip_rect(ui.clip_rect().intersect(rect));
        add(self, &mut child);
    }
    fn wiki_tabs_bar(&mut self, ui: &mut egui::Ui) {
        let tone = crate::ui::t(ui);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.;
            for t in 0..self.extra.wiki_tabs.len() {
                if self.extra.wiki_rename == Some(crate::extras::WikiRename::Tab(t)) {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.extra.wiki_tabs[t]).desired_width(90.),
                    );
                    r.request_focus();
                    if r.lost_focus() {
                        self.extra.wiki_rename = None;
                    }
                    continue;
                }
                let on = self.extra.wiki_tab == t;
                let text = egui::RichText::new(&self.extra.wiki_tabs[t])
                    .size(13.)
                    .color(if on { egui::Color32::WHITE } else { tone.ink });
                let r = ui
                    .add(
                        egui::Button::new(text)
                            .fill(if on { ACCENT } else { tone.accent_soft })
                            .corner_radius(egui::CornerRadius {
                                nw: 8,
                                ne: 8,
                                sw: 0,
                                se: 0,
                            })
                            .min_size(Vec2::new(72., 30.)),
                    )
                    .on_hover_text("누르면 이 위키의 트리 보기 · 더블클릭: 이름 바꾸기");
                // Switching tabs changes only the tree; open memos stay.
                if r.clicked() {
                    self.extra.wiki_tab = t;
                    self.extra.folder_selected = None;
                }
                if r.double_clicked() {
                    self.extra.wiki_rename = Some(crate::extras::WikiRename::Tab(t));
                }
                r.context_menu(|ui| {
                    if ui.button("이름 바꾸기").clicked() {
                        self.extra.wiki_rename = Some(crate::extras::WikiRename::Tab(t));
                        ui.close_menu();
                    }
                });
                self.targets.push(crate::dragging::Target {
                    viewport: ui.ctx().viewport_id(),
                    layer: ui.layer_id(),
                    rect: r.rect.intersect(ui.clip_rect()),
                    kind: crate::dragging::Destination::WikiTab(t),
                });
            }
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("+").size(14.))
                        .fill(tone.accent_soft)
                        .min_size(Vec2::new(32., 30.)),
                )
                .on_hover_text("새 위키 탭")
                .clicked()
            {
                let n = self.extra.wiki_tabs.len() + 1;
                self.extra.wiki_tabs.push(format!("위키 {n}"));
                self.extra.wiki_tab = n - 1;
                self.extra.folder_selected = None;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(egui::Button::new(RichText::new("+  새 폴더").strong()))
                    .clicked()
                {
                    self.create_wiki_folder(self.extra.folder_selected);
                }
                crate::ui::hint(
                    ui,
                    "미분류 메모를 위 트리로 끌어 분류 · 트리에서 우클릭: 새 노트",
                );
            });
        });
    }
    /// ① The current tab's folders and filed memos.
    fn wiki_tree(&mut self, ui: &mut egui::Ui) {
        let tab = self.extra.wiki_tab;
        let area = ui.max_rect();
        // The empty part of the tree is the tab's root: drop and right-click.
        let bg = ui.interact(area, ui.id().with("tree_bg"), egui::Sense::click());
        self.targets.push(crate::dragging::Target {
            viewport: ui.ctx().viewport_id(),
            layer: ui.layer_id(),
            rect: area.intersect(ui.clip_rect()),
            kind: crate::dragging::Destination::WikiTab(tab),
        });
        let mut new_note = None;
        let mut new_folder = None;
        bg.context_menu(|ui| {
            if ui.button("새 위키 노트").clicked() {
                new_note = Some(None);
                ui.close_menu();
            }
            if ui.button("새 폴더").clicked() {
                new_folder = Some(None);
                ui.close_menu();
            }
        });
        egui::ScrollArea::vertical()
            .id_salt("wiki_tree_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(4.);
                let roots: Vec<_> = (0..self.extra.folders.len())
                    .filter(|&f| {
                        self.extra.folders[f].parent.is_none() && self.extra.folders[f].tab == tab
                    })
                    .collect();
                for f in roots {
                    self.folder_tree(ui, f, &mut new_note, &mut new_folder);
                }
                let notes: Vec<_> = self
                    .data
                    .notes
                    .iter()
                    .enumerate()
                    .filter(|(i, n)| {
                        self.visible(*i) && n.wiki && n.folder.is_none() && n.wiki_tab == Some(tab)
                    })
                    .map(|(i, _)| i)
                    .collect();
                for i in notes {
                    self.wiki_note_row(ui, i, 30.);
                }
            });
        if let Some(folder) = new_folder {
            self.create_wiki_folder(folder);
        }
        if let Some(folder) = new_note {
            let i = self.new_wiki_note(
                folder.map_or(Some(tab), |f| Some(self.extra.folders[f].tab)),
                folder,
            );
            self.wiki_open(i);
        }
    }
    /// ② Wiki memos not filed in any tab, newest first.
    fn wiki_unfiled(&mut self, ui: &mut egui::Ui) {
        let tone = crate::ui::t(ui);
        let area = ui.max_rect();
        self.wiki_drop(ui, area, None);
        let mut ids: Vec<_> = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(i, n)| {
                self.visible(*i) && n.wiki && n.folder.is_none() && n.wiki_tab.is_none()
            })
            .map(|(i, _)| i)
            .collect();
        ids.sort_by_key(|&i| std::cmp::Reverse(self.data.notes[i].created));
        ui.add_space(8.);
        ui.horizontal(|ui| {
            ui.add_space(8.);
            ui.label(RichText::new("미분류 위키").size(12.).color(tone.muted));
            ui.label(
                RichText::new(ids.len().to_string())
                    .size(12.)
                    .color(tone.faint),
            );
        });
        ui.add_space(2.);
        egui::ScrollArea::vertical()
            .id_salt("wiki_unfiled_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if ids.is_empty() {
                    ui.add_space(10.);
                    crate::ui::hint(ui, "  메모의 ‘위키’ 표시를 켜면 여기에 모입니다");
                }
                for i in ids {
                    self.wiki_note_row(ui, i, 28.);
                }
            });
    }
    fn wiki_note_row(&mut self, ui: &mut egui::Ui, i: usize, height: f32) {
        let n = &self.data.notes[i];
        let title = if n.title.trim().is_empty() {
            "제목 없음".to_owned()
        } else {
            n.title.clone()
        };
        let meta = if n.wiki_comments.is_empty() {
            String::new()
        } else {
            format!("댓글 {}", n.wiki_comments.len())
        };
        let open = self.extra.wiki_panes[self.extra.wiki_focus] == Some(i);
        let r = crate::ui::list_row(
            ui,
            ui.id().with(("wiki_note", i)),
            open,
            Some(n.color),
            &title,
            &meta,
            height,
        )
        .on_hover_text("누르면 선택한 영역에 열기 · 끌어서 분류");
        self.drag_source(ui, &r, i, false);
        if r.clicked() {
            self.wiki_open(i);
        }
    }
    /// A new, empty wiki memo filed in `tab` / `folder` (both `None`: unfiled).
    fn new_wiki_note(&mut self, tab: Option<usize>, folder: Option<usize>) -> usize {
        let i = self.data.notes.len();
        let mut n = Note::new("", None, i);
        n.body.clear();
        n.wiki = true;
        n.wiki_tab = tab;
        n.folder = folder;
        self.data.notes.push(n);
        if let Some(f) = folder {
            self.extra.folder_open.insert(f);
        }
        i
    }
    /// Show memo `i` in the selected area. While area 2 holds a memo that was
    /// opened blank, picked memos go to area 1 so area 2 keeps it.
    pub fn wiki_open(&mut self, i: usize) {
        let k = if self.extra.wiki_blank {
            0
        } else {
            self.extra.wiki_focus
        };
        self.extra.wiki_panes[k] = Some(i);
    }
    fn wiki_pane(&mut self, ui: &mut egui::Ui, k: usize) {
        let tone = crate::ui::t(ui);
        let rect = ui.max_rect();
        let focused = self.extra.wiki_focus == k;
        if ui.rect_contains_pointer(rect) && ui.input(|i| i.pointer.any_pressed()) {
            self.extra.wiki_focus = k;
        }
        // Header: which area this is, and its tools.
        let mut close = false;
        ui.horizontal(|ui| {
            ui.set_min_height(26.);
            let label = if k == 0 { "영역 1" } else { "영역 2" };
            let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.), egui::Sense::hover());
            ui.painter().circle_filled(
                dot.center(),
                4.,
                if focused { ACCENT } else { tone.line_strong },
            );
            ui.label(RichText::new(label).size(12.).color(if focused {
                ACCENT
            } else {
                tone.muted
            }));
            if focused {
                crate::ui::hint(ui, "· 고른 메모가 여기에 열립니다");
            }
            if let Some(i) = self.extra.wiki_panes[k] {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::icons::small_icon(ui, crate::icons::Icon::Close, "영역 닫기")
                        .clicked()
                    {
                        close = true;
                    }
                    let comments = self.data.notes[i].wiki_comments.len();
                    let tip = format!("댓글 {comments}");
                    if crate::icons::toggle_icon(
                        ui,
                        crate::icons::Icon::Notes,
                        &tip,
                        self.extra.wiki_comments[k],
                    )
                    .clicked()
                    {
                        self.extra.wiki_comments[k] = !self.extra.wiki_comments[k];
                    }
                    if crate::icons::small_icon(ui, crate::icons::Icon::Sticky, "스티커로 열기")
                        .clicked()
                    {
                        self.open_note(i);
                    }
                });
            }
        });
        let body = egui::Rect::from_min_max(egui::pos2(rect.left(), rect.top() + 32.), rect.max);
        if focused {
            ui.painter().rect_stroke(
                body.expand(1.),
                19.,
                egui::Stroke::new(1.5_f32, tone.accent_soft),
                egui::StrokeKind::Outside,
            );
        }
        match self.extra.wiki_panes[k] {
            Some(i) => {
                let comments = self.extra.wiki_comments[k];
                let note_h = if comments {
                    (body.height() * 0.6).round()
                } else {
                    body.height()
                };
                let note = egui::Rect::from_min_size(body.min, Vec2::new(body.width(), note_h));
                self.in_rect(ui, note, ("wiki_note_pane", k), |a, ui| {
                    ui.set_height(note.height());
                    a.inline_note(ui, i)
                });
                if comments {
                    let area = egui::Rect::from_min_max(
                        egui::pos2(body.left(), note.bottom() + 8.),
                        body.max,
                    );
                    self.in_rect(ui, area, ("wiki_comments_pane", k), |a, ui| {
                        egui::ScrollArea::vertical()
                            .id_salt(("wiki_comments_scroll", k))
                            .auto_shrink([false, false])
                            .show(ui, |ui| a.comments(ui, i, 0));
                    });
                }
            }
            None => {
                ui.painter().rect_filled(body, 18., tone.sidebar);
                self.in_rect(ui, body, ("wiki_empty_pane", k), |a, ui| {
                    ui.add_space(body.height() * 0.36);
                    ui.vertical_centered(|ui| {
                        if k == 1 {
                            let (r, resp) =
                                ui.allocate_exact_size(Vec2::splat(56.), egui::Sense::click());
                            let fill = if resp.hovered() {
                                tone.accent_soft
                            } else {
                                tone.bg
                            };
                            ui.painter().circle_filled(r.center(), 28., fill);
                            crate::icons::paint_icon(
                                ui.painter(),
                                r.shrink(14.),
                                crate::icons::Icon::Plus,
                                ACCENT,
                            );
                            ui.add_space(8.);
                            ui.label(RichText::new("빈 메모 열기").color(tone.muted));
                            if resp
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                let i = a.new_wiki_note(None, None);
                                a.extra.wiki_panes[1] = Some(i);
                                a.extra.wiki_blank = true;
                                a.extra.wiki_focus = 0;
                            }
                        } else {
                            crate::ui::empty_state(
                                ui,
                                crate::icons::Icon::Book,
                                "메모를 고르세요",
                                "왼쪽 트리나 미분류 목록에서 메모를 누르면 여기에 열립니다",
                            );
                        }
                    });
                });
            }
        }
        if close {
            if let Some(i) = self.extra.wiki_panes[k].take() {
                // A blank memo left empty is not kept.
                let n = &self.data.notes[i];
                if k == 1
                    && self.extra.wiki_blank
                    && n.title.trim().is_empty()
                    && n.body.trim().is_empty()
                {
                    self.data.notes[i].deleted = true;
                }
            }
            if k == 1 {
                self.extra.wiki_blank = false;
            }
        }
    }
    fn wiki_drop(&mut self, ui: &egui::Ui, rect: egui::Rect, f: Option<usize>) {
        self.targets.push(crate::dragging::Target {
            viewport: ui.ctx().viewport_id(),
            layer: ui.layer_id(),
            rect: rect.intersect(ui.clip_rect()),
            kind: crate::dragging::Destination::WikiFolder(f),
        });
    }
    fn folder_tree(
        &mut self,
        ui: &mut egui::Ui,
        f: usize,
        new_note: &mut Option<Option<usize>>,
        new_folder: &mut Option<Option<usize>>,
    ) {
        let open = self.extra.folder_open.contains(&f);
        let ids: Vec<_> = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(i, n)| self.visible(*i) && n.wiki && n.folder == Some(f))
            .map(|(i, _)| i)
            .collect();
        if self.extra.wiki_rename == Some(crate::extras::WikiRename::Folder(f)) {
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.extra.folders[f].title)
                    .desired_width(f32::INFINITY),
            );
            r.request_focus();
            if r.lost_focus() {
                self.extra.wiki_rename = None;
            }
        } else {
            let title = self.extra.folders[f].title.clone();
            let (r, toggled) = crate::ui::tree_row(
                ui,
                ui.id().with(("wiki_folder_row", f)),
                self.extra.folder_selected == Some(f),
                Some(open),
                crate::icons::Icon::Folder,
                &title,
                ids.len(),
            );
            if toggled || r.clicked() {
                if open {
                    self.extra.folder_open.remove(&f);
                } else {
                    self.extra.folder_open.insert(f);
                }
                self.extra.folder_selected = Some(f);
            }
            let description = self.extra.folders[f].description.clone();
            let r = if description.trim().is_empty() {
                r
            } else {
                r.on_hover_text(description)
            };
            r.context_menu(|ui| {
                if ui.button("새 위키 노트").clicked() {
                    *new_note = Some(Some(f));
                    ui.close_menu();
                }
                if ui.button("새 하위 폴더").clicked() {
                    *new_folder = Some(Some(f));
                    ui.close_menu();
                }
                if ui.button("이름 바꾸기").clicked() {
                    self.extra.wiki_rename = Some(crate::extras::WikiRename::Folder(f));
                    ui.close_menu();
                }
            });
            self.folder_source(ui, &r, f);
            self.wiki_drop(ui, r.rect, Some(f));
        }
        if open {
            ui.indent(("wiki_folder", f), |ui| {
                // The folder's memo area also files dropped memos into it.
                let top = ui.cursor().top();
                for i in ids {
                    self.wiki_note_row(ui, i, 28.);
                }
                let children: Vec<_> = (0..self.extra.folders.len())
                    .filter(|&c| self.extra.folders[c].parent == Some(f))
                    .collect();
                let notes_bottom = ui.cursor().top();
                if notes_bottom > top {
                    let area =
                        egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), top..=notes_bottom);
                    self.wiki_drop(ui, area, Some(f));
                }
                for c in children {
                    self.folder_tree(ui, c, new_note, new_folder);
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
        let x = a.create_wiki_folder(None);
        let y = a.create_wiki_folder(None);
        assert_eq!(a.extra.folders[x].parent, None);
        assert_eq!(a.extra.folders[y].parent, None);
        let z = a.create_wiki_folder(Some(x));
        assert_eq!(a.extra.folders[z].parent, Some(x));
        assert!(!a.move_folder(x, Some(z)));
        assert_eq!(a.extra.folders[x].parent, None);
        assert!(a.move_folder(y, Some(x)));
    }
    #[test]
    fn wiki_areas_keep_a_blank_memo_in_area_two_and_tabs_only_change_the_tree() {
        let mut a = app();
        a.data.notes[1].wiki = true;
        a.extra.wiki_focus = 1;
        a.wiki_open(0);
        assert_eq!(a.extra.wiki_panes, [None, Some(0)]);
        // A blank memo in area 2 stays; picked memos then go to area 1.
        let blank = a.new_wiki_note(None, None);
        a.extra.wiki_panes[1] = Some(blank);
        a.extra.wiki_blank = true;
        a.extra.wiki_focus = 1;
        a.wiki_open(1);
        assert_eq!(a.extra.wiki_panes, [Some(1), Some(blank)]);
        let n = &a.data.notes[blank];
        assert!(n.wiki && n.folder.is_none() && n.wiki_tab.is_none());
        // A second tab: folders made there and moved there follow the tab.
        a.extra.wiki_tabs.push("위키 2".into());
        a.extra.wiki_tab = 1;
        let f = a.create_wiki_folder(Some(0));
        assert_eq!(
            (a.extra.folders[f].tab, a.extra.folders[f].parent),
            (1, None)
        );
        a.move_folder_to_tab(0, 1);
        assert_eq!(a.extra.folders[0].tab, 1);
        assert_eq!(a.data.notes[0].wiki_tab, Some(1));
        assert_eq!(a.extra.wiki_panes, [Some(1), Some(blank)]);
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
