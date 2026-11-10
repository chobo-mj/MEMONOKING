//! Folder organization, detailed note lists and the in-memory recycle bin.
use crate::*;
use egui::{Sense, Stroke};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

type FindHit = (usize, Option<(usize, Range<usize>)>);

#[derive(Clone, Debug)]
pub struct Folder {
    pub title: String,
    pub parent: Option<usize>,
    pub open: bool,
    pub deleted: bool,
}

#[derive(Default)]
pub struct Catalog {
    pub tab: usize,
    pub folders: Vec<Folder>,
    pub assignments: BTreeMap<usize, usize>,
    pub selected: Option<usize>,
    pub selected_day: Option<NaiveDate>,
    pub selected_many: BTreeSet<usize>,
    pub folder_selected: Option<usize>,
    pub find_selection: Option<(usize, usize, Range<usize>)>,
    pub query: String,
    pub replace_open: bool,
    pub replace_find: String,
    pub replace_with: String,
    pub replace_scope: usize,
    project: Option<usize>,
    sort: usize,
    recurring: usize,
    wiki_only: bool,
    rename: Option<usize>,
    rename_focus: bool,
    find_index: usize,
    find_focus: bool,
    replace_message: String,
    purge_tab: Option<(usize, usize)>,
    drag_sources: Vec<(egui::Rect, FolderDrag)>,
    drag_targets: Vec<(egui::Rect, Option<usize>)>,
    dragging: Option<(FolderDrag, egui::Pos2)>,
    table_offset: f32,
    table_viewport_width: f32,
}

const COLUMN_WIDTHS: [f32; 8] = [32., 32., 180., 240., 140., 85., 140., 32.];
const TABLE_WIDTH: f32 = 881.;

#[derive(Clone, Copy)]
enum TableRow {
    Note(usize, Option<NaiveDate>),
    Tab(usize, usize),
}

enum TableCommand {
    Toggle(usize),
    Complete(usize, Option<NaiveDate>, bool),
    Reminder(usize),
    Open(usize, Option<NaiveDate>),
    Trash(usize),
    Restore(usize),
    Purge(usize),
    RestoreTab(usize, usize),
    PurgeTab(usize, usize),
}

fn table_cells(rect: egui::Rect) -> [egui::Rect; 8] {
    let mut x = rect.left();
    std::array::from_fn(|i| {
        let r = egui::Rect::from_min_size(
            egui::pos2(x, rect.top()),
            Vec2::new(COLUMN_WIDTHS[i], rect.height()),
        );
        x += COLUMN_WIDTHS[i];
        r
    })
}

fn table_cell(ui: &mut egui::Ui, rect: egui::Rect, salt: impl std::hash::Hash) -> egui::Ui {
    let rect = rect.shrink2(Vec2::new(3., 2.));
    let mut cell = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(salt)
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    cell.set_clip_rect(ui.clip_rect().intersect(rect));
    cell.spacing_mut().item_spacing.x = 4.;
    cell
}

impl Catalog {
    pub fn focus_find(&mut self) {
        self.find_focus = true;
    }

    pub fn create_folder(&mut self) -> usize {
        let parent = self
            .folder_selected
            .filter(|&f| self.folders.get(f).is_some_and(|folder| !folder.deleted));
        let id = self.folders.len();
        self.folders.push(Folder {
            title: "새 폴더".into(),
            parent,
            open: true,
            deleted: false,
        });
        if let Some(p) = parent {
            self.folders[p].open = true;
        }
        // Keep the selected parent, so another click creates a sibling.
        self.rename = Some(id);
        self.rename_focus = true;
        id
    }

    pub fn move_folder(&mut self, folder: usize, parent: Option<usize>) -> bool {
        if self.folders.get(folder).is_none_or(|f| f.deleted) {
            return false;
        }
        let mut cursor = parent;
        let mut visited = BTreeSet::new();
        while let Some(p) = cursor {
            if p == folder || !visited.insert(p) {
                return false;
            }
            let Some(node) = self.folders.get(p).filter(|f| !f.deleted) else {
                return false;
            };
            cursor = node.parent;
        }
        self.folders[folder].parent = parent;
        if let Some(p) = parent {
            self.folders[p].open = true;
        }
        true
    }

    pub fn assign_notes(&mut self, notes: &[usize], folder: Option<usize>) {
        if folder.is_some_and(|f| self.folders.get(f).is_none_or(|f| f.deleted)) {
            return;
        }
        for &note in notes {
            if let Some(f) = folder {
                self.assignments.insert(note, f);
            } else {
                self.assignments.remove(&note);
            }
        }
    }

    fn remove_folder(&mut self, f: usize) {
        let parent = self.folders[f].parent;
        self.folders[f].deleted = true;
        for folder in &mut self.folders {
            if folder.parent == Some(f) {
                folder.parent = parent;
            }
        }
        self.assignments.retain(|_, v| *v != f);
        if self.folder_selected == Some(f) {
            self.folder_selected = None;
        }
    }
}

#[derive(Clone)]
enum FolderDrag {
    Notes(Vec<usize>),
    Folder(usize),
}

#[derive(Clone, Copy)]
enum Action {
    Eye,
    Bell,
    Trash,
    Restore,
}

fn action(ui: &mut egui::Ui, kind: Action, tooltip: &str, selected: bool) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(Vec2::splat(26.), Sense::click());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tooltip));
    if r.hovered() || r.has_focus() || selected {
        ui.painter().rect_filled(
            rect,
            5.,
            if selected {
                Color32::from_rgba_unmultiplied(184, 72, 53, 18)
            } else {
                ui.visuals().widgets.hovered.weak_bg_fill
            },
        );
    }
    let c = if selected { ACCENT } else { MUTED };
    let s = Stroke::new(1.35_f32, c);
    let p = |x: f32, y: f32| rect.min + Vec2::new(x, y);
    let line = |a: (f32, f32), b: (f32, f32)| {
        ui.painter().line_segment([p(a.0, a.1), p(b.0, b.1)], s);
    };
    match kind {
        Action::Eye => {
            ui.painter().add(egui::Shape::line(
                vec![
                    p(3., 13.),
                    p(7., 8.),
                    p(13., 6.),
                    p(19., 8.),
                    p(23., 13.),
                    p(19., 18.),
                    p(13., 20.),
                    p(7., 18.),
                    p(3., 13.),
                ],
                s,
            ));
            ui.painter().circle_stroke(p(13., 13.), 3.2, s);
        }
        Action::Bell => {
            ui.painter().add(egui::Shape::line(
                vec![
                    p(5., 19.),
                    p(8., 16.),
                    p(8., 11.),
                    p(9., 7.),
                    p(13., 5.),
                    p(17., 7.),
                    p(18., 11.),
                    p(18., 16.),
                    p(21., 19.),
                    p(5., 19.),
                ],
                s,
            ));
            line((11., 22.), (15., 22.));
            line((13., 3.), (13., 5.));
        }
        Action::Trash => {
            line((5., 7.), (21., 7.));
            line((10., 4.), (16., 4.));
            ui.painter().add(egui::Shape::line(
                vec![p(7., 9.), p(8., 22.), p(18., 22.), p(19., 9.)],
                s,
            ));
            line((11., 11.), (11., 19.));
            line((15., 11.), (15., 19.));
        }
        Action::Restore => {
            ui.painter().add(egui::Shape::line(
                vec![
                    p(7., 8.),
                    p(13., 5.),
                    p(20., 8.),
                    p(22., 14.),
                    p(19., 20.),
                    p(12., 22.),
                    p(7., 19.),
                ],
                s,
            ));
            line((4., 4.), (4., 11.));
            line((4., 11.), (11., 11.));
        }
    }
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

impl App {
    pub fn catalog_ui(&mut self, ui: &mut egui::Ui) {
        self.catalog.drag_sources.clear();
        self.catalog.drag_targets.clear();
        ui.heading("전체메모");
        ui.add_space(10.);
        ui.horizontal(|ui| {
            for (i, name) in ["폴더 보기", "전체 메모 보기", "관계 보드"]
                .iter()
                .enumerate()
            {
                ui.selectable_value(&mut self.catalog.tab, i, *name);
            }
        });
        ui.add_space(12.);
        if self.catalog.tab == 2 {
            self.board(ui);
            return;
        }
        if self.catalog.tab == 0 {
            self.catalog_folders(ui);
        } else {
            self.catalog_controls(ui);
            let ids = self.catalog_filtered_ids();
            self.catalog_find_bar(ui, &ids);
            let rows: Vec<_> = ids
                .into_iter()
                .map(|i| (i, self.data.notes[i].date))
                .collect();
            self.catalog_split_list(ui, &rows, false, false);
        }
        self.catalog_drag_input(ui.ctx());
        self.catalog_replace_window(ui.ctx());
    }

