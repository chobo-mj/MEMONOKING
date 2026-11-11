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
    pub bound_note: Option<(usize, Option<NaiveDate>)>,
    pub date: NaiveDate,
    pub month: NaiveDate,
    pub hour: u32,
    pub minute: u32,
}
impl Picker {
    pub fn new(target: usize, date: NaiveDate, time: chrono::NaiveTime) -> Self {
        Self {
            owner: if target == 0 || target == 8 || target == 9 {
                egui::ViewportId::from_hash_of("note_editor")
            } else {
                main_viewport()
            },
            target,
            bound_note: None,
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
    /// Show completed memos in the memo lists.
    pub show_done: bool,
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
    pub draft_documents: HashMap<(usize, usize, Option<NaiveDate>), crate::composer::Document>,
    pub comment_documents:
        HashMap<(usize, usize, Option<NaiveDate>, usize), crate::composer::Document>,
    pub wiki_unfiled: bool,
    pub folder_open: HashSet<usize>,
    pub completion_scope: Option<(usize, NaiveDate, bool)>,
    pub comment_occurrence: Option<NaiveDate>,
    pub edit_occurrence: Option<NaiveDate>,
    pub missing_image: Option<usize>,
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
    pub size_applied: Option<(usize, u8)>,
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
            show_done: false,
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
            draft_documents: HashMap::new(),
            comment_documents: HashMap::new(),
            wiki_unfiled: false,
            folder_open: HashSet::from([0]),
            completion_scope: None,
            comment_occurrence: None,
            edit_occurrence: None,
            missing_image: None,
            journal_open: HashMap::new(),
            scope_owner: main_viewport(),
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
            size_applied: None,
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
            || (!n.locked
                && n.document
                    .plain_text(&n.body)
                    .to_lowercase()
                    .contains(&self.search.to_lowercase())))
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
            && (!self.extra.image || (!n.locked && (n.has_image || n.document.has_images())))
            && (!self.extra.table || (!n.locked && (n.has_table || n.document.has_tables())))
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
        let color = self.data.notes[i].color;
        let r = crate::ui::icon_menu(ui, crate::icons::Icon::Palette, "메모 색상", |ui| {
            crate::desktop::color_choices(ui, &mut self.data.notes[i].color)
        });
        // Paint the current color over the icon, as a swatch.
        let swatch = egui::Rect::from_center_size(r.rect.center(), Vec2::splat(18.));
        ui.painter().rect_filled(swatch, 5., color);
        ui.painter().rect_stroke(
            swatch,
            5.,
            egui::Stroke::new(1.0_f32, Color32::from_black_alpha(35)),
            egui::StrokeKind::Inside,
        );
    }
    pub fn detached_note(&mut self, ctx: &egui::Context) {
        self.render_note_windows(ctx);
    }
    pub fn date_controls(&mut self, ui: &mut egui::Ui, i: usize) {
        let start = self.data.notes[i].date;
        let time = self.data.notes[i].time;
        let end = start.map(|d| d + Duration::days(self.data.notes[i].duration.max(1) as i64 - 1));
        let end_time = self.data.notes[i].end_time.unwrap_or(time);
        let created = format!(
            "{} 생성",
            self.data.notes[i].created.format("%Y.%m.%d %H:%M")
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.;
            let label = start
                .map(|d| format!("{} {}", d.format("%m.%d"), time.format("%H:%M")))
                .unwrap_or_else(|| "일정 시작".into());
            // Both ends open the same schedule popup the sticker uses.
            let mut schedule =
                crate::icons::text_button(ui, crate::icons::Icon::Calendar, &label, "일정 설정")
                    .clicked();
            ui.label(RichText::new("→").color(crate::ui::t(ui).faint));
            let label = end
                .map(|d| format!("{} {}", d.format("%m.%d"), end_time.format("%H:%M")))
                .unwrap_or_else(|| "일정 완료".into());
            schedule |=
                crate::icons::text_button(ui, crate::icons::Icon::Calendar, &label, "일정 설정")
                    .clicked();
            ui.add_space(4.);
            let n = &mut self.data.notes[i];
            let repeat = match (n.date, n.repeat.is_none()) {
                (Some(d), false) => n.repeat.describe(d),
                _ => "정기 일정".into(),
            };
            schedule |= crate::ui::pill(ui, n.recurring || !n.repeat.is_none(), &repeat)
                .on_hover_text("반복 일정 설정")
                .clicked();
            if crate::ui::pill(ui, n.wiki, "위키").clicked() {
                n.wiki = !n.wiki;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                crate::ui::hint(ui, &created);
            });
            if schedule {
                self.open_schedule(i);
            }
        });
        ui.add_space(4.);
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt(("memo_project", i))
                .width(170.)
                .selected_text(format!(
                    "# {}",
                    self.data.notes[i]
                        .project
                        .map(|p| self.data.projects[p].as_str())
                        .unwrap_or("프로젝트 없음")
                ))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.data.notes[i].project, None, "프로젝트 없음");
                    for (p, name) in self.data.projects.iter().enumerate() {
                        if !self.extra.hidden_projects.contains(&p)
                            && !self.extra.deleted_projects.contains(&p)
                        {
                            ui.selectable_value(&mut self.data.notes[i].project, Some(p), name);
                        }
                    }
                });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(t) = self.editor_completion() {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(format!(
                                    "완료 시간 : {}",
                                    t.format("%Y.%m.%d %H:%M")
                                ))
                                .small()
                                .color(ACCENT),
                            )
                            .frame(false),
                        )
                        .on_hover_text("완료 기록 날짜와 시간 수정")
                        .clicked()
                    {
                        let mut p = Picker::new(8, t.date(), t.time());
                        p.owner = ui.ctx().viewport_id();
                        p.bound_note = Some((i, self.extra.edit_occurrence));
                        self.extra.picker = Some(p);
                    }
                }
            });
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
        if p.bound_note
            .is_some_and(|bound| bound != (self.active, self.extra.edit_occurrence))
        {
            return;
        }
        if p.target == 8 && self.editor_completion().is_none() {
            return;
        }
        let mut open = true;
        let mut applied = false;
        let title = match p.target {
            0 => "일정 시작",
            9 => "일정 종료",
            8 => "완료 시간",
            _ => "날짜 선택",
        };
        let with_time = matches!(p.target, 0 | 8 | 9);
        crate::ui::dialog(
            ctx,
            ("date_picker", p.target),
            title,
            [300., if with_time { 410. } else { 370. }],
            true,
            &mut open,
            |ui| {
                ui.label(RichText::new(title).strong().size(15.));
                ui.add_space(6.);
                let mut day = p.date;
                crate::ui::month_grid(ui, &mut p.month, &mut day);
                p.date = day;
                if with_time {
                    ui.add_space(4.);
                    crate::ui::time_row(ui, &mut p.hour, &mut p.minute);
                }
                ui.add_space(6.);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::ui::primary(ui, "적용").clicked() {
                        applied = true;
                    }
                    if ui.button("오늘").clicked() {
                        p.date = Local::now().date_naive();
                        p.month = p.date.with_day(1).unwrap();
                    }
                });
            },
        );
        if applied {
            match p.target {
                0 => {
                    let n = &mut self.data.notes[self.active];
                    n.move_start(p.date);
                    n.time = chrono::NaiveTime::from_hms_opt(p.hour, p.minute, 0).unwrap();
                    if n.duration == 1 && n.end_time.is_some_and(|t| t < n.time) {
                        n.end_time = Some(n.time);
                    }
                    self.date_edit = p.date.to_string();
                    self.time_edit = n.time.format("%H:%M").to_string();
                }
                9 => {
                    let n = &mut self.data.notes[self.active];
                    let chosen = p.date.and_hms_opt(p.hour, p.minute, 0).unwrap();
                    let current = n.date.unwrap_or(p.date).and_time(n.time);
                    n.set_span(current, chosen);
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
        if self.data.notes[i].locked {
            ui.label("잠긴 메모의 댓글은 표시하지 않습니다.");
            return;
        }
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
        for (index, c) in list.into_iter().enumerate() {
            ui.group(|ui| {
                if let Some(doc) = self
                    .extra
                    .comment_documents
                    .get(&(channel, i, scope, index))
                {
                    let (timestamp, body) = c.split_once('\n').unwrap_or(("", &c));
                    ui.label(RichText::new(timestamp).small().color(MUTED));
                    crate::composer::view(
                        ui,
                        body,
                        doc,
                        ("comment_view", channel, i, scope, index),
                    );
                } else {
                    ui.label(c);
                }
            });
        }
        let key = (channel, i, scope);
        let mut draft = self.extra.drafts.remove(&key).unwrap_or_default();
        let mut document = self.extra.draft_documents.remove(&key).unwrap_or_default();
        document.toolbar_visible = true;
        crate::composer::show(
            ui,
            &mut draft,
            &mut document,
            ("comment_draft", channel, i, scope),
        );
        if ui.button("댓글 추가").clicked()
            && (!draft.trim().is_empty()
                || document.has_tables()
                || document.has_images()
                || document.tabs.len() > 1)
        {
            let c = format!("{}\n{}", Local::now().format("%Y-%m-%d %H:%M"), draft);
            let index = if let Some(d) = scope {
                let comments = self.data.notes[i].occurrence_comments.entry(d).or_default();
                let index = comments.len();
                comments.push(c);
                index
            } else if channel == 0 {
                let index = self.data.notes[i].wiki_comments.len();
                self.data.notes[i].wiki_comments.push(c);
                index
            } else {
                let index = self.data.notes[i].journal_comments.len();
                self.data.notes[i].journal_comments.push(c);
                index
            };
            self.extra
                .comment_documents
                .insert((channel, i, scope, index), document);
            document = Default::default();
            draft.clear();
        }
        self.extra.drafts.insert(key, draft);
        self.extra.draft_documents.insert(key, document);
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
                cols[1].label(if self.data.notes[i].locked {
                    "잠긴 메모"
                } else {
                    &self.data.notes[i].body
                });
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
        crate::ui::dialog(
            ctx,
            "project_manager",
            "프로젝트 관리",
            [460., 420.],
            false,
            &mut open,
            |ui| {
                ui.label(RichText::new("프로젝트").strong().size(15.));
                crate::ui::hint(
                    ui,
                    "메모 본문 첫 줄에 #이름 + 스페이스로도 만들 수 있습니다",
                );
                ui.add_space(8.);
                ui.horizontal(|ui| {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.project_name)
                            .desired_width(ui.available_width() - 70.)
                            .margin(egui::Margin::symmetric(10, 6))
                            .hint_text("새 프로젝트 이름"),
                    );
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if (crate::ui::primary(ui, "추가").clicked() || enter)
                        && !self.project_name.trim().is_empty()
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
                        self.project_name.clear();
                    }
                });
                ui.add_space(10.);
                crate::ui::divider(ui);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for p in 0..self.data.projects.len() {
                        if self.extra.deleted_projects.contains(&p) {
                            continue;
                        }
                        let hidden = self.extra.hidden_projects.contains(&p);
                        ui.horizontal(|ui| {
                            ui.set_min_height(34.);
                            let tone = crate::ui::t(ui);
                            ui.label(RichText::new("#").color(tone.faint));
                            ui.label(if hidden {
                                RichText::new(&self.data.projects[p]).color(tone.faint)
                            } else {
                                RichText::new(&self.data.projects[p])
                            });
                            if hidden {
                                crate::ui::chip(ui, "숨김", tone.field, tone.muted);
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if crate::icons::small_icon(
                                        ui,
                                        crate::icons::Icon::Trash,
                                        "프로젝트 삭제",
                                    )
                                    .clicked()
                                    {
                                        self.extra.delete_project = Some(p);
                                        self.extra.delete_notes = false;
                                    }
                                    if ui
                                        .button(if hidden {
                                            "다시 보이기"
                                        } else {
                                            "숨기기"
                                        })
                                        .on_hover_text("숨기면 관련 메모도 함께 숨겨집니다")
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
                                },
                            );
                        });
                        crate::ui::divider(ui);
                    }
                });
            },
        );
        self.extra.project_manager = open;
        if let Some(p) = self.extra.delete_project {
            let mut open = true;
            crate::ui::dialog(
                ctx,
                "delete_project",
                "프로젝트 삭제",
                [380., 250.],
                true,
                &mut open,
                |ui| {
                    ui.label(
                        RichText::new(format!("‘{}’ 프로젝트를 삭제합니다", self.data.projects[p]))
                            .strong()
                            .size(15.),
                    );
                    ui.add_space(8.);
                    ui.radio_value(
                        &mut self.extra.delete_notes,
                        false,
                        "이름만 삭제 · 메모는 유지",
                    );
                    ui.radio_value(
                        &mut self.extra.delete_notes,
                        true,
                        "연관된 메모도 모두 휴지통으로",
                    );
                    let (ok, cancel) = crate::ui::dialog_buttons(ui, "삭제", "취소");
                    if ok {
                        self.delete_project(p, self.extra.delete_notes);
                        self.extra.delete_project = None;
                    }
                    if cancel {
                        self.extra.delete_project = None;
                    }
                },
            );
            if !open {
                self.extra.delete_project = None;
            }
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
        crate::ui::dialog(
            ctx,
            "search_options",
            "상세 검색",
            [460., 470.],
            false,
            &mut open,
            |ui| {
                ui.label(RichText::new("상세 검색").strong().size(15.));
                crate::ui::hint(
                    ui,
                    "제목과 본문을 함께 검색합니다 · 선택한 조건을 모두 만족해야 합니다",
                );
                ui.add_space(8.);
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .desired_width(f32::INFINITY)
                        .margin(egui::Margin::symmetric(10, 7))
                        .hint_text("검색어"),
                );
                ui.add_space(8.);
                crate::ui::toggle_row(ui, &mut self.extra.search_created, "생성일 범위", "");
                if self.extra.search_created {
                    ui.horizontal(|ui| {
                        self.date_filter_button(ui, 2, "시작", self.extra.created_from);
                        self.date_filter_button(ui, 3, "끝", self.extra.created_to);
                    });
                }
                crate::ui::toggle_row(ui, &mut self.extra.search_scheduled, "일정 범위", "");
                if self.extra.search_scheduled {
                    ui.horizontal(|ui| {
                        self.date_filter_button(ui, 4, "시작", self.extra.scheduled_from);
                        self.date_filter_button(ui, 5, "끝", self.extra.scheduled_to);
                    });
                }
                crate::ui::field(ui, "프로젝트", |ui| {
                    egui::ComboBox::from_id_salt("search_project")
                        .width(180.)
                        .selected_text(crate::refinements::project_label(
                            self.extra.search_project,
                            &self.data.projects,
                        ))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.extra.search_project,
                                None,
                                "모든 프로젝트",
                            );
                            ui.selectable_value(
                                &mut self.extra.search_project,
                                Some(NO_PROJECT),
                                "프로젝트 없음",
                            );
                            for (p, name) in self.data.projects.iter().enumerate() {
                                if !self.extra.deleted_projects.contains(&p) {
                                    ui.selectable_value(
                                        &mut self.extra.search_project,
                                        Some(p),
                                        name,
                                    );
                                }
                            }
                        });
                });
                ui.add_space(6.);
                ui.horizontal_wrapped(|ui| {
                    for (value, label) in [
                        (&mut self.extra.image, "이미지 있음"),
                        (&mut self.extra.table, "표 있음"),
                        (&mut self.extra.link, "링크 있음"),
                        (&mut self.extra.include_hidden, "숨긴 프로젝트 포함"),
                        (&mut self.extra.created_basis, "생성시간 기준 정렬"),
                    ] {
                        if crate::ui::pill(ui, *value, label).clicked() {
                            *value = !*value;
                        }
                    }
                });
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
                ui.add_space(8.);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            !invalid,
                            egui::Button::new(RichText::new("검색").color(Color32::WHITE).strong())
                                .fill(ACCENT),
                        )
                        .clicked()
                    {
                        self.extra.search_mode = true;
                        self.page = 0;
                        self.project_filter = None;
                        self.split = true;
                    }
                });
            },
        );
        self.extra.search_options = open;
    }
    pub fn extra_windows(&mut self, ctx: &egui::Context) {
        self.missing_image_dialog(ctx);
        self.project_window(ctx);
        self.search_window(ctx);
    }
}
impl App {
    pub fn utilities(&mut self, ui: &mut egui::Ui) {
        ui::tabs(
            ui,
            &mut self.extra.utilities_tab,
            &[
                "업무일지",
                "표 모아보기",
                "이미지 모아보기",
                "링크 · 즐겨찾기",
                "휴지통",
            ],
        );
        ui.add_space(12.);
        match self.extra.utilities_tab {
            0 => self.journal(ui),
            1 => self.content_tables(ui),
            2 => self.content_images(ui),
            3 => self.real_bookmark_gallery(ui),
            _ => self.trash_ui(ui),
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

#[cfg(test)]
mod picker_binding_tests {
    use super::*;
    #[test]
    fn time_picker_for_one_completed_occurrence_cannot_edit_another_completed_day() {
        let first = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        let next = first + Duration::days(1);
        let mut app = App::sample(first);
        app.data.notes[0].duration = 3;
        let t0 = first.and_hms_opt(10, 0, 0).unwrap();
        let t1 = next.and_hms_opt(11, 0, 0).unwrap();
        app.complete_span(0, first, true, false, t0);
        app.complete_span(0, next, true, false, t1);
        app.open_note_on(0, Some(first));
        let ctx = egui::Context::default();
        let mut picker = Picker::new(8, first, t0.time());
        picker.owner = ctx.viewport_id();
        picker.bound_note = Some((0, Some(first)));
        app.extra.picker = Some(picker);
        app.open_note_on(0, Some(next));
        assert_eq!(app.editor_completion(), Some(t1));
        app.picker_window(&ctx);
        assert!(app.extra.picker.is_none());
        assert_eq!(
            app.data.notes[0].occurrence_completed.get(&first),
            Some(&t0)
        );
        assert_eq!(app.data.notes[0].occurrence_completed.get(&next), Some(&t1));
    }
}
