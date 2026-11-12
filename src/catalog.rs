//! Folder organization, detailed note lists and the in-memory recycle bin.
use crate::*;
use egui::{Sense, Stroke};
use std::collections::HashMap;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

type FindHit = (usize, Option<(usize, Range<usize>)>);

/// One memo in the "모두 찾기" results: its title and the first body line
/// containing the text, with the found text's byte range in each.
#[derive(Clone, Debug)]
struct FindResult {
    note: usize,
    title_hit: Option<Range<usize>>,
    line: String,
    line_hit: Option<Range<usize>>,
    count: usize,
}

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
    /// Projects searched when the scope is "선택 프로젝트" (`NO_PROJECT`
    /// stands for memos without one).
    pub replace_projects: BTreeSet<usize>,
    /// Index of the current "찾기" match.
    replace_cursor: Option<usize>,
    /// "모두 찾기" results shown in place of the preview.
    pub results_open: bool,
    results: Vec<FindResult>,
    results_selected: BTreeSet<usize>,
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
    table_width: f32,
    pub preview_width: f32,
    /// User-adjusted widths for the 완료/알람/생성일/일정 columns (table
    /// indices 1, 4, 5, 6). `0.` means "not customized, use the default".
    pub column_widths: [f32; 4],
    /// Whether the 완료/알람/생성일/일정 columns are hidden. Defaulting to
    /// `false` (all visible) falls out of `#[derive(Default)]` for free.
    pub column_hidden: [bool; 4],
}

/// Fixed columns stay compact; title (2) and body (3) share the rest.
const COLUMN_WIDTHS: [f32; 8] = [30., 30., 0., 0., 96., 58., 86., 30.];
/// The narrowest table: below this the list scrolls horizontally.
const TABLE_WIDTH: f32 = 560.;
/// Table indices of the four columns a user can resize or hide, and their
/// labels in the same order as `Catalog::column_widths`/`column_hidden`.
const ADJUSTABLE_COLUMNS: [(usize, &str); 4] =
    [(1, "완료"), (4, "알람"), (5, "생성일"), (6, "일정")];
const ADJUSTABLE_MIN_WIDTH: f32 = 24.;
const ADJUSTABLE_MAX_WIDTH: f32 = 220.;

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

/// The effective width of one of the four user-adjustable columns: hidden
/// collapses it to zero, a custom width (stored as non-zero) overrides the
/// default, otherwise it keeps its original default width.
fn adjustable_width(cat: &Catalog, slot: usize) -> f32 {
    if cat.column_hidden[slot] {
        0.
    } else if cat.column_widths[slot] > 0. {
        cat.column_widths[slot]
    } else {
        COLUMN_WIDTHS[ADJUSTABLE_COLUMNS[slot].0]
    }
}