    pub fn detail_workspace(&mut self, ui: &mut egui::Ui, mode: usize) {
        ui.horizontal(|ui| {
            ui.heading(if self.extra.search_mode {
                "검색 결과"
            } else {
                "내 작업실"
            });
            if ui.button("캘린더 나란히").clicked() {
                self.split = true;
            }
        });
        ui.label(
            RichText::new("자세히 보기 · 방향키로 선택 · 제목과 본문을 눌러 편집")
                .small()
                .color(MUTED),
        );
        self.filters(ui);
        ui.add_space(8.);
        let today = Local::now().date_naive();
        let mut rows = vec![];
        for (i, n) in self.data.notes.iter().enumerate() {
            if !self.matches_search(i)
                || (!self.recurring && n.recurring)
                || !crate::refinements::project_match(self.project_filter, n.project)
                || (self.desktop.hide_locked && n.locked)
            {
                continue;
            }
            let days: Vec<_> = if n.date.is_some() {
                n.days().into_iter().map(Some).collect()
            } else {
                vec![None]
            };
            for d in days {
                if (self.desktop.hide_completed && n.done_on(d))
                    || !match mode {
                        2 => d == Some(today),
                        3 => d.is_some_and(|d| d > today),
                        _ => true,
                    }
                {
                    continue;
                }
                rows.push((i, d));
            }
        }
        rows.sort_by_key(|&(i, d)| (d.unwrap_or(NaiveDate::MAX), self.data.notes[i].time, i));
        let ids: Vec<_> = rows.iter().map(|&(i, _)| i).collect();
        self.catalog_find_bar(ui, &ids);
        self.catalog_split_list(ui, &rows, true, true);
        self.catalog_replace_window(ui.ctx());
    }

