use crate::*;
use chrono::{Duration, Timelike};
use std::collections::{HashMap, HashSet};
#[derive(Clone)]
pub struct Folder {
    pub title: String,
    pub description: String,
    pub parent: Option<usize>,
}
pub struct Picker {
    pub owner: egui::ViewportId,
    pub target: usize,
    pub date: NaiveDate,
    pub month: NaiveDate,
    pub hour: u32,
    pub minute: u32,
}
impl Picker {
    pub fn new(target: usize, date: NaiveDate, time: chrono::NaiveTime) -> Self {
        Self {
            owner: if target == 0 || target == 8 {
                egui::ViewportId::from_hash_of("note_editor")
            } else {
                egui::ViewportId::ROOT
            },
            target,
            date,
            month: date.with_day(1).unwrap(),
            hour: time.hour(),
            minute: time.minute(),
        }
    }
}
pub struct Extra {
    pub media: Vec<crate::media::Asset>,
    pub capture_project: Option<usize>,
    pub scene: egui::Rect,
    pub scene_initialized: bool,
    pub scene_size: Vec2,
    pub zoom: f32,
    pub board_tab: usize,
    pub utilities_tab: usize,
    pub created_basis: bool,
    pub hidden_projects: HashSet<usize>,
    pub deleted_projects: HashSet<usize>,
    pub project_manager: bool,
    pub delete_project: Option<usize>,
    pub delete_notes: bool,
    pub folders: Vec<Folder>,
    pub folder_selected: Option<usize>,
    pub wiki_selected: Option<usize>,
    pub tree_selected: Option<usize>,
    pub drafts: HashMap<(usize, usize, Option<NaiveDate>), String>,
    pub wiki_unfiled: bool,
    pub folder_open: HashSet<usize>,
    pub completion_scope: Option<(usize, NaiveDate, bool)>,
    pub comment_occurrence: Option<NaiveDate>,
    pub edit_occurrence: Option<NaiveDate>,
    pub missing_image: Option<usize>,
    pub swatch_size: Vec2,
    pub swatch_drag: Option<crate::refinements::SwatchDrag>,
    pub journal_open: HashMap<(usize, Option<NaiveDate>), bool>,
    pub scope_owner: egui::ViewportId,
    pub journal_all: bool,
    pub picker: Option<Picker>,
    pub search_options: bool,
    pub search_mode: bool,
    pub search_created: bool,
    pub search_scheduled: bool,
    pub created_from: Option<NaiveDate>,
    pub created_to: Option<NaiveDate>,
    pub scheduled_from: Option<NaiveDate>,
    pub scheduled_to: Option<NaiveDate>,
    pub search_project: Option<usize>,
    pub image: bool,
    pub table: bool,
    pub link: bool,
    pub include_hidden: bool,
    pub media_from: Option<NaiveDate>,
    pub media_to: Option<NaiveDate>,
    pub table_search: String,
    pub bookmarks: Vec<(String, String)>,
    pub note_size: Vec2,
    pub size_applied: Option<(usize, u8)>,
    pub custom_color: bool,
}
impl Default for Extra {
    fn default() -> Self {
        Self {
            media: vec![],
            capture_project: None,
            scene: egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(920., 500.)),
            scene_initialized: false,
            scene_size: Vec2::new(920., 500.),
            zoom: 1.,
            board_tab: 0,
            utilities_tab: 0,
            created_basis: false,
            hidden_projects: HashSet::new(),
            deleted_projects: HashSet::new(),
            project_manager: false,
            delete_project: None,
            delete_notes: false,
            folders: vec![Folder {
                title: "내 지식".into(),
                description: "업무 중 쌓인 참고 정보".into(),
                parent: None,
            }],
            folder_selected: None,
            wiki_selected: None,
            tree_selected: None,
            drafts: HashMap::new(),
            wiki_unfiled: false,
            folder_open: HashSet::from([0]),
            completion_scope: None,
            comment_occurrence: None,
            edit_occurrence: None,
            missing_image: None,
            swatch_size: Vec2::new(48., 48.),
            swatch_drag: None,
            journal_open: HashMap::new(),
            scope_owner: egui::ViewportId::ROOT,
            journal_all: false,
            picker: None,
            search_options: false,
            search_mode: false,
            search_created: false,
            search_scheduled: false,
            created_from: None,
            created_to: None,
            scheduled_from: None,
            scheduled_to: None,
            search_project: None,
            image: false,
            table: false,
            link: false,
            include_hidden: false,
            media_from: None,
            media_to: None,
            table_search: String::new(),
            bookmarks: vec![("프로젝트 참고 자료".into(), "https://example.com".into())],
            note_size: Vec2::new(600., 640.),
            size_applied: None,
            custom_color: false,
        }
    }
}
impl App {
    pub fn visible(&self, i: usize) -> bool {
        let n = &self.data.notes[i];
        !n.deleted
            && !n.project.is_some_and(|p| {
                self.extra.hidden_projects.contains(&p) || self.extra.deleted_projects.contains(&p)
            })
    }
    pub fn matches_search(&self, i: usize) -> bool {
        let n = &self.data.notes[i];
        if n.deleted {
            return false;
        }
        if !(self.extra.search_mode && self.extra.include_hidden) && !self.visible(i) {
            return false;
        }
        if !self.extra.search_mode {
            return true;
        }
        let date_matches = |d: Option<NaiveDate>, a: Option<NaiveDate>, b: Option<NaiveDate>| {
            d.is_some_and(|d| a.is_none_or(|a| d >= a) && b.is_none_or(|b| d <= b))
        };
        (self.search.is_empty()
            || n.title.to_lowercase().contains(&self.search.to_lowercase())
            || n.body.to_lowercase().contains(&self.search.to_lowercase()))
            && (!self.extra.search_created
                || date_matches(
                    Some(n.created.date()),
                    self.extra.created_from,
                    self.extra.created_to,
                ))
            && (!self.extra.search_scheduled
                || n.days().into_iter().any(|d| {
                    date_matches(Some(d), self.extra.scheduled_from, self.extra.scheduled_to)
                }))
            && crate::refinements::project_match(self.extra.search_project, n.project)
            && (!self.extra.image || n.has_image)
            && (!self.extra.table || n.has_table)
            && (!self.extra.link || n.has_link || n.body.contains("http"))
    }
    pub fn consume_project_header(&mut self, i: usize) {
        let body = self.data.notes[i].body.clone();
        let title = self.data.notes[i].title.clone();
        let (name, from_body) =
            if let Some(line) = body.lines().next().filter(|l| l.starts_with('#')) {
                if !body.contains('\n') {
                    return;
                }
                (line.trim_start_matches('#').trim().to_owned(), true)
            } else if title.starts_with('#') && title.contains('\n') {
                (
                    title
                        .lines()
                        .next()
                        .unwrap()
                        .trim_start_matches('#')
                        .trim()
                        .to_owned(),
                    false,
                )
            } else {
                return;
            };
        if name.is_empty() {
            return;
        }
        let p = self
            .data
            .projects
            .iter()
            .enumerate()
            .find(|(p, s)| !self.extra.deleted_projects.contains(p) && **s == name)
            .map(|(p, _)| p)
            .unwrap_or_else(|| {
                self.data.projects.push(name.clone());
                self.data.projects.len() - 1
            });
        self.data.notes[i].project = Some(p);
        if from_body {
            self.data.notes[i].body = body.split_once('\n').unwrap().1.into();
            if title.starts_with('#') {
                self.data.notes[i].title = self.data.notes[i]
                    .body
                    .lines()
                    .next()
                    .unwrap_or("새 메모")
                    .into();
            }
        } else {
            self.data.notes[i].title = title.split_once('\n').unwrap().1.into();
        }
        self.toast = format!("#{name} 프로젝트에 추가했습니다");
    }
    pub fn note_options(&mut self, ui: &mut egui::Ui, i: usize) {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.data.notes[i].topmost, "항상 위에");
            ui.menu_button("접었을 때 색상", |ui| {
                ui.horizontal_wrapped(|ui| {
                    for color in PALETTE {
                        if ui
                            .add_sized([26., 26.], egui::Button::new("").fill(color))
                            .clicked()
                        {
                            self.data.notes[i].color = color;
                            ui.close_menu();
                        }
                    }
                });
                if ui.button("사용자 지정 색상…").clicked() {
                    self.extra.custom_color = true;
                }
            });
            if self.extra.custom_color {
                ui.color_edit_button_srgba(&mut self.data.notes[i].color);
            }
            ui.label(
                RichText::new(format!(
                    "생성 {}",
                    self.data.notes[i].created.format("%Y-%m-%d %H:%M")
                ))
                .small()
                .color(MUTED),
            );
        });
    }
    pub fn detached_note(&mut self, ctx: &egui::Context) {
        if !self.editor && !self.sticky {
            return;
        }
        let i = self.active;
        if self.data.notes[i].deleted {
            self.editor = false;
            self.sticky = false;
            return;
        }
        let fold = self.data.notes[i].fold;
        let size = match fold {
            1 => Vec2::new(380., 42.),
            2 => self.extra.swatch_size,
            _ => self.extra.note_size,
        };
        let min_size = match fold {
            0 => Vec2::new(360., 320.),
            1 => size,
            _ => Vec2::new(24., 24.),
        };
        let max_size = match fold {
            0 => Vec2::new(4096., 4096.),
            1 => size,
            _ => Vec2::new(360., 360.),
        };
        let id = egui::ViewportId::from_hash_of("note_editor");
        let mut open = true;
        let title = shorten(&self.data.notes[i].title, 32);
        let top = self.data.notes[i].topmost;
        ctx.show_viewport_immediate(
            id,
            egui::ViewportBuilder::default()
                .with_title(title)
                .with_decorations(fold == 0)
                .with_inner_size(size)
                .with_min_inner_size(min_size)
                .with_resizable(fold != 1)
                .with_window_level(if top {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                }),
            |ctx, _| {
                self.theme(ctx);
                if fold == 0 {
                    if let Some(r) = ctx.input(|i| i.viewport().inner_rect) {
                        if r.width() > 350. && r.height() > 300. {
                            self.extra.note_size = r.size();
                        }
                    }
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if top {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                }));
                if self.extra.size_applied != Some((i, fold)) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(fold == 0));
                    ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min_size));
                    ctx.send_viewport_cmd(egui::ViewportCommand::MaxInnerSize(max_size));
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(fold != 1));
                    self.extra.size_applied = Some((i, fold));
                }
                if fold == 2 && self.extra.size_applied == Some((i, 2)) {
                    if let Some(r) = ctx.input(|i| i.viewport().inner_rect) {
                        if r.width() >= 24.
                            && r.width() <= 360.
                            && r.height() >= 24.
                            && r.height() <= 360.
                        {
                            self.extra.swatch_size = r.size();
                        }
                    }
                }
                if ctx.input(|i| i.viewport().close_requested()) {
                    open = false;
                }
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::NONE
                            .fill(if fold > 0 {
                                self.data.notes[i].color
                            } else {
                                ctx.style().visuals.panel_fill
                            })
                            .inner_margin(if fold == 2 { 0. } else { 6. }),
                    )
                    .show(ctx, |ui| {
                        if fold == 2 {
                            self.swatch_ui(ctx, ui, i);
                        } else {
                            ui.horizontal(|ui| {
                                if ui.button("접기").clicked() {
                                    self.data.notes[i].fold = (fold + 1) % 3;
                                    self.extra.picker = None;
                                }
                                if fold == 1 {
                                    let r = ui.add(
                                        egui::Label::new(
                                            RichText::new(shorten(&self.data.notes[i].title, 12))
                                                .color(INK),
                                        )
                                        .sense(egui::Sense::drag()),
                                    );
                                    if r.drag_started() {
                                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                                    }
                                    if ui.small_button("펼치기").clicked() {
                                        self.data.notes[i].fold = 0;
                                    }
                                    if ui.small_button("×").clicked() {
                                        open = false;
                                    }
                                }
                            });
                            if fold == 0 {
                                egui::ScrollArea::vertical().show(ui, |ui| self.note_editor(ui));
                            }
                        }
                    });
                self.picker_window(ctx);
                self.scope_dialog(ctx);
            },
        );
        if !open {
            self.editor = false;
            self.sticky = false;
            self.extra.size_applied = None;
        }
    }
    pub fn date_controls(&mut self, ui: &mut egui::Ui, i: usize) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(format!(
                    "날짜·시간 선택  {} {}",
                    self.date_edit, self.time_edit
                ))
                .clicked()
            {
                self.extra.picker = Some(Picker::new(
                    0,
                    self.data.notes[i].date.unwrap_or(Local::now().date_naive()),
                    self.data.notes[i].time,
                ));
            }
            if self.data.notes[i].completed.is_some() && ui.button("완료 시각 달력").clicked()
            {
                let t = self.data.notes[i]
                    .completed
                    .unwrap_or(Local::now().naive_local());
                self.extra.picker = Some(Picker::new(8, t.date(), t.time()));
            }
        });
    }
    pub fn picker_window(&mut self, ctx: &egui::Context) {
        let Some(mut p) = self.extra.picker.take() else {
            return;
        };
        if p.owner != ctx.viewport_id() {
            self.extra.picker = Some(p);
            return;
        }
        let mut open = true;
        let mut applied = false;
        egui::Window::new("날짜와 시간 선택")
            .id(egui::Id::new("date_picker"))
            .open(&mut open)
            .default_width(300.)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("〈").clicked() {
                        p.month = (p.month - Duration::days(1)).with_day(1).unwrap();
                    }
                    ui.strong(p.month.format("%Y년 %m월").to_string());
                    if ui.button("〉").clicked() {
                        p.month = (p.month + Duration::days(32)).with_day(1).unwrap();
                    }
                });
                egui::Grid::new("picker_days")
                    .spacing([4., 4.])
                    .show(ui, |ui| {
                        for s in ["일", "월", "화", "수", "목", "금", "토"] {
                            ui.label(s);
                        }
                        ui.end_row();
                        let offset = p.month.weekday().num_days_from_sunday() as i64;
                        for slot in 0..42 {
                            let d = p.month + Duration::days(slot - offset);
                            if d.month() == p.month.month() {
                                if ui
                                    .add_sized(
                                        [30., 27.],
                                        egui::Button::new(d.day().to_string())
                                            .selected(p.date == d),
                                    )
                                    .clicked()
                                {
                                    p.date = d;
                                }
                            } else {
                                ui.label("");
                            }
                            if slot % 7 == 6 {
                                ui.end_row();
                            }
                        }
                    });
                ui.horizontal(|ui| {
                    ui.label("시간");
                    ui.add(egui::DragValue::new(&mut p.hour).range(0..=23));
                    ui.label(":");
                    ui.add(egui::DragValue::new(&mut p.minute).range(0..=59));
                });
                ui.horizontal(|ui| {
                    if ui.button("오늘").clicked() {
                        p.date = Local::now().date_naive();
                        p.month = p.date.with_day(1).unwrap();
                    }
                    if ui.button("선택 적용").clicked() {
                        applied = true;
                    }
                });
            });
        if applied {
            match p.target {
                0 => {
                    let n = &mut self.data.notes[self.active];
                    n.move_start(p.date);
                    n.time = chrono::NaiveTime::from_hms_opt(p.hour, p.minute, 0).unwrap();
                    self.date_edit = p.date.to_string();
                    self.time_edit = n.time.format("%H:%M").to_string();
                }
                1 => self.journal_date = p.date.to_string(),
                2 => {
                    self.extra.created_from = Some(p.date);
                    self.extra.created_to = Some(p.date);
                }
                3 => {
                    let (a, b) = crate::refinements::ordered_range(
                        self.extra.created_from.unwrap_or(p.date),
                        p.date,
                    );
                    self.extra.created_from = Some(a);
                    self.extra.created_to = Some(b);
                }
                4 => {
                    self.extra.scheduled_from = Some(p.date);
                    self.extra.scheduled_to = Some(p.date);
                }
                5 => {
                    let (a, b) = crate::refinements::ordered_range(
                        self.extra.scheduled_from.unwrap_or(p.date),
                        p.date,
                    );
                    self.extra.scheduled_from = Some(a);
                    self.extra.scheduled_to = Some(b);
                }
                6 => {
                    self.extra.media_from = Some(p.date);
                    self.extra.media_to = Some(p.date);
                }
                7 => {
                    let (a, b) = crate::refinements::ordered_range(
                        self.extra.media_from.unwrap_or(p.date),
                        p.date,
                    );
                    self.extra.media_from = Some(a);
                    self.extra.media_to = Some(b);
                }
                8 => {
                    let t = p.date.and_hms_opt(p.hour, p.minute, 0).unwrap();
                    self.set_completion_time(t);
                }
                _ => {}
            }
        } else if open {
            self.extra.picker = Some(p);
        }
    }
    pub fn date_filter_button(
        &mut self,
        ui: &mut egui::Ui,
        target: usize,
        name: &str,
        date: Option<NaiveDate>,
    ) {
        if ui
            .button(format!(
                "{} {}",
                name,
                date.map(|d| d.to_string()).unwrap_or("선택".into())
            ))
            .clicked()
        {
            self.extra.picker = Some(Picker::new(
                target,
                date.unwrap_or(Local::now().date_naive()),
                chrono::NaiveTime::MIN,
            ));
        }
    }
}
impl App {
    pub fn comments(&mut self, ui: &mut egui::Ui, i: usize, channel: usize) {
        let scope = if channel == 1 {
            self.extra.comment_occurrence
        } else {
            None
        };
        let list = if let Some(d) = scope {
            self.data.notes[i]
                .occurrence_comments
                .get(&d)
                .cloned()
                .unwrap_or_default()
        } else if channel == 0 {
            self.data.notes[i].wiki_comments.clone()
        } else {
            self.data.notes[i].journal_comments.clone()
        };
        for c in list {
            ui.group(|ui| {
                ui.label(c);
            });
        }
        let draft = self.extra.drafts.entry((channel, i, scope)).or_default();
        crate::refinements::body_input(ui, draft, 3, "댓글을 입력하세요");
        if ui.button("댓글 추가").clicked() && !draft.trim().is_empty() {
            let c = format!(
                "{}\n{}",
                Local::now().format("%Y-%m-%d %H:%M"),
                draft.trim()
            );
            if let Some(d) = scope {
                self.data.notes[i]
                    .occurrence_comments
                    .entry(d)
                    .or_default()
                    .push(c)
            } else if channel == 0 {
                self.data.notes[i].wiki_comments.push(c)
            } else {
                self.data.notes[i].journal_comments.push(c)
            }
            draft.clear();
        }
    }
    pub fn relation_tree(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            let mut roots: Vec<_> = self
                .data
                .notes
                .iter()
                .enumerate()
                .filter(|(i, n)| self.visible(*i) && n.parents.iter().all(|&p| !self.visible(p)))
                .map(|(i, _)| i)
                .collect();
            roots.sort_by_key(|&i| {
                let connected =
                    self.data
                        .notes
                        .iter()
                        .enumerate()
                        .any(|(j, n)| self.visible(j) && n.parents.contains(&i))
                        || self.data.links.iter().any(|&(a, b)| {
                            (a == i && self.visible(b)) || (b == i && self.visible(a))
                        });
                (!connected, self.data.notes[i].created)
            });
            for i in roots {
                self.tree_branch(&mut cols[0], i, &mut vec![]);
            }
            if let Some(i) = self.extra.tree_selected.filter(|&i| self.visible(i)) {
                cols[1].heading(&self.data.notes[i].title);
                cols[1].label(&self.data.notes[i].body);
                if cols[1].button("편집창 열기").clicked() {
                    self.open_note(i);
                }
            } else {
                cols[1].label("제목을 선택하면 메모 내용이 표시됩니다.");
            }
        });
    }
    fn tree_branch(&mut self, ui: &mut egui::Ui, i: usize, path: &mut Vec<usize>) {
        if path.contains(&i) {
            return;
        }
        path.push(i);
        let children: Vec<_> = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(j, n)| self.visible(*j) && n.parents.contains(&i))
            .map(|(j, _)| j)
            .collect();
        if children.is_empty() {
            if ui
                .selectable_label(
                    self.extra.tree_selected == Some(i),
                    &self.data.notes[i].title,
                )
                .clicked()
            {
                self.extra.tree_selected = Some(i);
            }
        } else {
            egui::CollapsingHeader::new(&self.data.notes[i].title)
                .id_salt(("node", path.clone()))
                .default_open(true)
                .show(ui, |ui| {
                    if ui
                        .selectable_label(self.extra.tree_selected == Some(i), "이 메모 보기")
                        .clicked()
                    {
                        self.extra.tree_selected = Some(i);
                    }
                    for c in children {
                        self.tree_branch(ui, c, path);
                    }
                });
        }
        let related: Vec<_> = self
            .data
            .links
            .iter()
            .filter_map(|&(a, b)| {
                if a == i {
                    Some(b)
                } else if b == i {
                    Some(a)
                } else {
                    None
                }
            })
            .filter(|&j| self.visible(j) && !path.contains(&j))
            .collect();
        if !related.is_empty() {
            ui.indent(("related_refs", path.clone()), |ui| {
                for j in related {
                    if ui
                        .selectable_label(
                            self.extra.tree_selected == Some(j),
                            format!("연결: {}", self.data.notes[j].title),
                        )
                        .clicked()
                    {
                        self.extra.tree_selected = Some(j);
                    }
                }
            });
        }
        path.pop();
    }
    pub fn auto_layout(&mut self) {
        let ids: Vec<_> = (0..self.data.notes.len())
            .filter(|&i| self.visible(i))
            .collect();
        for (k, i) in ids.into_iter().enumerate() {
            self.data.notes[i].pos =
                Vec2::new(30. + (k % 3) as f32 * 300., 30. + (k / 3) as f32 * 240.);
        }
        self.drag = None;
        self.extra.scene_initialized = false;
        self.toast = "메모 카드를 격자로 정렬했습니다 · 관계는 유지됩니다".into();
    }
    fn project_window(&mut self, ctx: &egui::Context) {
        let mut open = self.extra.project_manager;
        egui::Window::new("프로젝트 관리")
            .open(&mut open)
            .default_width(520.)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.project_name);
                    if ui.button("프로젝트 추가").clicked() && !self.project_name.trim().is_empty()
                    {
                        let name = self.project_name.trim().to_owned();
                        if !self
                            .data
                            .projects
                            .iter()
                            .enumerate()
                            .any(|(p, s)| !self.extra.deleted_projects.contains(&p) && *s == name)
                        {
                            self.data.projects.push(name);
                        }
                    }
                });
                for p in 0..self.data.projects.len() {
                    if self.extra.deleted_projects.contains(&p) {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        ui.label(&self.data.projects[p]);
                        let hidden = self.extra.hidden_projects.contains(&p);
                        if ui
                            .button(if hidden {
                                "다시 활성화"
                            } else {
                                "관련 메모 함께 숨기기"
                            })
                            .clicked()
                        {
                            if hidden {
                                self.extra.hidden_projects.remove(&p);
                            } else {
                                self.extra.hidden_projects.insert(p);
                                if self.data.notes[self.active].project == Some(p) {
                                    self.editor = false;
                                    self.sticky = false;
                                    self.extra.size_applied = None;
                                }
                                self.picked = self
                                    .picked
                                    .iter()
                                    .copied()
                                    .filter(|&i| self.visible(i))
                                    .collect();
                            }
                            self.project_filter = None;
                        }
                        if ui.button("삭제…").clicked() {
                            self.extra.delete_project = Some(p);
                            self.extra.delete_notes = false;
                        }
                    });
                }
            });
        self.extra.project_manager = open;
        if let Some(p) = self.extra.delete_project {
            egui::Window::new("프로젝트 삭제 범위")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(format!("{} 프로젝트", self.data.projects[p]));
                    ui.radio_value(
                        &mut self.extra.delete_notes,
                        false,
                        "이름만 삭제 · 메모는 유지",
                    );
                    ui.radio_value(
                        &mut self.extra.delete_notes,
                        true,
                        "연관된 메모도 모두 삭제",
                    );
                    if ui.button("선택 범위 삭제").clicked() {
                        self.delete_project(p, self.extra.delete_notes);
                        self.extra.delete_project = None;
                    }
                    if ui.button("취소").clicked() {
                        self.extra.delete_project = None;
                    }
                });
        }
    }
    pub fn delete_project(&mut self, p: usize, delete_notes: bool) {
        self.extra.deleted_projects.insert(p);
        self.extra.hidden_projects.remove(&p);
        for n in &mut self.data.notes {
            if n.project == Some(p) {
                n.project = None;
                if delete_notes {
                    n.deleted = true;
                }
            }
        }
        self.project_filter = None;
        self.picked.retain(|&i| !self.data.notes[i].deleted);
        if self.data.notes[self.active].deleted {
            self.editor = false;
            self.sticky = false;
        }
        self.toast = "프로젝트 삭제 범위를 반영했습니다 · 종료 시 초기화되는 미리보기입니다".into();
    }
    fn search_window(&mut self, ctx: &egui::Context) {
        let mut open = self.extra.search_options;
        egui::Window::new("상세 검색")
            .open(&mut open)
            .default_width(560.)
            .show(ctx, |ui| {
                ui.label("제목과 본문을 함께 검색합니다. 선택한 조건은 모두 만족해야 합니다.");
                ui.checkbox(&mut self.extra.search_created, "생성일 조건");
                ui.horizontal(|ui| {
                    self.date_filter_button(ui, 2, "시작", self.extra.created_from);
                    self.date_filter_button(ui, 3, "끝", self.extra.created_to);
                });
                ui.checkbox(&mut self.extra.search_scheduled, "일정 조건");
                ui.horizontal(|ui| {
                    self.date_filter_button(ui, 4, "시작", self.extra.scheduled_from);
                    self.date_filter_button(ui, 5, "끝", self.extra.scheduled_to);
                });
                egui::ComboBox::from_id_salt("search_project")
                    .selected_text(crate::refinements::project_label(
                        self.extra.search_project,
                        &self.data.projects,
                    ))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.extra.search_project, None, "모든 프로젝트");
                        ui.selectable_value(
                            &mut self.extra.search_project,
                            Some(NO_PROJECT),
                            "프로젝트 없음",
                        );
                        for (p, name) in self.data.projects.iter().enumerate() {
                            if !self.extra.deleted_projects.contains(&p) {
                                ui.selectable_value(&mut self.extra.search_project, Some(p), name);
                            }
                        }
                    });
                ui.checkbox(&mut self.extra.image, "첨부 이미지 있음");
                ui.checkbox(&mut self.extra.table, "첨부 표 있음");
                ui.checkbox(&mut self.extra.link, "첨부 링크 있음");
                ui.checkbox(&mut self.extra.include_hidden, "숨긴 프로젝트 포함");
                ui.checkbox(&mut self.extra.created_basis, "생성시간 기준으로 정렬·표시");
                let invalid = (self.extra.search_created
                    && self
                        .extra
                        .created_from
                        .zip(self.extra.created_to)
                        .is_some_and(|(a, b)| a > b))
                    || (self.extra.search_scheduled
                        && self
                            .extra
                            .scheduled_from
                            .zip(self.extra.scheduled_to)
                            .is_some_and(|(a, b)| a > b));
                if invalid {
                    ui.colored_label(ACCENT, "시작 날짜가 끝 날짜보다 늦습니다");
                }
                if ui
                    .add_enabled(!invalid, egui::Button::new("검색 적용"))
                    .clicked()
                {
                    self.extra.search_mode = true;
                    self.page = 0;
                    self.project_filter = None;
                    self.split = true;
                }
            });
        self.extra.search_options = open;
    }
    pub fn extra_windows(&mut self, ctx: &egui::Context) {
        self.scope_dialog(ctx);
        self.missing_image_dialog(ctx);
        self.project_window(ctx);
        self.search_window(ctx);
        self.picker_window(ctx);
    }
}
impl App {
    pub fn utilities(&mut self, ui: &mut egui::Ui) {
        ui.heading("편의기능");
        ui.horizontal_wrapped(|ui| {
            for (t, name) in [
                "업무일지",
                "표 모아보기",
                "이미지 모아보기",
                "링크 · 즐겨찾기",
            ]
            .iter()
            .enumerate()
            {
                ui.selectable_value(&mut self.extra.utilities_tab, t, *name);
            }
        });
        ui.separator();
        match self.extra.utilities_tab {
            0 => self.journal(ui),
            1 => self.real_table_gallery(ui),
            2 => self.real_image_gallery(ui),
            _ => self.real_bookmark_gallery(ui),
        }
    }
}
impl App {
    pub fn capture_project_header(&mut self) {
        if self.capture_text.starts_with('#') {
            if let Some((line, rest)) = self.capture_text.split_once('\n') {
                let name = line.trim_start_matches('#').trim().to_owned();
                let rest = rest.to_owned();
                if !name.is_empty() {
                    let p = self
                        .data
                        .projects
                        .iter()
                        .enumerate()
                        .find(|(p, s)| !self.extra.deleted_projects.contains(p) && **s == name)
                        .map(|(p, _)| p)
                        .unwrap_or_else(|| {
                            self.data.projects.push(name.clone());
                            self.data.projects.len() - 1
                        });
                    self.extra.capture_project = Some(p);
                    self.capture_text = rest;
                    self.toast = format!("#{name} 프로젝트를 선택했습니다");
                }
            }
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
    fn hidden_project_is_excluded_but_explicit_search_can_include_it() {
        let mut a = app();
        a.extra.hidden_projects.insert(0);
        assert!(!a.visible(0));
        assert!(!a.matches_search(0));
        a.extra.search_mode = true;
        a.extra.include_hidden = true;
        assert!(a.matches_search(0));
    }
    #[test]
    fn deleting_project_name_preserves_notes_and_comments() {
        let mut a = app();
        a.data.notes[0].wiki_comments.push("댓글".into());
        a.delete_project(0, false);
        assert!(!a.data.notes[0].deleted);
        assert_eq!(a.data.notes[0].project, None);
        assert_eq!(a.data.notes[0].wiki_comments, vec!["댓글"]);
    }
    #[test]
    fn deleting_project_with_notes_does_not_delete_other_projects() {
        let mut a = app();
        a.delete_project(0, true);
        assert!(a.data.notes[0].deleted);
        assert!(a.data.notes[1].deleted);
        assert!(!a.data.notes[2].deleted);
    }
    #[test]
    fn project_header_creates_one_project_and_preserves_content() {
        let mut a = app();
        a.capture_text = "#새 프로젝트\n본문".into();
        a.capture_project_header();
        let p = a.extra.capture_project.unwrap();
        assert_eq!(a.data.projects[p], "새 프로젝트");
        assert_eq!(a.capture_text, "본문");
        a.capture_text = "#새 프로젝트\n두번째".into();
        a.capture_project_header();
        assert_eq!(a.extra.capture_project, Some(p));
        assert_eq!(a.data.projects.len(), 3);
    }
    #[test]
    fn wiki_and_journal_comments_are_independent() {
        let mut a = app();
        a.data.notes[0].wiki_comments.push("위키 댓글".into());
        a.done(0, true);
        assert!(a.data.notes[0].wiki);
        assert!(a.data.notes[0].journal_comments.is_empty());
    }
    #[test]
    fn arranging_cards_preserves_dag_and_related_links() {
        let mut a = app();
        assert!(a.data.reparent(2, 0));
        assert!(a.data.reparent(2, 1));
        let links = a.data.links.clone();
        a.data.notes[0].pos = Vec2::new(800., 900.);
        a.auto_layout();
        assert_eq!(a.data.notes[2].parents, vec![0, 1]);
        assert_eq!(a.data.links, links);
        assert_eq!(a.data.notes[0].pos, Vec2::new(30., 30.));
    }
}