/// Column rectangles. The title and body columns take all width left by the
/// compact fixed columns, so they grow and shrink with the list pane. The
/// 완료/알람/생성일/일정 columns additionally honor the user's resize/hide
/// choices in `cat.column_widths`/`cat.column_hidden`.
fn table_cells(rect: egui::Rect, cat: &Catalog) -> [egui::Rect; 8] {
    let mut widths = COLUMN_WIDTHS;
    for (slot, &(idx, _)) in ADJUSTABLE_COLUMNS.iter().enumerate() {
        widths[idx] = adjustable_width(cat, slot);
    }
    let fixed: f32 = widths.iter().sum::<f32>() - widths[2] - widths[3];
    let flex = (rect.width().max(TABLE_WIDTH) - fixed).max(0.);
    widths[2] = (flex * 0.38).max(110.);
    widths[3] = (flex - widths[2]).max(0.);
    let mut x = rect.left();
    std::array::from_fn(|i| {
        let r = egui::Rect::from_min_size(
            egui::pos2(x, rect.top()),
            Vec2::new(widths[i], rect.height()),
        );
        x += widths[i];
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
    let icon = match kind {
        Action::Eye => crate::icons::Icon::Eye,
        Action::Bell => crate::icons::Icon::Bell,
        Action::Trash => crate::icons::Icon::Trash,
        Action::Restore => crate::icons::Icon::Restore2,
    };
    crate::icons::paint_icon(ui.painter(), rect.shrink(4.), icon, c);
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

impl App {
    pub fn catalog_ui(&mut self, ui: &mut egui::Ui) {
        self.catalog.drag_sources.clear();
        self.catalog.drag_targets.clear();
        ui::tabs(
            ui,
            &mut self.catalog.tab,
            &["폴더 보기", "전체 메모 보기", "관계 보드"],
        );
        ui.add_space(12.);
        if self.catalog.tab == 2 {
            self.board(ui);
            return;
        }
        if self.catalog.tab == 0 {
            self.catalog_folders(ui);
        } else {
            self.catalog_controls(ui);
            let mut cache = std::mem::take(&mut self.lists);
            let key = crate::lists::key(self.data.notes.len());
            let ids = cache.get("catalog", key, || self.catalog_filtered_ids());
            self.lists = cache;
            let ids = (*ids).clone();
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
        self.filters(ui);
        ui.add_space(10.);
        let today = Local::now().date_naive();
        let mut cache = std::mem::take(&mut self.lists);
        let key = crate::lists::key((mode, today, self.data.notes.len()));
        let rows = cache.get("detail", key, || self.detail_rows(mode, today));
        self.lists = cache;
        let ids: Vec<_> = rows.iter().map(|&(i, _)| i).collect();
        self.catalog_find_bar(ui, &ids);
        // Title and body clicks only show the memo in the preview; the eye
        // column opens it as a sticker.
        self.catalog_split_list(ui, &rows, false, true);
        self.catalog_replace_window(ui.ctx());
    }

    /// Rows of the detail list (each day of a multi-day memo is a row).
    fn detail_rows(&self, mode: usize, today: NaiveDate) -> Vec<(usize, Option<NaiveDate>)> {
        let mut rows = vec![];
        for (i, n) in self.data.notes.iter().enumerate() {
            if !self.matches_search(i)
                || self.is_guide(i)
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
                if (self.desktop.hide_completed && !self.extra.show_done && n.done_on(d))
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
        rows
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
            egui::ComboBox::from_id_salt("catalog_column_picker")
                .selected_text("열 표시")
                .show_ui(ui, |ui| {
                    for (slot, &(_, label)) in ADJUSTABLE_COLUMNS.iter().enumerate() {
                        let mut visible = !self.catalog.column_hidden[slot];
                        if ui.checkbox(&mut visible, label).changed() {
                            self.catalog.column_hidden[slot] = !visible;
                        }
                    }
                });
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
        let mut status = None;
        let status_slot = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.;
            let r = ui::search_field(
                ui,
                &mut self.catalog.query,
                "catalog_find",
                "목록에서 찾기  Ctrl+F",
                240.,
            );
            if self.catalog.find_focus {
                r.request_focus();
                self.catalog.find_focus = false;
            }
            reset = r.changed();
            if crate::icons::small_icon(ui, crate::icons::Icon::ChevronLeft, "이전 찾기").clicked()
            {
                direction = -1;
            }
            if crate::icons::small_icon(ui, crate::icons::Icon::ChevronRight, "다음 찾기  Enter")
                .clicked()
                || (r.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                direction = 1;
            }
            let slot = ui.cursor().min;
            ui.add_space(64.);
            if ui.button("찾아 바꾸기").clicked() {
                self.catalog.replace_open = true;
                if !self.catalog.query.is_empty() {
                    self.catalog.replace_find = self.catalog.query.clone();
                }
                ui::raise(ui.ctx(), "catalog_replace");
            }
            slot
        });
        let slot = status_slot.inner;
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
            status = Some(format!("{} / {}", self.catalog.find_index + 1, len));
        } else if !self.catalog.query.is_empty() {
            status = Some("결과 없음".into());
        }
        if let Some(text) = status {
            ui.painter().text(
                slot + Vec2::new(6., 13.),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(12.),
                ui::t(ui).muted,
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
        if self.catalog.preview_width <= 0. {
            self.catalog.preview_width = 300.;
        }
        // The list keeps at least its narrowest table; widening the preview
        // shrinks the list's title and body columns.
        let preview_max = (rect.width() - 30. - TABLE_WIDTH).max(240.);
        let preview_width = self.catalog.preview_width.clamp(240., preview_max);
        let list_width = (rect.width() - preview_width - 16.).max(180.);
        self.catalog.table_width = TABLE_WIDTH.max(list_width - 14.);
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
                let handle = egui::Rect::from_min_max(
                    egui::pos2(list_rect.right() + 2., rect.top()),
                    egui::pos2(list_rect.right() + 14., rect.bottom()),
                );
                let drag = ui
                    .interact(handle, ui.id().with("preview_splitter"), Sense::drag())
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
                    .on_hover_text("드래그해서 미리보기 너비 조절");
                if drag.hovered() || drag.dragged() {
                    ui.painter().line_segment(
                        [handle.center_top(), handle.center_bottom()],
                        Stroke::new(2.0_f32, ACCENT.gamma_multiply(0.6)),
                    );
                }
                if drag.dragged() {
                    self.catalog.preview_width =
                        (preview_width - drag.drag_delta().x).clamp(240., preview_max);
                }
                if drag.double_clicked() {
                    self.catalog.preview_width = 300.;
                }
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
        let offset = self.catalog.table_offset.clamp(
            0.,
            (self.catalog.table_width.max(TABLE_WIDTH) - header_width).max(0.),
        );
        egui::ScrollArea::horizontal()
            .id_salt(("table_header", trash))
            .horizontal_scroll_offset(offset)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .auto_shrink([false, false])
            .max_width(header_width)
            .max_height(36.)
            .show(ui, |ui| {
                let table_width = self.catalog.table_width.max(TABLE_WIDTH);
                ui.set_min_width(table_width);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(table_width, 34.), Sense::hover());
                ui.painter().line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    Stroke::new(1_f32, ui::t(ui).line),
                );
                let cells = table_cells(rect, &self.catalog);
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
                for (slot, &(idx, _)) in ADJUSTABLE_COLUMNS.iter().enumerate() {
                    if self.catalog.column_hidden[slot] {
                        continue;
                    }
                    let handle = egui::Rect::from_min_max(
                        egui::pos2(cells[idx].right() - 3., rect.top()),
                        egui::pos2(cells[idx].right() + 3., rect.bottom()),
                    );
                    let drag = ui
                        .interact(handle, ui.id().with(("column_resize", idx)), Sense::drag())
                        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
                        .on_hover_text("드래그해서 열 너비 조절 · 더블클릭으로 기본값");
                    if drag.hovered() || drag.dragged() {
                        ui.painter().line_segment(
                            [handle.center_top(), handle.center_bottom()],
                            Stroke::new(2.0_f32, ACCENT.gamma_multiply(0.6)),
                        );
                    }
                    if drag.dragged() {
                        let current = adjustable_width(&self.catalog, slot);
                        self.catalog.column_widths[slot] = (current + drag.drag_delta().x)
                            .clamp(ADJUSTABLE_MIN_WIDTH, ADJUSTABLE_MAX_WIDTH);
                    }
                    if drag.double_clicked() {
                        self.catalog.column_widths[slot] = 0.;
                    }
                }
            });
        let mut pending = vec![];
        let body = egui::ScrollArea::both()
            .id_salt(("detail_table_body", trash))
            .horizontal_scroll_offset(offset)
            .auto_shrink([false, false])
            .max_height((height - 44.).max(100.))
            .show(ui, |ui| {
                ui.set_min_width(self.catalog.table_width.max(TABLE_WIDTH));
                if rows.is_empty() {
                    ui.add_space(24.);
                    ui.label(RichText::new("표시할 메모가 없습니다").color(MUTED));
                }
                let mut previous = Some(NaiveDate::MIN);
                for (at, &row) in rows.iter().enumerate() {
                    if let TableRow::Note(_, day) = row {
                        if date_groups && previous != day {
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(self.catalog.table_width.max(TABLE_WIDTH), 28.),
                                Sense::hover(),
                            );
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
                    let key = match row {
                        TableRow::Note(i, d) => {
                            (0, i, d.map(|d| d.num_days_from_ce()).unwrap_or(0))
                        }
                        TableRow::Tab(i, t) => (1, i, t as i32),
                    };
                    let selected = self.catalog.selected == Some(key.1);
                    crate::lists::lazy_row(ui, key, 36., selected, |ui| {
                        self.catalog_table_row(ui, row, at, open_editor, trash, &mut pending)
                    });
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
            0 => 34.,
            2 => 52.,
            _ => 42.,
        };
        ui.push_id(("table_row", i, day, tab, trash), |ui| {
            let (rect, response) = ui.allocate_exact_size(
                Vec2::new(self.catalog.table_width.max(TABLE_WIDTH), height),
                Sense::hover(),
            );
            let fill = if selected {
                ui.visuals().selection.bg_fill
            } else if ui.rect_contains_pointer(rect) {
                ui::t(ui).hover.gamma_multiply(0.6)
            } else {
                Color32::TRANSPARENT
            };
            let _ = at;
            ui.painter().rect_filled(rect, 6., fill);
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
            let cells = table_cells(rect, &self.catalog);
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
                if c.add_enabled(!trash, |ui: &mut egui::Ui| {
                    ui::round_check(ui, &mut done, None)
                })
                .on_hover_text("완료 상태")
                .changed()
                {
                    pending.push(TableCommand::Complete(i, day, done));
                }
            }
            {
                let mut c = table_cell(ui, cells[2], "title");
                let mut text = RichText::new(&title);
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

    pub(crate) fn catalog_preview(&mut self, ui: &mut egui::Ui, height: f32) {
        if self.catalog.results_open {
            self.catalog_results(ui, height);
            return;
        }
        let Some(i) = self
            .catalog
            .selected
            .filter(|&i| i < self.data.notes.len() && !self.data.notes[i].deleted)
        else {
            let tone = ui::t(ui);
            egui::Frame::NONE
                .fill(tone.sidebar)
                .corner_radius(12.)
                .inner_margin(18.)
                .show(ui, |ui| {
                    ui.set_min_height(height - 36.);
                    ui.set_min_width(ui.available_width());
                    ui.add_space(height * 0.28);
                    ui::empty_state(
                        ui,
                        crate::icons::Icon::Notes,
                        "선택한 메모가 없습니다",
                        "목록에서 메모를 고르면 여기에서 바로 읽고 고칠 수 있습니다",
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

    /// Visible memos by folder (`None`: not filed), reused until input.
    fn catalog_folder_index(&mut self) -> std::rc::Rc<HashMap<Option<usize>, Vec<usize>>> {
        let mut cache = std::mem::take(&mut self.lists);
        let key = crate::lists::key(self.data.notes.len());
        let index = cache.get("catalog_folder_index", key, || {
            let mut index: HashMap<Option<usize>, Vec<usize>> = HashMap::new();
            for i in (0..self.data.notes.len()).filter(|&i| self.visible(i)) {
                index
                    .entry(self.catalog.assignments.get(&i).copied())
                    .or_default()
                    .push(i);
            }
            index
        });
        self.lists = cache;
        index
    }
    fn catalog_folders(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new(RichText::new("+  새 폴더").strong()))
                .clicked()
            {
                self.catalog.create_folder();
            }
            if ui::pill(ui, self.catalog.folder_selected.is_none(), "최상위").clicked() {
                self.catalog.folder_selected = None;
            }
            ui::hint(
                ui,
                match self
                    .catalog
                    .folder_selected
                    .and_then(|f| self.catalog.folders.get(f))
                {
                    Some(f) => format!(
                        "새 폴더는 ‘{}’ 아래에 만들어집니다 · 이름은 더블클릭으로 수정",
                        f.title
                    ),
                    None => "폴더를 선택한 뒤 추가하면 하위 폴더가 됩니다".into(),
                },
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
                    let folder_height = (height * 0.42).max(130.);
                    let folder_count = self.catalog.folders.iter().filter(|f| !f.deleted).count();
                    ui::section_header(
                        &mut cols[0],
                        crate::icons::Icon::Folder,
                        "폴더",
                        Some(folder_count),
                    );
                    ui::card(&mut cols[0], |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("catalog_folders")
                            .max_height((folder_height - 54.).max(70.))
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
                                    ui::empty_state(
                                        ui,
                                        crate::icons::Icon::Folder,
                                        "아직 폴더가 없습니다",
                                        "‘+ 새 폴더’로 만든 뒤 메모를 끌어 놓으세요",
                                    );
                                }
                                for f in roots {
                                    self.catalog_folder_node(ui, f, 0);
                                }
                                let r = ui.add_sized(
                                    [ui.available_width(), 26.],
                                    egui::Label::new(
                                        RichText::new("여기로 끌면 최상위 폴더가 됩니다")
                                            .size(11.)
                                            .color(ui::t(ui).faint),
                                    )
                                    .sense(Sense::hover()),
                                );
                                self.catalog_folder_drop(ui, &r, None);
                            });
                    });
                    cols[0].add_space(14.);
                    let unfiled = self
                        .catalog_folder_index()
                        .get(&None)
                        .cloned()
                        .unwrap_or_default();
                    cols[0].horizontal(|ui| {
                        ui::section_header(
                            ui,
                            crate::icons::Icon::Notes,
                            "분류하지 않은 메모",
                            Some(unfiled.len()),
                        );
                        if !self.catalog.selected_many.is_empty() {
                            ui::chip(
                                ui,
                                &format!("{}개 선택", self.catalog.selected_many.len()),
                                ui::t(ui).accent_soft,
                                ACCENT,
                            );
                        }
                    });
                    ui::card(&mut cols[0], |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("catalog_unfiled")
                            .max_height((height - folder_height - 112.).max(70.))
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if unfiled.is_empty() {
                                    ui::empty_state(
                                        ui,
                                        crate::icons::Icon::Notes,
                                        "모든 메모를 분류했습니다",
                                        "",
                                    );
                                }
                                for i in unfiled {
                                    crate::lists::lazy_row(
                                        ui,
                                        ("folder_note", i),
                                        34.,
                                        false,
                                        |ui| self.catalog_folder_note(ui, i),
                                    );
                                }
                                let r = ui.add_sized(
                                    [ui.available_width(), 26.],
                                    egui::Label::new(
                                        RichText::new("여기로 끌면 폴더 분류가 해제됩니다")
                                            .size(11.)
                                            .color(ui::t(ui).faint),
                                    )
                                    .sense(Sense::hover()),
                                );
                                self.catalog_folder_drop(ui, &r, None);
                            });
                    });
                    cols[0].add_space(4.);
                    ui::hint(
                        &mut cols[0],
                        "Ctrl+클릭으로 여러 메모를 고른 뒤 폴더로 끌어 분류합니다",
                    );
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
        let inner = if self.catalog.rename == Some(f) {
            ui.horizontal(|ui| {
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
            })
            .response
        } else {
            let tone = ui::t(ui);
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.), Sense::hover());
            let r = ui.interact(
                rect,
                ui.id().with(("folder_row", f)),
                Sense::click_and_drag(),
            );
            if selected {
                ui.painter().rect_filled(rect, 7., tone.accent_soft);
            } else if r.hovered() {
                ui.painter().rect_filled(rect, 7., tone.hover);
            }
            let chevron = egui::Rect::from_center_size(
                egui::pos2(rect.left() + 13., rect.center().y),
                Vec2::splat(18.),
            );
            crate::icons::paint_icon(
                ui.painter(),
                chevron.shrink(4.),
                if self.catalog.folders[f].open {
                    crate::icons::Icon::ChevronDown
                } else {
                    crate::icons::Icon::ChevronRight
                },
                tone.muted,
            );
            crate::icons::paint_icon(
                ui.painter(),
                egui::Rect::from_center_size(
                    egui::pos2(rect.left() + 34., rect.center().y),
                    Vec2::splat(15.),
                ),
                crate::icons::Icon::Folder,
                if selected { ACCENT } else { tone.muted },
            );
            let count = self
                .catalog_folder_index()
                .get(&Some(f))
                .map_or(0, Vec::len);
            ui.painter().text(
                egui::pos2(rect.left() + 48., rect.center().y),
                egui::Align2::LEFT_CENTER,
                shorten(&self.catalog.folders[f].title, 36),
                egui::FontId::proportional(13.5),
                if selected { ACCENT } else { tone.ink },
            );
            if count > 0 {
                ui.painter().text(
                    egui::pos2(rect.right() - 10., rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    count.to_string(),
                    egui::FontId::proportional(11.5),
                    tone.faint,
                );
            }
            r.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Label,
                    true,
                    &self.catalog.folders[f].title,
                )
            });
            if r.clicked() {
                if r.interact_pointer_pos()
                    .is_some_and(|p| chevron.expand(4.).contains(p))
                {
                    self.catalog.folders[f].open = !self.catalog.folders[f].open;
                } else {
                    self.catalog.folder_selected = Some(f);
                }
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
            r
        };
        let inner = egui::InnerResponse::new((), inner);
        self.catalog_folder_drop(ui, &inner.response, Some(f));
        if remove {
            self.catalog.remove_folder(f);
            return;
        }
        if self.catalog.folders[f].open {
            ui.indent(("catalog_folder", f), |ui| {
                let notes = self
                    .catalog_folder_index()
                    .get(&Some(f))
                    .cloned()
                    .unwrap_or_default();
                for i in notes {
                    crate::lists::lazy_row(ui, ("folder_note", i), 34., false, |ui| {
                        self.catalog_folder_note(ui, i)
                    });
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
            0 => 28.,
            2 => 40.,
            _ => 34.,
        };
        let n = &self.data.notes[i];
        let meta = n
            .date
            .map(|d| d.format("%m.%d").to_string())
            .unwrap_or_default();
        let r = ui::list_row(
            ui,
            ui.id().with(("folder_note", i)),
            selected,
            Some(n.color),
            &n.title,
            &meta,
            height,
        );
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
        ui::hint(
            ui,
            "삭제한 메모와 본문 탭을 복원하거나 영구 삭제합니다 · 삭제한 메모는 설정의 보관 기간이 지나면 자동으로 지워집니다",
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
        ui::section_header(
            ui,
            crate::icons::Icon::Trash,
            "휴지통",
            Some(notes.len() + tabs.len()),
        );
        let rows: Vec<_> = notes.into_iter().chain(tabs).collect();
        if rows.is_empty() {
            ui.add_space(8.);
            ui::card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui::empty_state(
                    ui,
                    crate::icons::Icon::Trash,
                    "휴지통이 비어 있습니다",
                    "삭제한 메모와 본문 탭이 여기에 모입니다",
                );
            });
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
        let mut open = true;
        ui::dialog(
            ctx,
            "purge_deleted_tab",
            "본문 탭 영구 삭제",
            [400., 210.],
            true,
            &mut open,
            |ui| {
                ui.label(RichText::new(title).strong().size(15.));
                ui.add_space(4.);
                ui::hint(
                    ui,
                    "이 탭의 본문과 표·이미지를 영구 삭제합니다. 삭제 후에는 복원할 수 없습니다.",
                );
                (confirmed, cancel) = ui::dialog_buttons(ui, "영구 삭제", "취소");
            },
        );
        if confirmed {
            self.catalog_confirm_tab_purge();
        } else if cancel || !open {
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
        let mut command = None;
        let mut target = vec![];
        ui::dialog(
            ctx,
            "catalog_replace",
            "찾아 바꾸기",
            [460., 420.],
            false,
            &mut open,
            |ui| {
                ui.label(RichText::new("찾아 바꾸기").strong().size(15.));
                ui.add_space(8.);
                ui::field(ui, "찾을 내용", |ui| {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.catalog.replace_find)
                            .desired_width(f32::INFINITY)
                            .margin(egui::Margin::symmetric(8, 5)),
                    );
                    if r.changed() {
                        self.catalog.replace_cursor = None;
                    }
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        command = Some(ReplaceCommand::FindNext);
                    }
                });
                ui::field(ui, "바꿀 내용", |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.catalog.replace_with)
                            .desired_width(f32::INFINITY)
                            .margin(egui::Margin::symmetric(8, 5)),
                    );
                });
                ui.add_space(6.);
                ui::field(ui, "범위", |ui| {
                    if ui::segmented(
                        ui,
                        &mut self.catalog.replace_scope,
                        &["현재 메모", "선택 프로젝트", "전체 메모"],
                    ) {
                        self.catalog.replace_cursor = None;
                    }
                });
                if self.catalog.replace_scope == 1 && self.catalog.replace_projects.is_empty() {
                    let current = self
                        .catalog
                        .selected
                        .and_then(|i| self.data.notes[i].project)
                        .unwrap_or(NO_PROJECT);
                    self.catalog.replace_projects.insert(current);
                }
                ui::field(ui, "프로젝트", |ui| {
                    ui.add_enabled_ui(self.catalog.replace_scope == 1, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            let mut choices = vec![(NO_PROJECT, "프로젝트 없음".to_string())];
                            for (p, name) in self.data.projects.iter().enumerate() {
                                if !self.extra.deleted_projects.contains(&p) {
                                    choices.push((p, format!("# {name}")));
                                }
                            }
                            for (p, name) in choices {
                                let on = self.catalog.replace_projects.contains(&p);
                                if ui::pill(ui, on, &name).clicked() {
                                    if on {
                                        self.catalog.replace_projects.remove(&p);
                                    } else {
                                        self.catalog.replace_projects.insert(p);
                                    }
                                    self.catalog.replace_cursor = None;
                                }
                            }
                        });
                    });
                });
                target = self.catalog_replace_targets();
                ui.label(
                    RichText::new(format!(
                        "{}개 메모에서 찾음 · 잠긴 메모와 휴지통 제외",
                        target.len()
                    ))
                    .small()
                    .color(MUTED),
                );
                ui.add_space(4.);
                if !self.catalog.replace_message.is_empty() {
                    ui.label(RichText::new(&self.catalog.replace_message).color(ACCENT));
                }
                ui.add_space(4.);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let any = !target.is_empty();
                    if ui
                        .add_enabled(
                            any,
                            egui::Button::new(
                                RichText::new("모두 바꾸기").color(Color32::WHITE).strong(),
                            )
                            .fill(ACCENT),
                        )
                        .clicked()
                    {
                        command = Some(ReplaceCommand::ReplaceAll);
                    }
                    if ui.add_enabled(any, egui::Button::new("바꾸기")).clicked() {
                        command = Some(ReplaceCommand::Replace);
                    }
                    if ui
                        .add_enabled(any, egui::Button::new("모두 찾기"))
                        .clicked()
                    {
                        command = Some(ReplaceCommand::FindAll);
                    }
                    if ui.add_enabled(any, egui::Button::new("찾기")).clicked() {
                        command = Some(ReplaceCommand::FindNext);
                    }
                });
            },
        );
        self.catalog.replace_open = open;
        match command {
            Some(ReplaceCommand::FindNext) => self.replace_find_next(&target),
            Some(ReplaceCommand::Replace) => {
                self.replace_current(&target);
            }
            Some(ReplaceCommand::FindAll) => self.replace_find_all(&target),
            Some(ReplaceCommand::ReplaceAll) => {
                let count = self.replace_count(&target);
                self.catalog_apply_replacement(&target);
                self.catalog.replace_cursor = None;
                self.catalog.replace_message =
                    format!("{}개 메모에서 {count}곳을 바꿨습니다", target.len());
                if self.catalog.results_open {
                    let target = self.catalog_replace_targets();
                    self.replace_find_all(&target);
                }
            }
            None => {}
        }
    }

    /// Exact (case-sensitive) matches in `targets`, in reading order: title
    /// first, then each tab.
    fn replace_matches(&self, targets: &[usize]) -> Vec<FindHit> {
        let find = &self.catalog.replace_find;
        let mut hits = vec![];
        if find.is_empty() {
            return hits;
        }
        for &i in targets {
            let n = &self.data.notes[i];
            for _ in n.title.match_indices(find.as_str()) {
                hits.push((i, None));
            }
            for (tab, body) in std::iter::once((0, n.body.as_str())).chain(
                n.document
                    .tabs
                    .iter()
                    .enumerate()
                    .skip(1)
                    .map(|(tab, t)| (tab, t.body.as_str())),
            ) {
                for range in exact_ranges(body, find) {
                    hits.push((i, Some((tab, range))));
                }
            }
        }
        hits
    }
    fn replace_count(&self, targets: &[usize]) -> usize {
        self.replace_matches(targets).len()
    }
    /// Select the next match: its memo in the list and the text in the preview.
    fn replace_find_next(&mut self, targets: &[usize]) {
        let hits = self.replace_matches(targets);
        if hits.is_empty() {
            self.catalog.replace_message = "찾는 내용이 없습니다".into();
            self.catalog.replace_cursor = None;
            return;
        }
        let at = self
            .catalog
            .replace_cursor
            .map_or(0, |c| (c + 1) % hits.len());
        self.catalog.replace_cursor = Some(at);
        let (id, location) = hits[at].clone();
        self.catalog.results_open = false;
        self.catalog.selected = Some(id);
        self.catalog.selected_many = BTreeSet::from([id]);
        self.catalog.find_selection = location.map(|(tab, range)| (id, tab, range));
        self.catalog.replace_message = format!(
            "{} / {} · {}",
            at + 1,
            hits.len(),
            if self.data.notes[id].title.is_empty() {
                "제목 없음"
            } else {
                &self.data.notes[id].title
            }
        );
    }
    /// Replace the current match, then move on to the next one.
    fn replace_current(&mut self, targets: &[usize]) {
        let hits = self.replace_matches(targets);
        let Some(at) = self.catalog.replace_cursor.filter(|&c| c < hits.len()) else {
            self.replace_find_next(targets);
            return;
        };
        let (i, location) = hits[at].clone();
        let find = self.catalog.replace_find.clone();
        let with = self.catalog.replace_with.clone();
        let n = &mut self.data.notes[i];
        match location {
            None => n.title = n.title.replacen(&find, &with, 1),
            Some((tab, range)) => {
                if let Some(first) = n.document.tabs.first_mut() {
                    sync_tab_body(first, &n.body);
                }
                if let Some(t) = n.document.tabs.get_mut(tab) {
                    replace_tab_range(t, range, &find, &with);
                }
                if let Some(first) = n.document.tabs.first() {
                    n.body = first.body.clone();
                }
            }
        }
        // The replaced text is no longer a match: the next one now sits at
        // the same index.
        self.catalog.replace_cursor = at.checked_sub(1);
        if self.replace_matches(targets).is_empty() {
            self.catalog.replace_cursor = None;
            self.catalog.replace_message = "모두 바꿨습니다".into();
        } else {
            if at == 0 {
                self.catalog.replace_cursor = None;
            }
            self.replace_find_next(targets);
        }
    }
    /// Gather one result per memo and show them in the preview pane.
    fn replace_find_all(&mut self, targets: &[usize]) {
        let find = self.catalog.replace_find.clone();
        let mut results = vec![];
        for &i in targets {
            let n = &self.data.notes[i];
            let count = n.title.matches(find.as_str()).count()
                + std::iter::once(n.body.as_str())
                    .chain(n.document.tabs.iter().skip(1).map(|t| t.body.as_str()))
                    .map(|b| b.matches(find.as_str()).count())
                    .sum::<usize>();
            if count == 0 {
                continue;
            }
            let title_hit = n.title.find(find.as_str()).map(|b| b..b + find.len());
            let line = std::iter::once(n.body.as_str())
                .chain(n.document.tabs.iter().skip(1).map(|t| t.body.as_str()))
                .flat_map(|b| b.lines())
                .find(|l| l.contains(find.as_str()))
                .map(|l| crate::composer::strip_pictures(l).trim().to_string());
            let (line, line_hit) = match line {
                Some(l) => {
                    let hit = l.find(find.as_str()).map(|b| b..b + find.len());
                    (l, hit)
                }
                None => ("제목에서 찾았습니다".into(), None),
            };
            results.push(FindResult {
                note: i,
                title_hit,
                line,
                line_hit,
                count,
            });
        }
        self.catalog
            .results_selected
            .retain(|n| results.iter().any(|r| r.note == *n));
        self.catalog.replace_message = format!("{}개 메모에서 찾았습니다", results.len());
        self.catalog.results = results;
        self.catalog.results_open = true;
    }
    /// The "모두 찾기" results in the preview pane, with the actions on top
    /// so they stay reachable when the dialog falls behind the main window.
    fn catalog_results(&mut self, ui: &mut egui::Ui, height: f32) {
        let tone = ui::t(ui);
        let mut action = None;
        ui.set_height(height);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("찾기 결과").strong().size(14.));
            ui.label(
                RichText::new(format!(
                    "{}개 · {}개 선택",
                    self.catalog.results.len(),
                    self.catalog.results_selected.len()
                ))
                .size(12.)
                .color(tone.faint),
            );
        });
        ui.add_space(4.);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.catalog.results_selected.is_empty(),
                    egui::Button::new("바꾸기"),
                )
                .on_hover_text("선택한 결과의 메모에서 바꿉니다")
                .clicked()
            {
                action = Some(0);
            }
            if ui.button("모두 바꾸기").clicked() {
                action = Some(1);
            }
            if ui.button("다른 찾기").clicked() {
                action = Some(2);
            }
            if ui.button("찾아 바꾸기 마침").clicked() {
                action = Some(3);
            }
        });
        ui.add_space(6.);
        let find = self.catalog.replace_find.clone();
        egui::ScrollArea::vertical()
            .id_salt("find_results")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.catalog.results.is_empty() {
                    ui::empty_state(
                        ui,
                        crate::icons::Icon::Search,
                        "찾은 내용이 없습니다",
                        "‘다른 찾기’로 검색어를 바꿔 보세요",
                    );
                }
                for k in 0..self.catalog.results.len() {
                    let r = self.catalog.results[k].clone();
                    let selected = self.catalog.results_selected.contains(&r.note);
                    let (rect, resp) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 52.), Sense::click());
                    let color = self.data.notes[r.note].color;
                    ui.painter().rect(
                        rect,
                        8.,
                        if selected {
                            tone.accent_soft
                        } else if resp.hovered() {
                            tone.hover
                        } else {
                            tone.bg
                        },
                        Stroke::new(1.0_f32, if selected { ACCENT } else { tone.line }),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().circle_filled(
                        egui::pos2(rect.left() + 14., rect.top() + 17.),
                        4.5,
                        color,
                    );
                    let x = rect.left() + 26.;
                    let width = rect.right() - x - 10.;
                    let title = &self.data.notes[r.note].title;
                    let title = if title.is_empty() {
                        "제목 없음"
                    } else {
                        title
                    };
                    let job = highlight_job(
                        title,
                        r.title_hit.clone(),
                        egui::FontId::proportional(13.5),
                        tone.ink,
                        width,
                    );
                    let galley = ui.fonts(|f| f.layout_job(job));
                    ui.painter()
                        .galley(egui::pos2(x, rect.top() + 8.), galley, tone.ink);
                    let job = highlight_job(
                        &r.line,
                        r.line_hit.clone(),
                        egui::FontId::proportional(12.),
                        tone.muted,
                        width,
                    );
                    let galley = ui.fonts(|f| f.layout_job(job));
                    ui.painter()
                        .galley(egui::pos2(x, rect.top() + 29.), galley, tone.muted);
                    if r.count > 1 {
                        ui.painter().text(
                            egui::pos2(rect.right() - 10., rect.top() + 16.),
                            egui::Align2::RIGHT_CENTER,
                            format!("{}곳", r.count),
                            egui::FontId::proportional(11.),
                            tone.faint,
                        );
                    }
                    let resp = resp
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(format!("‘{find}’ · 클릭해서 선택 / 해제"));
                    if resp.clicked() {
                        if !self.catalog.results_selected.remove(&r.note) {
                            self.catalog.results_selected.insert(r.note);
                        }
                        self.catalog.selected = Some(r.note);
                    }
                    ui.add_space(4.);
                }
            });
        match action {
            Some(0) => {
                let target: Vec<_> = self.catalog.results_selected.iter().copied().collect();
                self.catalog_apply_replacement(&target);
                self.catalog.results_selected.clear();
                let target = self.catalog_replace_targets();
                self.replace_find_all(&target);
            }
            Some(1) => {
                let target: Vec<_> = self.catalog.results.iter().map(|r| r.note).collect();
                self.catalog_apply_replacement(&target);
                let target = self.catalog_replace_targets();
                self.replace_find_all(&target);
            }
            Some(2) => {
                self.catalog.replace_open = true;
                ui::raise(ui.ctx(), "catalog_replace");
            }
            Some(3) => {
                self.catalog.results_open = false;
                self.catalog.replace_open = false;
                self.catalog.results.clear();
                self.catalog.results_selected.clear();
            }
            _ => {}
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
                        1 if !self.catalog.replace_projects.is_empty() => self
                            .catalog
                            .replace_projects
                            .contains(&n.project.unwrap_or(NO_PROJECT)),
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

#[derive(Clone, Copy)]
enum ReplaceCommand {
    FindNext,
    Replace,
    FindAll,
    ReplaceAll,
}

/// Character ranges of exact (case-sensitive) occurrences of `find`.
fn exact_ranges(text: &str, find: &str) -> Vec<Range<usize>> {
    if find.is_empty() {
        return vec![];
    }
    let width = find.chars().count();
    text.match_indices(find)
        .map(|(byte, _)| {
            let start = text[..byte].chars().count();
            start..start + width
        })
        .collect()
}

/// Replace the occurrence of `find` at character range `range`, if it is
/// still there, keeping formatting spans anchored.
fn replace_tab_range(
    tab: &mut crate::composer::Tab,
    range: Range<usize>,
    find: &str,
    replacement: &str,
) -> bool {
    let Some((start, _)) = tab.body.char_indices().nth(range.start) else {
        return false;
    };
    if !tab.body[start..].starts_with(find) {
        return false;
    }
    tab.body
        .replace_range(start..start + find.len(), replacement);
    remap_spans(
        &mut tab.spans,
        range.start,
        range.end,
        replacement.chars().count(),
    );
    true
}

/// One line of text truncated to `width`, with `hit` (a byte range)
/// highlighted.
fn highlight_job(
    text: &str,
    hit: Option<Range<usize>>,
    font: egui::FontId,
    color: Color32,
    width: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let plain = egui::TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };
    match hit.filter(|h| {
        h.end <= text.len() && text.is_char_boundary(h.start) && text.is_char_boundary(h.end)
    }) {
        Some(h) => {
            // Keep the hit visible on long lines: start a little before it.
            let lead = text[..h.start]
                .char_indices()
                .rev()
                .nth(16)
                .map_or(0, |(b, _)| b);
            if lead > 0 {
                job.append("…", 0., plain.clone());
            }
            job.append(&text[lead..h.start], 0., plain.clone());
            job.append(
                &text[h.clone()],
                0.,
                egui::TextFormat {
                    font_id: font,
                    color: INK,
                    background: Color32::from_rgb(255, 222, 120),
                    ..Default::default()
                },
            );
            job.append(&text[h.end..], 0., plain);
        }
        None => job.append(text, 0., plain),
    }
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(40.));
    job
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

    #[test]
    fn find_next_and_replace_walk_matches_across_projects() {
        let mut a = App::sample(Local::now().date_naive());
        a.data.notes[0].body = "사과 하나, 사과 둘".into();
        a.data.notes[0].project = Some(0);
        a.data.notes[1].body = "사과 셋".into();
        a.data.notes[1].project = Some(1);
        a.data.notes[2].body = "사과 넷".into();
        a.data.notes[2].project = None;
        a.catalog.replace_find = "사과".into();
        a.catalog.replace_with = "배".into();
        a.catalog.replace_scope = 1;
        a.catalog.replace_projects = BTreeSet::from([0, 1]);
        let targets = a.catalog_replace_targets();
        assert!(targets.contains(&0) && targets.contains(&1) && !targets.contains(&2));
        let total = a.replace_count(&targets);
        a.replace_find_next(&targets);
        assert_eq!(
            a.catalog.find_selection.as_ref().map(|s| s.2.clone()),
            Some(0..2)
        );
        // Replace the first, then the cursor stays on the next match.
        a.replace_current(&targets);
        assert_eq!(a.data.notes[0].body, "배 하나, 사과 둘");
        assert_eq!(a.replace_count(&targets), total - 1);
        assert_eq!(
            a.catalog.find_selection.as_ref().map(|s| s.2.clone()),
            Some(6..8)
        );
        // "모두 찾기" lists one result per memo with the hit highlighted.
        a.replace_find_all(&targets);
        assert!(a.catalog.results_open);
        let r = a.catalog.results.iter().find(|r| r.note == 1).unwrap();
        assert_eq!(&r.line[r.line_hit.clone().unwrap()], "사과");
        assert!(
            a.data.notes[2].body.contains("사과"),
            "other projects are untouched"
        );
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
    fn the_table_shrinks_with_the_list_and_scrolls_only_when_too_narrow() {
        let mut a = representative_table_app();
        let ctx = egui::Context::default();
        // A typical window: every column fits without horizontal scrolling.
        render_table_layout(&mut a, &ctx, 1400.);
        let out = render_table_layout(&mut a, &ctx, 1400.);
        let visible = a.catalog.table_viewport_width;
        assert!(
            (a.catalog.table_width - visible).abs() < 20.,
            "{} vs {visible}",
            a.catalog.table_width
        );
        let header = text_rect(&out, "생성일", 1400.);
        let created = text_rect(&out, "260102", 1400.);
        assert!((header.left() - created.left()).abs() < 1.);
        // Widening the preview narrows the list instead of hiding columns.
        let before = a.catalog.table_width;
        a.catalog.preview_width += 120.;
        render_table_layout(&mut a, &ctx, 1400.);
        assert!(a.catalog.table_width < before);
        // Below the narrowest table the delete column is still reachable.
        a.catalog.table_offset = f32::MAX;
        render_table_layout(&mut a, &ctx, 700.);
        render_table_layout(&mut a, &ctx, 700.);
        let visible = a.catalog.table_viewport_width;
        let last = table_cells(
            egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(TABLE_WIDTH, 48.)),
            &a.catalog,
        )[7];
        assert!(last.center().x - a.catalog.table_offset > 0.);
        assert!(last.right() - a.catalog.table_offset <= visible + 1.);
    }

    #[test]
    fn hidden_columns_collapse_to_zero_and_custom_widths_are_honored() {
        let base = Catalog::default();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1400., 48.));
        let default_cells = table_cells(rect, &base);
        // 알람 (slot 1 -> column 4) starts at its default width.
        assert_eq!(default_cells[4].width(), COLUMN_WIDTHS[4]);

        let mut hidden = Catalog::default();
        hidden.column_hidden[1] = true;
        let cells = table_cells(rect, &hidden);
        assert_eq!(cells[4].width(), 0.);
        // The space freed by hiding 알람 goes to 내용 (column 3), not lost.
        assert!(cells[3].width() > default_cells[3].width());

        let mut resized = Catalog::default();
        resized.column_widths[1] = 150.;
        let cells = table_cells(rect, &resized);
        assert_eq!(cells[4].width(), 150.);

        // Out-of-range custom widths are clamped where they're applied in
        // the UI (the drag handler), not inside table_cells itself; a
        // sentinel of exactly 0. still falls back to the default rather
        // than collapsing the column, so "not customized" and "hidden" stay
        // distinct states.
        let mut zeroed = Catalog::default();
        zeroed.column_widths[1] = 0.;
        let cells = table_cells(rect, &zeroed);
        assert_eq!(cells[4].width(), COLUMN_WIDTHS[4]);
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
        for n in a.data.notes.iter_mut() {
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