    fn catalog_filtered_ids(&self) -> Vec<usize> {
        let mut ids: Vec<_> = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(i, n)| {
                self.visible(*i)
                    && crate::refinements::project_match(self.catalog.project, n.project)
                    && (!self.catalog.wiki_only || n.wiki)
                    && match self.catalog.recurring {
                        1 => !n.recurring,
                        2 => n.recurring,
                        _ => true,
                    }
            })
            .map(|(i, _)| i)
            .collect();
        ids.sort_by_key(|&i| {
            let n = &self.data.notes[i];
            if self.catalog.sort == 1 {
                (n.date.unwrap_or(NaiveDate::MAX), n.time, i)
            } else {
                (n.created.date(), n.created.time(), i)
            }
        });
        ids
    }

    fn catalog_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("catalog_project")
                .selected_text(crate::refinements::project_label(
                    self.catalog.project,
                    &self.data.projects,
                ))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.catalog.project, None, "모든 프로젝트");
                    ui.selectable_value(
                        &mut self.catalog.project,
                        Some(NO_PROJECT),
                        "프로젝트 없음",
                    );
                    for (p, name) in self.data.projects.iter().enumerate() {
                        if !self.extra.hidden_projects.contains(&p)
                            && !self.extra.deleted_projects.contains(&p)
                        {
                            ui.selectable_value(&mut self.catalog.project, Some(p), name);
                        }
                    }
                });
            egui::ComboBox::from_id_salt("catalog_sort")
                .selected_text(["생성일자 순", "일정 순"][self.catalog.sort])
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.catalog.sort, 0, "생성일자 순");
                    ui.selectable_value(&mut self.catalog.sort, 1, "일정 순");
                });
            egui::ComboBox::from_id_salt("catalog_recurrence")
                .selected_text(
                    ["정기 일정 포함", "정기 일정 제외", "정기 일정만"][self.catalog.recurring],
                )
                .show_ui(ui, |ui| {
                    for (i, text) in ["정기 일정 포함", "정기 일정 제외", "정기 일정만"]
                        .iter()
                        .enumerate()
                    {
                        ui.selectable_value(&mut self.catalog.recurring, i, *text);
                    }
                });
            ui.checkbox(&mut self.catalog.wiki_only, "위키만 보기");
        });
        ui.add_space(8.);
    }

    fn catalog_find_bar(&mut self, ui: &mut egui::Ui, ids: &[usize]) {
        if ui
            .ctx()
            .input(|i| i.modifiers.command && i.key_pressed(egui::Key::F))
        {
            self.catalog.find_focus = true;
        }
        if ui
            .ctx()
            .input(|i| i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::H))
        {
            self.catalog.replace_open = true;
        }
        let mut direction = 0;
        let mut reset = false;
        ui.horizontal_wrapped(|ui| {
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.catalog.query)
                    .desired_width(200.)
                    .hint_text("이 목록에서 빠른 찾기"),
            );
            if self.catalog.find_focus {
                r.request_focus();
                self.catalog.find_focus = false;
            }
            reset = r.changed();
            if ui.small_button("이전 찾기").clicked() {
                direction = -1;
            }
            if ui.small_button("다음 찾기").clicked()
                || (r.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                direction = 1;
            }
            if ui.small_button("찾아 바꾸기").clicked() {
                self.catalog.replace_open = true;
                self.catalog.replace_find = self.catalog.query.clone();
            }
        });
        let hits = self.catalog_find_hits(ids);
        if reset {
            self.catalog.find_index = 0;
        }
        if !hits.is_empty() {
            let len = hits.len();
            if direction != 0 {
                self.catalog.find_index = if direction > 0 {
                    (self.catalog.find_index + 1) % len
                } else {
                    (self.catalog.find_index + len - 1) % len
                };
            }
            self.catalog.find_index %= len;
            if reset || direction != 0 {
                let (id, location) = hits[self.catalog.find_index].clone();
                self.catalog.selected = Some(id);
                self.catalog.selected_many = BTreeSet::from([id]);
                // Typing a query must keep focus in the search field. Only an
                // explicit next/previous action moves the editor's caret.
                self.catalog.find_selection = if direction != 0 {
                    location.map(|(tab, range)| (id, tab, range))
                } else {
                    None
                };
            }
            ui.label(
                RichText::new(format!("{} / {} 위치", self.catalog.find_index + 1, len))
                    .small()
                    .color(MUTED),
            );
        } else if !self.catalog.query.is_empty() {
            ui.label(
                RichText::new("일치하는 내용을 찾지 못했습니다")
                    .small()
                    .color(MUTED),
            );
        }
        ui.add_space(8.);
    }

    fn catalog_find_hits(&self, ids: &[usize]) -> Vec<FindHit> {
        if self.catalog.query.is_empty() {
            return vec![];
        }
        let mut seen = BTreeSet::new();
        let mut hits = vec![];
        for &i in ids {
            if !seen.insert(i) {
                continue;
            }
            let n = &self.data.notes[i];
            if n.title
                .to_lowercase()
                .contains(&self.catalog.query.to_lowercase())
            {
                hits.push((i, None));
            }
            if !n.locked {
                for (tab, body) in std::iter::once((0, n.body.as_str())).chain(
                    n.document
                        .tabs
                        .iter()
                        .enumerate()
                        .skip(1)
                        .map(|(tab, t)| (tab, t.body.as_str())),
                ) {
                    for range in find_ranges(body, &self.catalog.query) {
                        hits.push((i, Some((tab, range))));
                    }
                }
            }
        }
        hits
    }

    fn catalog_split_list(
        &mut self,
        ui: &mut egui::Ui,
        rows: &[(usize, Option<NaiveDate>)],
        open_editor: bool,
        date_groups: bool,
    ) {
        if self
            .catalog
            .selected
            .is_none_or(|s| !rows.iter().any(|r| r.0 == s))
        {
            self.catalog.selected = rows.first().map(|r| r.0);
        }
        if !ui.ctx().wants_keyboard_input() && !rows.is_empty() {
            let direction = ui.input(|i| {
                if i.key_pressed(egui::Key::ArrowDown) {
                    1
                } else if i.key_pressed(egui::Key::ArrowUp) {
                    -1
                } else {
                    0
                }
            });
            if direction != 0 {
                let at = rows
                    .iter()
                    .position(|r| Some(r.0) == self.catalog.selected)
                    .unwrap_or(0);
                let next = (at as isize + direction).clamp(0, rows.len() as isize - 1) as usize;
                self.catalog.selected = Some(rows[next].0);
                self.catalog.selected_day = rows[next].1;
                self.catalog.selected_many = BTreeSet::from([rows[next].0]);
            }
        }
        let height = panel_height(ui);
        let rect =
            egui::Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), height));
        let preview_width = 300_f32.min(rect.width() * 0.42).max(250.);
        let list_width = (rect.width() - preview_width - 16.).max(180.);
        let table_rows: Vec<_> = rows.iter().map(|&(i, d)| TableRow::Note(i, d)).collect();
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
            |ui| {
                ui.set_min_size(rect.size());
                let list_rect = egui::Rect::from_min_size(rect.min, Vec2::new(list_width, height));
                let preview_rect = egui::Rect::from_min_size(
                    egui::pos2(list_rect.right() + 16., rect.top()),
                    Vec2::new(preview_width, height),
                );
                let mut list = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("detail_table_pane")
                        .max_rect(list_rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                );
                list.set_clip_rect(ui.clip_rect().intersect(list_rect));
                self.catalog_table(
                    &mut list,
                    &table_rows,
                    height,
                    open_editor,
                    false,
                    date_groups,
                );
                let mut preview = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("detail_preview_pane")
                        .max_rect(preview_rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                );
                preview.set_clip_rect(ui.clip_rect().intersect(preview_rect));
                self.catalog_preview(&mut preview, height);
            },
        );
    }

    fn catalog_table(
        &mut self,
        ui: &mut egui::Ui,
        rows: &[TableRow],
        height: f32,
        open_editor: bool,
        trash: bool,
        date_groups: bool,
    ) {
        let header_width = if self.catalog.table_viewport_width > 0. {
            self.catalog.table_viewport_width.min(ui.available_width())
        } else {
            ui.available_width()
        };
        let offset = self
            .catalog
            .table_offset
            .clamp(0., (TABLE_WIDTH - header_width).max(0.));
        egui::ScrollArea::horizontal()
            .id_salt(("table_header", trash))
            .horizontal_scroll_offset(offset)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .auto_shrink([false, false])
            .max_width(header_width)
            .max_height(36.)
            .show(ui, |ui| {
                ui.set_min_width(TABLE_WIDTH);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(TABLE_WIDTH, 36.), Sense::hover());
                ui.painter()
                    .rect_filled(rect, 0., ui.visuals().faint_bg_color);
                ui.painter().line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    Stroke::new(1_f32, ui.visuals().widgets.noninteractive.bg_stroke.color),
                );
                let cells = table_cells(rect);
                for (col, title) in [
                    if trash { "복원" } else { "표시" },
                    "완료",
                    "제목",
                    "내용",
                    "알람",
                    "생성일",
                    "일정",
                    "삭제",
                ]
                .iter()
                .enumerate()
                {
                    let mut cell = table_cell(ui, cells[col], ("header", col));
                    cell.add(
                        egui::Label::new(RichText::new(*title).size(12.).color(MUTED).strong())
                            .truncate(),
                    );
                }
            });
        let mut pending = vec![];
        let body = egui::ScrollArea::both()
            .id_salt(("detail_table_body", trash))
            .horizontal_scroll_offset(offset)
            .auto_shrink([false, false])
            .max_height((height - 44.).max(100.))
            .show(ui, |ui| {
                ui.set_min_width(TABLE_WIDTH);
                if rows.is_empty() {
                    ui.add_space(24.);
                    ui.label(RichText::new("표시할 메모가 없습니다").color(MUTED));
                }
                let mut previous = Some(NaiveDate::MIN);
                for (at, &row) in rows.iter().enumerate() {
                    if let TableRow::Note(_, day) = row {
                        if date_groups && previous != day {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(TABLE_WIDTH, 26.), Sense::hover());
                            ui.painter().text(
                                rect.left_center() + Vec2::new(7., 0.),
                                egui::Align2::LEFT_CENTER,
                                day.map(|d| d.format("%m월 %d일").to_string())
                                    .unwrap_or("일정 없는 메모".into()),
                                egui::FontId::proportional(12.),
                                MUTED,
                            );
                            previous = day;
                        }
                    }
                    self.catalog_table_row(ui, row, at, open_editor, trash, &mut pending);
                }
            });
        self.catalog.table_offset = body.state.offset.x;
        self.catalog.table_viewport_width = body.inner_rect.width();
        for command in pending {
            match command {
                TableCommand::Toggle(i) => self.toggle_sticker(i),
                TableCommand::Complete(i, d, done) => {
                    self.done_at(i, d, done);
                    self.extra.scope_owner = ui.ctx().viewport_id();
                }
                TableCommand::Reminder(i) => self.open_reminder(i),
                TableCommand::Open(i, d) => self.open_note_on(i, d),
                TableCommand::Trash(i) => self.trash_note(i),
                TableCommand::Restore(i) => self.restore_note(i),
                TableCommand::Purge(i) => self.request_purge(i),
                TableCommand::RestoreTab(i, t) => {
                    self.catalog_restore_tab(i, t);
                }
                TableCommand::PurgeTab(i, t) => self.catalog.purge_tab = Some((i, t)),
            }
        }
    }

    fn catalog_table_row(
        &mut self,
        ui: &mut egui::Ui,
        row: TableRow,
        at: usize,
        open_editor: bool,
        trash: bool,
        pending: &mut Vec<TableCommand>,
    ) {
        let (i, day, tab) = match row {
            TableRow::Note(i, d) => (i, d, None),
            TableRow::Tab(i, t) => (i, self.data.notes[i].date, Some(t)),
        };
        let n = &self.data.notes[i];
        let title = if let Some(t) = tab {
            let Some(t) = n.document.trash.get(t) else {
                return;
            };
            format!("{} - {}", n.title, t.title)
        } else {
            n.title.clone()
        };
        let body = if n.locked {
            "잠긴 메모".into()
        } else if let Some(t) = tab {
            n.document.trash[t].body.replace('\n', " ")
        } else {
            n.body.replace('\n', " ")
        };
        let reminder = n.reminder.map(|t| t.format("%m/%d %H:%M").to_string());
        let created = n.created.format("%y%m%d").to_string();
        let schedule = day
            .map(|d| format!("{} {}", d.format("%m/%d"), n.time.format("%H:%M")))
            .unwrap_or("—".into());
        let mut done = n.done_on(day);
        let locked = n.locked;
        let selected = self.catalog.selected == Some(i) || self.catalog.selected_many.contains(&i);
        let height = match self.desktop.row_spacing {
            0 => 38.,
            2 => 58.,
            _ => 48.,
        };
        ui.push_id(("table_row", i, day, tab, trash), |ui| {
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(TABLE_WIDTH, height), Sense::hover());
            let fill = if selected {
                ui.visuals().selection.bg_fill
            } else if at % 2 == 1 {
                ui.visuals().faint_bg_color
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(rect, 0., fill);
            ui.painter().line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                Stroke::new(
                    1_f32,
                    ui.visuals()
                        .widgets
                        .noninteractive
                        .bg_stroke
                        .color
                        .gamma_multiply(0.18),
                ),
            );
            let cells = table_cells(rect);
            {
                let mut c = table_cell(ui, cells[0], "eye");
                if trash {
                    if action(&mut c, Action::Restore, "복원하기", false)
                        .on_hover_text("복원하기")
                        .clicked()
                    {
                        pending.push(if let Some(t) = tab {
                            TableCommand::RestoreTab(i, t)
                        } else {
                            TableCommand::Restore(i)
                        });
                    }
                } else {
                    let opened = self.studio.windows.get(&i).is_some_and(|w| w.open);
                    if action(&mut c, Action::Eye, "스티커 표시 / 숨기기", opened).clicked()
                    {
                        pending.push(TableCommand::Toggle(i));
                    }
                }
            }
            {
                let mut c = table_cell(ui, cells[1], "done");
                if c.add_enabled(!trash, egui::Checkbox::new(&mut done, ""))
                    .on_hover_text("완료 상태")
                    .changed()
                {
                    pending.push(TableCommand::Complete(i, day, done));
                }
            }
            {
                let mut c = table_cell(ui, cells[2], "title");
                let mut text = RichText::new(&title).strong();
                if done {
                    text = text.strikethrough().color(MUTED);
                }
                let r = c
                    .add(
                        egui::Label::new(text)
                            .truncate()
                            .sense(Sense::click_and_drag()),
                    )
                    .on_hover_text(&title);
                if tab.is_none() && !trash {
                    self.catalog_note_response(&mut c, &r, i, day, open_editor);
                } else if r.clicked() {
                    self.catalog_select(&c, i, day);
                }
            }
            {
                let mut c = table_cell(ui, cells[3], "body");
                let r = c.add(
                    egui::Label::new(RichText::new(shorten(&body, 100)).color(MUTED))
                        .truncate()
                        .sense(Sense::click_and_drag()),
                );
                let r = if locked {
                    r.on_hover_text("잠금 해제 후 내용을 확인할 수 있습니다")
                } else {
                    r.on_hover_text(&body)
                };
                if tab.is_none() && !trash {
                    self.catalog_note_response(&mut c, &r, i, day, open_editor);
                } else if r.clicked() {
                    self.catalog_select(&c, i, day);
                }
            }
            {
                let mut c = table_cell(ui, cells[4], "reminder");
                if !trash {
                    let has = reminder.is_some();
                    if action(&mut c, Action::Bell, "알람 설정", has).clicked() {
                        pending.push(TableCommand::Reminder(i));
                    }
                    if let Some(text) = reminder {
                        if c.add(
                            egui::Label::new(RichText::new(text).size(12.).color(if has {
                                ACCENT
                            } else {
                                MUTED
                            }))
                            .truncate()
                            .sense(Sense::click()),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                        {
                            pending.push(TableCommand::Reminder(i));
                        }
                    }
                } else {
                    c.add(
                        egui::Label::new(
                            RichText::new(reminder.unwrap_or("—".into()))
                                .size(12.)
                                .color(MUTED),
                        )
                        .truncate(),
                    );
                }
            }
            {
                let mut c = table_cell(ui, cells[5], "created");
                c.add(egui::Label::new(RichText::new(created).size(12.).color(MUTED)).truncate());
            }
            {
                let mut c = table_cell(ui, cells[6], "schedule");
                let r = c.add(
                    egui::Label::new(RichText::new(schedule).size(12.).color(MUTED))
                        .truncate()
                        .sense(if trash {
                            Sense::hover()
                        } else {
                            Sense::click()
                        }),
                );
                if !trash
                    && r.on_hover_text("일정 편집")
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                {
                    pending.push(TableCommand::Open(i, day));
                }
            }
            {
                let mut c = table_cell(ui, cells[7], "delete");
                if action(
                    &mut c,
                    Action::Trash,
                    if trash {
                        "영구 삭제"
                    } else {
                        "휴지통으로 이동"
                    },
                    false,
                )
                .clicked()
                {
                    pending.push(if let Some(t) = tab {
                        TableCommand::PurgeTab(i, t)
                    } else if trash {
                        TableCommand::Purge(i)
                    } else {
                        TableCommand::Trash(i)
                    });
                }
            }
            if selected
                && !ui.ctx().wants_keyboard_input()
                && ui.input(|i| {
                    i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::ArrowUp)
                })
            {
                let target = egui::Rect::from_min_max(
                    egui::pos2(ui.clip_rect().left() + 2., rect.top()),
                    egui::pos2(ui.clip_rect().left() + 3., rect.bottom()),
                );
                ui.scroll_to_rect(target, Some(egui::Align::Center));
            }
            let _ = response;
        });
    }

    fn catalog_note_response(
        &mut self,
        ui: &mut egui::Ui,
        r: &egui::Response,
        i: usize,
        day: Option<NaiveDate>,
        open_editor: bool,
    ) {
        if r.clicked() {
            self.catalog_select(ui, i, day);
            if open_editor {
                self.open_note_on(i, day);
            }
        }
        if r.drag_started() && !self.catalog.selected_many.contains(&i) {
            self.catalog.selected_many = BTreeSet::from([i]);
        }
        let notes = if self.catalog.selected_many.contains(&i) {
            self.catalog.selected_many.iter().copied().collect()
        } else {
            vec![i]
        };
        self.catalog
            .drag_sources
            .push((r.rect.intersect(ui.clip_rect()), FolderDrag::Notes(notes)));
    }

    fn catalog_select(&mut self, ui: &egui::Ui, i: usize, day: Option<NaiveDate>) {
        self.catalog.selected = Some(i);
        self.catalog.selected_day = day;
        if ui.input(|i| i.modifiers.command) {
            if !self.catalog.selected_many.remove(&i) {
                self.catalog.selected_many.insert(i);
            }
        } else {
            self.catalog.selected_many = BTreeSet::from([i]);
        }
    }

    fn catalog_preview(&mut self, ui: &mut egui::Ui, height: f32) {
        let Some(i) = self
            .catalog
            .selected
            .filter(|&i| i < self.data.notes.len() && !self.data.notes[i].deleted)
        else {
            egui::Frame::NONE
                .fill(ui.visuals().faint_bg_color)
                .corner_radius(8.)
                .inner_margin(18.)
                .show(ui, |ui| {
                    ui.set_min_height(height - 36.);
                    ui.label(
                        RichText::new("메모를 선택하면 여기에서 읽고 수정할 수 있습니다")
                            .color(MUTED),
                    );
                });
            return;
        };
        // inline_note already draws the rounded, colored paper. Wrapping it
        // in another Frame would add a second card, duplicate chrome and take
        // useful width away from the title and body.
        ui.set_height(height);
        if self.data.notes[i].locked {
            egui::Frame::NONE
                .fill(ui.visuals().faint_bg_color)
                .corner_radius(18.)
                .inner_margin(20.)
                .show(ui, |ui| {
                    ui.set_min_height((height - 40.).max(120.));
                    ui.heading(&self.data.notes[i].title);
                    ui.add_space(24.);
                    ui.label(RichText::new("잠긴 메모입니다").color(MUTED));
                    if ui.button("잠금 해제하기").clicked() {
                        self.open_note(i);
                    }
                });
        } else {
            self.inline_note(ui, i);
        }
    }

    fn catalog_folders(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("+ 새 폴더").clicked() {
                self.catalog.create_folder();
            }
            if ui
                .selectable_label(self.catalog.folder_selected.is_none(), "최상위 선택")
                .clicked()
            {
                self.catalog.folder_selected = None;
            }
            ui.label(
                RichText::new("폴더 선택 후 추가하면 하위 폴더가 됩니다")
                    .small()
                    .color(MUTED),
            );
        });
        ui.add_space(9.);
        let height = panel_height(ui);
        let rect =
            egui::Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), height));
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
            |ui| {
                ui.set_min_size(rect.size());
                ui.set_clip_rect(ui.clip_rect().intersect(rect));
                ui.columns(2, |cols| {
                    for column in cols.iter_mut() {
                        column.set_height(height);
                    }
                    let folder_height = (height * 0.43).max(120.);
                    egui::Frame::NONE
                        .fill(cols[0].visuals().faint_bg_color)
                        .corner_radius(7.)
                        .inner_margin(10.)
                        .show(&mut cols[0], |ui| {
                            ui.strong("폴더");
                            egui::ScrollArea::vertical()
                                .id_salt("catalog_folders")
                                .max_height((folder_height - 46.).max(70.))
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    let roots: Vec<_> = self
                                        .catalog
                                        .folders
                                        .iter()
                                        .enumerate()
                                        .filter(|(_, f)| !f.deleted && f.parent.is_none())
                                        .map(|(i, _)| i)
                                        .collect();
                                    if roots.is_empty() {
                                        ui.label(
                                            RichText::new(
                                                "새 폴더를 만들고 아래 메모를 끌어 놓으세요",
                                            )
                                            .small()
                                            .color(MUTED),
                                        );
                                    }
                                    for f in roots {
                                        self.catalog_folder_node(ui, f, 0);
                                    }
                                    let r = ui.add_sized(
                                        [ui.available_width(), 28.],
                                        egui::Label::new(
                                            RichText::new("최상위로 폴더 이동")
                                                .small()
                                                .color(MUTED),
                                        )
                                        .sense(Sense::hover()),
                                    );
                                    self.catalog_folder_drop(ui, &r, None);
                                });
                        });
                    cols[0].add_space(12.);
                    cols[0].horizontal(|ui| {
                        ui.strong("아직 분류하지 않은 메모");
                        if !self.catalog.selected_many.is_empty() {
                            ui.label(
                                RichText::new(format!(
                                    "{}개 선택",
                                    self.catalog.selected_many.len()
                                ))
                                .small()
                                .color(ACCENT),
                            );
                        }
                    });
                    cols[0].label(
                        RichText::new("Ctrl+클릭으로 여러 메모를 선택한 뒤 폴더로 드래그")
                            .small()
                            .color(MUTED),
                    );
                    let unfiled: Vec<_> = self
                        .data
                        .notes
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| {
                            self.visible(*i) && !self.catalog.assignments.contains_key(i)
                        })
                        .map(|(i, _)| i)
                        .collect();
                    egui::ScrollArea::vertical()
                        .id_salt("catalog_unfiled")
                        .max_height((height - folder_height - 90.).max(70.))
                        .auto_shrink([false, false])
                        .show(&mut cols[0], |ui| {
                            if unfiled.is_empty() {
                                ui.label(RichText::new("모든 메모를 분류했습니다").color(MUTED));
                            }
                            for i in unfiled {
                                self.catalog_folder_note(ui, i);
                            }
                            let r = ui.add_sized(
                                [ui.available_width(), 28.],
                                egui::Label::new(
                                    RichText::new("여기로 끌어 분류 해제").small().color(MUTED),
                                )
                                .sense(Sense::hover()),
                            );
                            self.catalog_folder_drop(ui, &r, None);
                        });
                    self.catalog_preview(&mut cols[1], height);
                });
            },
        );
    }

    fn catalog_folder_node(&mut self, ui: &mut egui::Ui, f: usize, depth: usize) {
        if depth > self.catalog.folders.len() {
            return;
        }
        let selected = self.catalog.folder_selected == Some(f);
        let mut remove = false;
        let inner = ui.horizontal(|ui| {
            let icon = if self.catalog.folders[f].open {
                crate::icons::Icon::Fold
            } else {
                crate::icons::Icon::ChevronRight
            };
            if crate::icons::icon_button(ui, icon, "폴더 펼치기 / 접기").clicked() {
                self.catalog.folders[f].open = !self.catalog.folders[f].open;
            }
            if self.catalog.rename == Some(f) {
                let r = ui.add(
                    egui::TextEdit::singleline(&mut self.catalog.folders[f].title)
                        .desired_width((ui.available_width() - 10.).max(80.)),
                );
                if self.catalog.rename_focus {
                    r.request_focus();
                    self.catalog.rename_focus = false;
                }
                if r.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.catalog.rename = None;
                }
            } else {
                let text = if selected {
                    RichText::new(&self.catalog.folders[f].title)
                        .strong()
                        .color(ACCENT)
                } else {
                    RichText::new(&self.catalog.folders[f].title)
                };
                let r = ui.add(egui::Label::new(text).sense(Sense::click_and_drag()));
                if r.clicked() {
                    self.catalog.folder_selected = Some(f);
                }
                if r.double_clicked() {
                    self.catalog.rename = Some(f);
                    self.catalog.rename_focus = true;
                }
                self.catalog
                    .drag_sources
                    .push((r.rect.intersect(ui.clip_rect()), FolderDrag::Folder(f)));
                r.context_menu(|ui| {
                    if ui.button("이름 바꾸기").clicked() {
                        self.catalog.rename = Some(f);
                        self.catalog.rename_focus = true;
                        ui.close_menu();
                    }
                    if ui.button("폴더 삭제 · 메모 유지").clicked() {
                        remove = true;
                        ui.close_menu();
                    }
                });
            }
        });
        self.catalog_folder_drop(ui, &inner.response, Some(f));
        if remove {
            self.catalog.remove_folder(f);
            return;
        }
        if self.catalog.folders[f].open {
            ui.indent(("catalog_folder", f), |ui| {
                let notes: Vec<_> = self
                    .catalog
                    .assignments
                    .iter()
                    .filter(|(i, folder)| {
                        **folder == f && **i < self.data.notes.len() && self.visible(**i)
                    })
                    .map(|(&i, _)| i)
                    .collect();
                for i in notes {
                    self.catalog_folder_note(ui, i);
                }
                let children: Vec<_> = self
                    .catalog
                    .folders
                    .iter()
                    .enumerate()
                    .filter(|(_, child)| !child.deleted && child.parent == Some(f))
                    .map(|(i, _)| i)
                    .collect();
                for c in children {
                    self.catalog_folder_node(ui, c, depth + 1);
                }
            });
        }
    }

    fn catalog_folder_note(&mut self, ui: &mut egui::Ui, i: usize) {
        let selected = self.catalog.selected_many.contains(&i) || self.catalog.selected == Some(i);
        let height = match self.desktop.row_spacing {
            0 => 25.,
            2 => 38.,
            _ => 30.,
        };
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
        let r = ui.interact(
            rect,
            ui.id().with(("folder_note", i)),
            Sense::click_and_drag(),
        );
        if selected || r.hovered() {
            ui.painter().rect_filled(
                rect,
                5.,
                if selected {
                    ui.visuals().selection.bg_fill
                } else {
                    ui.visuals().faint_bg_color
                },
            );
        }
        ui.painter().text(
            rect.left_center() + Vec2::new(8., 0.),
            egui::Align2::LEFT_CENTER,
            shorten(&self.data.notes[i].title, 42),
            egui::FontId::proportional(14.),
            if selected {
                ACCENT
            } else {
                ui.visuals().text_color()
            },
        );
        r.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &self.data.notes[i].title)
        });
        self.catalog_note_response(ui, &r, i, self.data.notes[i].date, false);
        r.context_menu(|ui| {
            if ui.button("독립 편집창 열기").clicked() {
                self.open_note(i);
                ui.close_menu();
            }
            if self.catalog.assignments.contains_key(&i) && ui.button("폴더 분류 해제").clicked()
            {
                self.catalog.assignments.remove(&i);
                ui.close_menu();
            }
            if ui.button("휴지통으로 이동").clicked() {
                self.trash_note(i);
                ui.close_menu();
            }
        });
    }

    fn catalog_folder_drop(&mut self, ui: &egui::Ui, r: &egui::Response, folder: Option<usize>) {
        let rect = r.rect.intersect(ui.clip_rect());
        self.catalog.drag_targets.push((rect, folder));
        if self.catalog.dragging.is_some()
            && ui.input(|i| i.pointer.hover_pos().is_some_and(|p| rect.contains(p)))
        {
            ui.painter().rect_stroke(
                rect.expand(2.),
                4.,
                Stroke::new(1.5_f32, ACCENT),
                egui::StrokeKind::Inside,
            );
        }
    }
    fn catalog_drag_input(&mut self, ctx: &egui::Context) {
        if self.settings
            || self.desktop.lock_target.is_some()
            || self.desktop.reminder_target.is_some()
            || self.desktop.purge_target.is_some()
            || self.catalog.replace_open
            || ctx.memory(|m| m.any_popup_open())
        {
            self.catalog.dragging = None;
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.catalog.dragging = None;
            return;
        }
        for event in ctx.input(|i| i.events.clone()) {
            match event {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    ..
                } => {
                    self.catalog.dragging = self
                        .catalog
                        .drag_sources
                        .iter()
                        .rev()
                        .find(|(r, _)| r.contains(pos))
                        .map(|(_, source)| (source.clone(), pos));
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    ..
                } => {
                    if let Some((source, origin)) = self.catalog.dragging.take() {
                        if origin.distance(pos) < 4. {
                            continue;
                        }
                        let Some(folder) = self
                            .catalog
                            .drag_targets
                            .iter()
                            .rev()
                            .find(|(r, _)| r.contains(pos))
                            .map(|(_, f)| *f)
                        else {
                            continue;
                        };
                        match source {
                            FolderDrag::Notes(notes) => {
                                let notes: Vec<_> = notes
                                    .into_iter()
                                    .filter(|&i| i < self.data.notes.len() && self.visible(i))
                                    .collect();
                                self.catalog.assign_notes(&notes, folder);
                                self.toast = if folder.is_some() {
                                    format!("{}개 메모를 폴더에 배치했습니다", notes.len())
                                } else {
                                    format!("{}개 메모의 폴더 분류를 해제했습니다", notes.len())
                                };
                            }
                            FolderDrag::Folder(f) => {
                                if !self.catalog.move_folder(f, folder) {
                                    self.toast =
                                        "자기 자신이나 하위 폴더로 이동할 수 없습니다".into();
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn trash_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("휴지통");
        ui.label(
            RichText::new(
                "삭제한 메모와 본문 탭을 복원하거나 영구 삭제합니다. 현재 실행 중에만 보관됩니다.",
            )
            .small()
            .color(MUTED),
        );
        ui.add_space(10.);
        let notes: Vec<_> = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.deleted && !n.purged)
            .map(|(i, n)| TableRow::Note(i, n.date))
            .collect();
        let tabs: Vec<_> = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.purged)
            .flat_map(|(i, n)| (0..n.document.trash.len()).map(move |t| TableRow::Tab(i, t)))
            .collect();
        ui.label(
            RichText::new(format!("메모 {}개 · 본문 탭 {}개", notes.len(), tabs.len()))
                .small()
                .color(MUTED),
        );
        let rows: Vec<_> = notes.into_iter().chain(tabs).collect();
        if rows.is_empty() {
            ui.add_space(24.);
            ui.label("휴지통이 비어 있습니다");
        } else {
            let height = panel_height(ui);
            let rect =
                egui::Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), height));
            ui.allocate_new_ui(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
                |ui| {
                    ui.set_min_size(rect.size());
                    ui.set_clip_rect(ui.clip_rect().intersect(rect));
                    self.catalog_table(ui, &rows, height, false, true, false);
                },
            );
        }
        self.catalog_tab_purge_dialog(ui.ctx());
    }

    fn catalog_restore_tab(&mut self, i: usize, tab: usize) -> bool {
        let Some(n) = self.data.notes.get_mut(i).filter(|n| !n.purged) else {
            return false;
        };
        if tab >= n.document.trash.len() {
            return false;
        }
        let mut restored = n.document.trash.remove(tab);
        restored.hidden = false;
        let title = restored.title.clone();
        n.document.tabs.push(restored);
        n.document.active = n.document.tabs.len() - 1;
        self.restore_note(i);
        self.catalog.selected = Some(i);
        self.toast = format!("{title} 탭을 원래 메모에 복원했습니다");
        true
    }

    fn catalog_tab_purge_dialog(&mut self, ctx: &egui::Context) {
        let Some((i, tab)) = self.catalog.purge_tab else {
            return;
        };
        let Some(n) = self.data.notes.get(i).filter(|n| !n.purged) else {
            self.catalog.purge_tab = None;
            return;
        };
        let Some(item) = n.document.trash.get(tab) else {
            self.catalog.purge_tab = None;
            return;
        };
        let title = format!("{} - {}", n.title, item.title);
        let mut confirmed = false;
        let mut cancel = false;
        // A modal prevents list changes from shifting the selected tab's index
        // while the user reviews this irreversible in-memory action.
        let response = egui::Modal::new(egui::Id::new("purge_deleted_tab")).show(ctx, |ui| {
            ui.set_max_width(380.);
            ui.heading("본문 탭 영구 삭제");
            ui.label(title);
            ui.add_space(8.);
            ui.label("이 탭의 본문과 표·이미지를 영구 삭제할까요? 삭제 후에는 복원할 수 없습니다.");
            ui.add_space(12.);
            ui.horizontal(|ui| {
                cancel = ui.button("취소").clicked();
                confirmed = ui
                    .button(RichText::new("영구 삭제").color(ACCENT))
                    .clicked();
            });
        });
        if confirmed {
            self.catalog_confirm_tab_purge();
        } else if cancel || response.should_close() {
            self.catalog.purge_tab = None;
        }
    }

    fn catalog_confirm_tab_purge(&mut self) {
        let Some((i, tab)) = self.catalog.purge_tab.take() else {
            return;
        };
        if let Some(n) = self.data.notes.get_mut(i).filter(|n| !n.purged) {
            if tab < n.document.trash.len() {
                n.document.trash.remove(tab);
                self.toast = "삭제한 본문 탭을 영구 삭제했습니다".into();
            }
        }
    }

    pub fn catalog_replace_window(&mut self, ctx: &egui::Context) {
        if !self.catalog.replace_open {
            return;
        }
        let mut open = true;
        let mut apply = false;
        let mut target = vec![];
        egui::Window::new("찾아 바꾸기")
            .id(egui::Id::new("catalog_replace"))
            .open(&mut open)
            .default_width(420.)
            .show(ctx, |ui| {
                ui.label("찾을 내용");
                ui.text_edit_singleline(&mut self.catalog.replace_find);
                ui.label("바꿀 내용");
                ui.text_edit_singleline(&mut self.catalog.replace_with);
                ui.add_space(6.);
                ui.horizontal_wrapped(|ui| {
                    for (i, name) in ["현재 메모", "선택 프로젝트", "전체 메모"]
                        .iter()
                        .enumerate()
                    {
                        ui.selectable_value(&mut self.catalog.replace_scope, i, *name);
                    }
                });
                target = self.catalog_replace_targets();
                ui.label(
                    RichText::new(format!(
                        "{}개 메모에 적용 · 잠긴 메모와 휴지통 제외",
                        target.len()
                    ))
                    .small()
                    .color(MUTED),
                );
                if ui
                    .add_enabled(
                        !target.is_empty(),
                        egui::Button::new("위 범위에서 모두 바꾸기"),
                    )
                    .clicked()
                {
                    apply = true;
                }
                if !self.catalog.replace_message.is_empty() {
                    ui.label(&self.catalog.replace_message);
                }
            });
        self.catalog.replace_open = open;
        if apply {
            self.catalog_apply_replacement(&target);
            self.catalog.replace_message = format!("{}개 메모를 변경했습니다", target.len());
        }
    }

    fn catalog_replace_targets(&self) -> Vec<usize> {
        let selected = self.catalog.selected.or(Some(self.active));
        let project = self
            .catalog
            .project
            .or(self.project_filter)
            .or_else(|| selected.and_then(|i| self.data.notes[i].project));
        self.data
            .notes
            .iter()
            .enumerate()
            .filter(|(i, n)| {
                self.visible(*i)
                    && !n.locked
                    && !self.catalog.replace_find.is_empty()
                    && match self.catalog.replace_scope {
                        0 => selected == Some(*i),
                        1 => crate::refinements::project_match(
                            Some(project.unwrap_or(NO_PROJECT)),
                            n.project,
                        ),
                        _ => true,
                    }
                    && (n.title.contains(&self.catalog.replace_find)
                        || n.body.contains(&self.catalog.replace_find)
                        || n.document
                            .tabs
                            .iter()
                            .skip(1)
                            .any(|t| t.body.contains(&self.catalog.replace_find)))
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn catalog_apply_replacement(&mut self, targets: &[usize]) {
        let find = &self.catalog.replace_find;
        let replacement = &self.catalog.replace_with;
        if find.is_empty() {
            return;
        }
        for &i in targets {
            let n = &mut self.data.notes[i];
            n.title = n.title.replace(find, replacement);
            // The legacy body remains authoritative for the first tab. Bring
            // it into the document before replacing, so the next editor draw
            // cannot overwrite another tab's new text with an old first body.
            if let Some(first) = n.document.tabs.first_mut() {
                sync_tab_body(first, &n.body);
            }
            for tab in &mut n.document.tabs {
                replace_tab_text(tab, find, replacement);
            }
            n.body = n
                .document
                .tabs
                .first()
                .map(|t| t.body.clone())
                .unwrap_or_else(|| n.body.replace(find, replacement));
        }
    }
}

fn remap_spans(spans: &mut Vec<crate::composer::Span>, start: usize, end: usize, added: usize) {
    let removed = end - start;
    let shift = |position: usize| position.saturating_sub(removed) + added;
    for span in spans.iter_mut() {
        span.range.start = if span.range.start >= end {
            shift(span.range.start)
        } else if span.range.start > start {
            start
        } else {
            span.range.start
        };
        span.range.end = if span.range.end >= end {
            shift(span.range.end)
        } else if span.range.end > start {
            start + added
        } else {
            span.range.end
        };
    }
    spans.retain(|s| s.range.start < s.range.end);
}

fn sync_tab_body(tab: &mut crate::composer::Tab, body: &str) {
    if tab.body == body {
        return;
    }
    let old: Vec<_> = tab.body.chars().collect();
    let new: Vec<_> = body.chars().collect();
    let head = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let tail = old[head..]
        .iter()
        .rev()
        .zip(new[head..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    remap_spans(
        &mut tab.spans,
        head,
        old.len() - tail,
        new.len() - head - tail,
    );
    tab.body = body.into();
}

fn replace_tab_text(tab: &mut crate::composer::Tab, find: &str, replacement: &str) {
    // Apply from the end so byte and character offsets in earlier matches
    // remain valid. Keep rich formatting anchored after length-changing edits.
    let matches: Vec<_> = tab
        .body
        .match_indices(find)
        .map(|(byte, _)| (byte, tab.body[..byte].chars().count()))
        .collect();
    let removed = find.chars().count();
    let added = replacement.chars().count();
    for (byte, character) in matches.into_iter().rev() {
        tab.body.replace_range(byte..byte + find.len(), replacement);
        remap_spans(&mut tab.spans, character, character + removed, added);
    }
}

/// Character ranges, rather than byte offsets, match TextEdit's cursor contract.
fn panel_height(ui: &egui::Ui) -> f32 {
    // This view can live inside an outer ScrollArea and a horizontal row. Its
    // inherited max_rect may be one row high (or infinite), so derive a finite
    // pane from the visible viewport, rather than inheriting that row height.
    let bottom = ui
        .clip_rect()
        .bottom()
        .min(ui.ctx().screen_rect().bottom() - 18.);
    (bottom - ui.cursor().top() - 8.).clamp(180., 720.)
}

fn find_ranges(text: &str, query: &str) -> Vec<Range<usize>> {
    if query.is_empty() {
        return vec![];
    }
    // Compare folded text while retaining offsets in the original character
    // sequence, so Korean and multi-byte characters select the right range.
    let chars: Vec<_> = text.chars().collect();
    let width = query.chars().count();
    let query = query.to_lowercase();
    let mut ranges = vec![];
    let mut from = 0;
    while from + width <= chars.len() {
        let candidate: String = chars[from..from + width].iter().collect();
        if candidate.to_lowercase() == query {
            ranges.push(from..from + width);
            from += width;
        } else {
            from += 1;
        }
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    fn render_table_layout(a: &mut App, ctx: &egui::Context, width: f32) -> egui::FullOutput {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(width, 760.));
        ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::SidePanel::left("table_test_sidebar")
                    .exact_width(220.)
                    .show(ctx, |_| {});
                egui::CentralPanel::default().show(ctx, |ui| {
                    a.catalog_split_list(ui, &[(0, a.data.notes[0].date)], false, false);
                });
            },
        )
    }

    fn text_rect(output: &egui::FullOutput, text: &str, before_x: f32) -> egui::Rect {
        output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(t) = &shape.shape {
                    let rect = t.galley.rect.translate(t.pos.to_vec2());
                    if t.galley.job.text == text
                        && rect.left() < before_x
                        && shape.clip_rect.intersects(rect)
                    {
                        return Some(rect);
                    }
                }
                None
            })
            .unwrap_or_else(|| panic!("visible table text missing: {text}"))
    }

    fn representative_table_app() -> App {
        let mut a = App::sample(Local::now().date_naive());
        let n = &mut a.data.notes[0];
        n.title = "COLUMN TITLE".into();
        n.body = "COLUMN BODY".into();
        n.created = NaiveDate::from_ymd_opt(2026, 1, 2)
            .unwrap()
            .and_hms_opt(1, 2, 3)
            .unwrap();
        n.reminder = Some(
            NaiveDate::from_ymd_opt(2026, 2, 3)
                .unwrap()
                .and_hms_opt(4, 5, 0)
                .unwrap(),
        );
        n.date = NaiveDate::from_ymd_opt(2026, 11, 10);
        n.time = chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        a
    }

    #[test]
    fn detailed_table_keeps_fields_on_one_horizontal_row_with_aligned_headers() {
        let mut a = representative_table_app();
        let ctx = egui::Context::default();
        render_table_layout(&mut a, &ctx, 1440.);
        let out = render_table_layout(&mut a, &ctx, 1440.);
        let values = [
            "COLUMN TITLE",
            "COLUMN BODY",
            "02/03 04:05",
            "260102",
            "11/10 09:00",
        ]
        .map(|s| text_rect(&out, s, 1110.));
        assert!(values.windows(2).all(|p| p[0].left() < p[1].left()));
        assert!(values
            .iter()
            .all(|r| (r.center().y - values[0].center().y).abs() < 4.));
        for (name, index) in [("제목", 0), ("내용", 1), ("생성일", 3), ("일정", 4)] {
            let header = text_rect(&out, name, 1110.);
            assert!((header.left() - values[index].left()).abs() < 1.);
            assert!(header.bottom() < values[index].top());
        }
        assert!(
            a.catalog.table_viewport_width > 800.,
            "300px preview must not take half the workspace"
        );
    }

    #[test]
    fn narrow_table_scroll_can_reach_delete_column_without_moving_the_preview() {
        let mut a = representative_table_app();
        let ctx = egui::Context::default();
        a.catalog.table_offset = f32::MAX;
        render_table_layout(&mut a, &ctx, 1100.);
        let out = render_table_layout(&mut a, &ctx, 1100.);
        let visible = a.catalog.table_viewport_width;
        assert!(visible > 350. && visible < 650.);
        assert!(a.catalog.table_offset > 200.);
        let last = table_cells(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            Vec2::new(TABLE_WIDTH, 48.),
        ))[7];
        assert!(last.center().x - a.catalog.table_offset > 0.);
        assert!(last.right() - a.catalog.table_offset <= visible + 1.);
        let header = text_rect(&out, "생성일", 780.);
        let created = text_rect(&out, "260102", 780.);
        assert!((header.left() - created.left()).abs() < 1.);
    }
    #[test]
    fn folder_drop_uses_real_rendered_note_and_folder_bounds() {
        let mut a = App::sample(Local::now().date_naive());
        let folder = a.catalog.create_folder();
        a.catalog.rename = None;
        let ctx = egui::Context::default();
        let draw = |a: &mut App, events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1180., 760.),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| a.catalog_ui(ui));
                },
            );
        };
        draw(&mut a, vec![]);
        draw(&mut a, vec![]);
        let from = a
            .catalog
            .drag_sources
            .iter()
            .find_map(|(r, p)| {
                if matches!(p,FolderDrag::Notes(n) if n==&vec![0]) {
                    Some(r.center())
                } else {
                    None
                }
            })
            .unwrap();
        let to = a
            .catalog
            .drag_targets
            .iter()
            .find_map(|(r, f)| {
                if *f == Some(folder) {
                    Some(r.center())
                } else {
                    None
                }
            })
            .unwrap();
        draw(
            &mut a,
            vec![
                egui::Event::PointerMoved(from),
                egui::Event::PointerButton {
                    pos: from,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerMoved(to),
                egui::Event::PointerButton {
                    pos: to,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ],
        );
        assert_eq!(
            a.catalog.assignments.get(&0),
            Some(&folder),
            "{from:?} -> {to:?}, {}",
            a.toast
        );
    }
    #[test]
    fn rapid_note_drop_uses_press_source_and_keeps_folder_identity() {
        let mut a = App::sample(Local::now().date_naive());
        let folder = a.catalog.create_folder();
        a.catalog.drag_sources = vec![(
            egui::Rect::from_min_size(egui::pos2(0., 100.), Vec2::new(200., 30.)),
            FolderDrag::Notes(vec![0, 1]),
        )];
        a.catalog.drag_targets = vec![(
            egui::Rect::from_min_size(egui::pos2(0., 0.), Vec2::new(200., 30.)),
            Some(folder),
        )];
        let ctx = egui::Context::default();
        let press = egui::pos2(40., 110.);
        let release = egui::pos2(40., 10.);
        let events = vec![
            egui::Event::PointerButton {
                pos: press,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
            egui::Event::PointerMoved(release),
            egui::Event::PointerButton {
                pos: release,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            },
        ];
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| a.catalog_drag_input(ctx),
        );
        assert_eq!(a.catalog.assignments.get(&0), Some(&folder));
        assert_eq!(a.catalog.assignments.get(&1), Some(&folder));
        assert_eq!(a.catalog.folders[folder].parent, None);
    }
    #[test]
    fn creating_siblings_keeps_parent_context_and_roots_remain_possible() {
        let mut c = Catalog::default();
        let a = c.create_folder();
        c.folder_selected = Some(a);
        let b = c.create_folder();
        let d = c.create_folder();
        assert_eq!(c.folders[b].parent, Some(a));
        assert_eq!(c.folders[d].parent, Some(a));
        c.folder_selected = None;
        let root = c.create_folder();
        assert_eq!(c.folders[root].parent, None);
    }
    #[test]
    fn move_rejects_descendants_and_deleting_folder_preserves_other_assignments() {
        let mut c = Catalog::default();
        let root = c.create_folder();
        c.folder_selected = Some(root);
        let child = c.create_folder();
        assert!(!c.move_folder(root, Some(child)));
        c.assign_notes(&[0, 1], Some(child));
        c.assign_notes(&[2], Some(root));
        c.remove_folder(child);
        assert!(!c.assignments.contains_key(&0));
        assert!(!c.assignments.contains_key(&1));
        assert_eq!(c.assignments[&2], root);
    }
    #[test]
    fn batch_assignment_moves_originals_and_search_uses_character_boundaries() {
        let mut c = Catalog::default();
        let a = c.create_folder();
        let b = c.create_folder();
        c.assign_notes(&[4, 7], Some(a));
        c.assign_notes(&[4, 7], Some(b));
        assert_eq!(c.assignments.len(), 2);
        assert_eq!(c.assignments[&4], b);
        assert_eq!(find_ranges("가나다🙂 가나다", "가나다"), vec![0..3, 5..8]);
        assert_eq!(find_ranges("한글 Memo와 MEMO", "memo"), vec![3..7, 9..13]);
    }
    #[test]
    fn find_tracks_all_tabs_and_unicode_locations_without_locked_body_leaks() {
        let mut a = App::sample(Local::now().date_naive());
        let n = &mut a.data.notes[0];
        n.title = "제목".into();
        n.body = "한글🙂 찾기 찾기".into();
        n.document.tabs[0].body = "오래된 첫 탭".into();
        n.document.add_tab();
        n.document.tabs[1].body = "숨긴 찾기".into();
        n.document.tabs[1].hidden = true;
        a.catalog.query = "찾기".into();
        assert_eq!(
            a.catalog_find_hits(&[0, 0]),
            vec![
                (0, Some((0, 4..6))),
                (0, Some((0, 7..9))),
                (0, Some((1, 3..5))),
            ]
        );
        a.data.notes[0].locked = true;
        assert!(a.catalog_find_hits(&[0]).is_empty());
        a.data.notes[0].title = "찾기 제목".into();
        assert_eq!(a.catalog_find_hits(&[0]), vec![(0, None)]);
    }
    #[test]
    fn replacement_scopes_cover_other_tabs_and_keep_excluded_notes_unchanged() {
        let mut a = App::sample(Local::now().date_naive());
        for n in &mut a.data.notes {
            n.body = "첫 본문".into();
            n.document.add_tab();
            n.document.tabs[1].body = "대상 한글🙂 대상".into();
        }
        a.data.notes[0].body = "대상 첫 본문".into();
        a.data.notes[2].project = Some(1);
        a.data.notes[3].deleted = true;
        a.data.notes[4].locked = true;
        a.extra.hidden_projects.insert(1);
        a.catalog.selected = Some(0);
        a.catalog.replace_find = "대상".into();
        a.catalog.replace_with = "교체된 내용".into();
        assert_eq!(a.catalog_replace_targets(), vec![0]);
        a.catalog.replace_scope = 1;
        assert_eq!(a.catalog_replace_targets(), vec![0, 1]);
        a.catalog.replace_scope = 2;
        let targets = a.catalog_replace_targets();
        assert_eq!(targets, vec![0, 1, 5]);
        a.catalog_apply_replacement(&targets);
        for i in [0, 1, 5] {
            let n = &a.data.notes[i];
            assert_eq!(n.document.tabs[1].body, "교체된 내용 한글🙂 교체된 내용");
            assert_eq!(n.body, n.document.tabs[0].body);
        }
        assert_eq!(a.data.notes[0].body, "교체된 내용 첫 본문");
        for i in [2, 3, 4] {
            assert_eq!(a.data.notes[i].document.tabs[1].body, "대상 한글🙂 대상");
        }
        a.catalog.replace_find.clear();
        assert!(a.catalog_replace_targets().is_empty());
    }
    #[test]
    fn replacement_keeps_formatting_on_multibyte_text_after_length_changes() {
        let mut a = App::sample(Local::now().date_naive());
        let n = &mut a.data.notes[0];
        n.body = "가🙂 끝".into();
        n.document.tabs[0].body = n.body.clone();
        n.document.tabs[0].spans.push(crate::composer::Span {
            range: 3..4,
            format: crate::composer::Format {
                bold: true,
                ..Default::default()
            },
        });
        n.document.add_tab();
        n.document.tabs[1].body = "가🙂 가🙂 끝".into();
        n.document.tabs[1].spans.push(crate::composer::Span {
            range: 6..7,
            format: crate::composer::Format {
                underline: true,
                ..Default::default()
            },
        });
        a.catalog.replace_find = "가🙂".into();
        a.catalog.replace_with = "늘어난글".into();
        a.catalog_apply_replacement(&[0]);
        let n = &a.data.notes[0];
        assert_eq!(n.body, "늘어난글 끝");
        assert_eq!(n.document.tabs[0].spans[0].range, 5..6);
        assert_eq!(n.document.tabs[1].body, "늘어난글 늘어난글 끝");
        assert_eq!(n.document.tabs[1].spans[0].range, 10..11);
        assert!(n.document.tabs[1].spans[0].format.underline);
    }
    #[test]
    fn deleted_tab_restore_keeps_content_and_recovers_its_deleted_owner() {
        let mut a = App::sample(Local::now().date_naive());
        let n = &mut a.data.notes[0];
        n.document.tabs[0].title = "참고 자료".into();
        n.document.tabs[0].body = "이 내용을 복원합니다".into();
        n.document.tabs[0].hidden = true;
        n.document.tabs[0]
            .blocks
            .push(crate::composer::Block::Table(crate::composer::Table::new(
                2, 3,
            )));
        n.document.delete_tab(0);
        n.body.clear();
        n.deleted = true;
        assert!(a.catalog_restore_tab(0, 0));
        let n = &a.data.notes[0];
        let tab = &n.document.tabs[n.document.active];
        assert!(!n.deleted);
        assert_eq!(tab.title, "참고 자료");
        assert_eq!(tab.body, "이 내용을 복원합니다");
        assert!(!tab.hidden);
        assert_eq!(tab.blocks.len(), 1);
        assert!(n.document.trash.is_empty());
    }
    #[test]
    fn deleted_tab_purge_requires_pending_target_and_never_purges_other_tabs() {
        let mut a = App::sample(Local::now().date_naive());
        a.data.notes[0].document.add_tab();
        a.data.notes[0].document.tabs[1].body = "첫 번째 삭제 탭".into();
        a.data.notes[0].document.delete_tab(1);
        a.data.notes[0].document.add_tab();
        a.data.notes[0].document.tabs[1].body = "남겨둘 삭제 탭".into();
        a.data.notes[0].document.delete_tab(1);
        a.catalog_confirm_tab_purge();
        assert_eq!(a.data.notes[0].document.trash.len(), 2);
        a.catalog.purge_tab = Some((0, 0));
        assert_eq!(a.data.notes[0].document.trash.len(), 2);
        a.catalog_confirm_tab_purge();
        assert!(a.catalog.purge_tab.is_none());
        assert_eq!(a.data.notes[0].document.trash.len(), 1);
        assert_eq!(a.data.notes[0].document.trash[0].body, "남겨둘 삭제 탭");
        assert_eq!(a.data.notes[0].document.tabs.len(), 1);
    }
    #[test]
    fn detailed_list_inside_outer_scroll_paints_multiple_complete_rows() {
        let mut a = App::sample(Local::now().date_naive());
        a.split = false;
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1180., 760.));
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::SidePanel::left("test_sidebar")
                    .exact_width(220.)
                    .show(ctx, |_| {});
                egui::TopBottomPanel::top("test_toolbar")
                    .exact_height(54.)
                    .show(ctx, |_| {});
                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(20.);
                            ui.vertical(|ui| {
                                ui.set_min_width((ui.available_width() - 25.).max(200.));
                                a.detail_workspace(ui, 0);
                            });
                        });
                    });
                });
            },
        );
        let second_row = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    if text.galley.job.text.contains("화면 흐름 검토") {
                        let bounds = text.galley.rect.translate(text.pos.to_vec2());
                        return Some(shape.clip_rect.contains(bounds.center()));
                    }
                }
                None
            })
            .collect::<Vec<_>>();
        assert!(
            !second_row.is_empty(),
            "second row must be painted, rather than clipped to one line"
        );
        assert!(second_row.into_iter().all(|visible| visible));
    }
}
