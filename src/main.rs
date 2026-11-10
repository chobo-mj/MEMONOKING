#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod catalog;
mod composer;
mod content_collections;
mod desktop;
mod dragging;
mod extras;
mod icons;
mod media;
mod model;
mod native_window;
mod refinements;
mod studio;
mod views;
use chrono::{Datelike, Local, NaiveDate};
use eframe::egui::{self, Color32, RichText, Vec2};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use model::{Note, Workspace};
const PALETTE: [Color32; 10] = [
    Color32::from_rgb(246, 211, 103),
    Color32::from_rgb(239, 161, 149),
    Color32::from_rgb(176, 202, 160),
    Color32::from_rgb(151, 194, 215),
    Color32::from_rgb(181, 165, 213),
    Color32::from_rgb(231, 178, 203),
    Color32::from_rgb(225, 187, 142),
    Color32::from_rgb(145, 205, 185),
    Color32::from_rgb(185, 190, 195),
    Color32::from_rgb(228, 222, 204),
];
const NO_PROJECT: usize = usize::MAX;
const INK: Color32 = Color32::from_rgb(48, 47, 44);
const MUTED: Color32 = Color32::from_rgb(113, 114, 116);
const ACCENT: Color32 = Color32::from_rgb(184, 72, 53);
#[derive(Clone)]
struct NoteDrag;
struct App {
    desktop: desktop::Desktop,
    catalog: catalog::Catalog,
    studio: studio::Studio,
    extra: extras::Extra,
    sources: Vec<dragging::Source>,
    targets: Vec<dragging::Target>,
    drag: Option<dragging::Drag>,
    data: Workspace,
    page: usize,
    active: usize,
    sidebar: bool,
    split: bool,
    dark: bool,
    search: String,
    editor: bool,
    sticky: bool,
    floating: bool,
    capture: bool,
    capture_text: String,
    settings: bool,
    month: NaiveDate,
    day: NaiveDate,
    project_filter: Option<usize>,
    recurring: bool,
    board_filter: usize,
    picked: Vec<usize>,
    project_name: String,
    link_source: Option<usize>,
    journal_month: bool,
    journal_date: String,
    toast: String,
    date_edit: String,
    time_edit: String,
    completion_edit: String,
    edit_error: String,
    hotkeys: Option<GlobalHotKeyManager>,
    registered: Vec<HotKey>,
    shortcuts: [String; 3],
    hotkey_status: String,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        if let Ok(bytes) = std::fs::read("C:/Windows/Fonts/malgun.ttf") {
            fonts
                .font_data
                .insert("korean".into(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "korean".into());
        }
        for (name, file, index) in [
            ("맑은 고딕", "malgun.ttf", 0),
            ("맑은 고딕 Bold", "malgunbd.ttf", 0),
            ("굴림", "gulim.ttc", 0),
            ("돋움", "gulim.ttc", 2),
            ("바탕", "batang.ttc", 0),
            ("궁서체", "batang.ttc", 2),
        ] {
            if let Ok(bytes) = std::fs::read(format!("C:/Windows/Fonts/{file}")) {
                let mut data = egui::FontData::from_owned(bytes);
                data.index = index;
                fonts.font_data.insert(name.into(), data.into());
                fonts.families.insert(
                    egui::FontFamily::Name(name.into()),
                    vec![name.into(), "korean".into()],
                );
            }
        }
        let fallback = fonts.families[&egui::FontFamily::Proportional].clone();
        for (key, entries) in &mut fonts.families {
            if matches!(key, egui::FontFamily::Name(_)) {
                for f in &fallback {
                    if !entries.contains(f) {
                        entries.push(f.clone());
                    }
                }
            }
        }
        cc.egui_ctx.set_fonts(fonts);
        let mut a = Self::sample(Local::now().date_naive());
        a.hotkeys = GlobalHotKeyManager::new().ok();
        a.apply_shortcuts();
        a
    }
    fn sample(today: NaiveDate) -> Self {
        Self {
            desktop: desktop::Desktop::default(),
            catalog: catalog::Catalog::default(),
            extra: extras::Extra::default(),
            studio: studio::Studio::default(),
            sources: vec![],
            targets: vec![],
            drag: None,
            data: Workspace::sample(today),
            page: 0,
            active: 0,
            sidebar: true,
            split: true,
            dark: false,
            search: String::new(),
            editor: false,
            sticky: false,
            floating: false,
            capture: false,
            capture_text: String::new(),
            settings: false,
            month: today.with_day(1).unwrap(),
            day: today,
            project_filter: None,
            recurring: true,
            board_filter: 0,
            picked: vec![],
            project_name: "새 프로젝트".into(),
            link_source: None,
            journal_month: false,
            journal_date: today.to_string(),
            toast: String::new(),
            date_edit: String::new(),
            time_edit: String::new(),
            completion_edit: String::new(),
            edit_error: String::new(),
            hotkeys: None,
            registered: vec![],
            shortcuts: [
                "Ctrl+Alt+N".into(),
                "Ctrl+Alt+C".into(),
                "Ctrl+Alt+S".into(),
            ],
            hotkey_status: String::new(),
        }
    }
    fn theme(&self, ctx: &egui::Context) {
        let mut v = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        v.panel_fill = if self.dark {
            Color32::from_rgb(29, 30, 29)
        } else {
            Color32::from_rgb(255, 254, 252)
        };
        v.override_text_color = Some(if self.dark {
            Color32::from_rgb(232, 231, 225)
        } else {
            INK
        });
        v.selection.bg_fill = if self.dark {
            Color32::from_rgb(80, 53, 45)
        } else {
            Color32::from_rgb(252, 236, 225)
        };
        v.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
        v.widgets.active.weak_bg_fill = v.selection.bg_fill;
        v.window_fill = v.panel_fill;
        v.window_corner_radius = egui::CornerRadius::same(10);
        v.window_stroke = egui::Stroke::new(
            1.0_f32,
            if self.dark {
                Color32::from_gray(65)
            } else {
                Color32::from_rgb(224, 225, 227)
            },
        );
        v.widgets.inactive.weak_bg_fill = if self.dark {
            Color32::from_rgb(42, 43, 41)
        } else {
            Color32::from_rgb(244, 244, 240)
        };
        v.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        v.widgets.hovered.weak_bg_fill = if self.dark {
            Color32::from_rgb(55, 55, 51)
        } else {
            Color32::from_rgb(235, 236, 230)
        };
        v.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
        v.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
        ctx.set_visuals(v);
        let selected = egui::FontFamily::Name(self.desktop.font.clone().into());
        let family = if ctx.fonts(|f| f.families().contains(&selected)) {
            selected
        } else {
            egui::FontFamily::Proportional
        };
        ctx.style_mut(|s| {
            s.spacing.item_spacing = Vec2::new(8., 8.);
            s.spacing.button_padding = Vec2::new(9., 6.);
            s.text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(16.));
            s.text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(15.));
            s.text_styles
                .insert(egui::TextStyle::Small, egui::FontId::proportional(12.));
            s.text_styles
                .insert(egui::TextStyle::Heading, egui::FontId::proportional(25.));
            for (style, font) in &mut s.text_styles {
                font.family = family.clone();
                if *style == egui::TextStyle::Body {
                    font.size = self.desktop.font_size * 4. / 3.;
                }
            }
        });
    }
    fn apply_shortcuts(&mut self) {
        let parsed = self
            .shortcuts
            .iter()
            .map(|s| s.parse::<HotKey>())
            .collect::<Result<Vec<_>, _>>();
        let Ok(new) = parsed else {
            self.hotkey_status = "Ctrl+Alt+N 형식으로 입력하세요".into();
            return;
        };
        if new
            .iter()
            .map(|h| h.id())
            .collect::<std::collections::HashSet<_>>()
            .len()
            != new.len()
        {
            self.hotkey_status = "각 동작에 서로 다른 단축키를 지정하세요".into();
            return;
        }
        let Some(m) = &self.hotkeys else {
            self.hotkey_status = "전역 단축키를 시작할 수 없습니다".into();
            return;
        };
        let old = self.registered.clone();
        for h in &old {
            let _ = m.unregister(*h);
        }
        self.registered.clear();
        for h in &new {
            if let Err(e) = m.register(*h) {
                for h in &self.registered {
                    let _ = m.unregister(*h);
                }
                self.registered.clear();
                let mut restored = true;
                for h in old {
                    if m.register(h).is_ok() {
                        self.registered.push(h);
                    } else {
                        restored = false;
                    }
                }
                if !restored {
                    for h in &self.registered {
                        let _ = m.unregister(*h);
                    }
                    self.registered.clear();
                }
                self.hotkey_status = if restored {
                    format!("등록 실패·기존 설정 복구: {e}")
                } else {
                    format!("등록 및 복구 실패·단축키 비활성: {e}")
                };
                return;
            }
            self.registered.push(*h);
        }
        self.hotkey_status = "전역 단축키 사용 중 · 앱 실행 중에 동작".into();
    }
    fn shortcut_label(&self, action: usize) -> String {
        self.registered
            .get(action)
            .map(|h| {
                h.to_string()
                    .replace("control+", "Ctrl+")
                    .replace("alt+", "Alt+")
                    .replace("shift+", "Shift+")
                    .replace("super+", "Win+")
                    .replace("Key", "")
                    .replace("Digit", "")
            })
            .unwrap_or_else(|| "사용 안 함".into())
    }
    fn open_note(&mut self, i: usize) {
        if self.data.notes[i].locked {
            self.ask_unlock(i, false);
            return;
        }
        self.active = i;
        self.extra.edit_occurrence = self.data.notes[i]
            .occurrence_completed
            .keys()
            .next_back()
            .copied();
        self.editor = true;
        self.sticky = true;
        self.studio.open(i, self.extra.edit_occurrence);
        if let Some(w) = self.studio.windows.get_mut(&i) {
            w.schedule_start = self.data.notes[i].date;
        }
        self.data.notes[i].fold = 0;
        self.edit_error.clear();
        let n = &self.data.notes[i];
        self.date_edit = n.date.map(|d| d.to_string()).unwrap_or_default();
        self.time_edit = n.time.format("%H:%M").to_string();
        self.completion_edit = n
            .completed
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();
    }
    fn add_note(&mut self, date: Option<NaiveDate>) {
        let i = self.data.notes.len();
        self.data.notes.push(Note::new("새 할 일", date, i));
        self.open_note(i);
    }
    fn done(&mut self, i: usize, value: bool) {
        self.done_at(i, None, value);
    }
}
impl Drop for App {
    fn drop(&mut self) {
        if let Some(m) = &self.hotkeys {
            for h in &self.registered {
                let _ = m.unregister(*h);
            }
        }
    }
}
impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0., 0., 0., 0.]
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.theme(ctx);
        self.drag_input(ctx);
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        while let Ok(e) = GlobalHotKeyEvent::receiver().try_recv() {
            if e.state == HotKeyState::Pressed {
                if self.registered.first().is_some_and(|h| h.id() == e.id) {
                    self.capture = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                } else if self.registered.get(2).is_some_and(|h| h.id() == e.id) {
                    self.new_sticker();
                } else if self.registered.get(1).is_some_and(|h| h.id() == e.id) {
                    self.floating = true;
                    ctx.send_viewport_cmd_to(
                        egui::ViewportId::from_hash_of("calendar"),
                        egui::ViewportCommand::Minimized(false),
                    );
                    ctx.send_viewport_cmd_to(
                        egui::ViewportId::from_hash_of("calendar"),
                        egui::ViewportCommand::Focus,
                    );
                }
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.editor = false;
            self.capture = false;
            self.settings = false;
            self.extra.search_options = false;
            self.extra.project_manager = false;
            self.extra.picker = None;
            self.link_source = None;
        }
        if ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::H)
        }) {
            for w in self.studio.windows.values_mut() {
                w.open = false;
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::P)) {
            self.desktop.project_popup = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::T)) {
            let i = self.catalog.selected.unwrap_or(self.active);
            self.data.notes[i].topmost = !self.data.notes[i].topmost;
        }
        if self.sidebar {
            egui::SidePanel::left("sidebar")
                .exact_width(220.)
                .frame(
                    egui::Frame::NONE
                        .fill(if self.dark {
                            Color32::from_rgb(25, 26, 25)
                        } else {
                            Color32::from_rgb(249, 247, 244)
                        })
                        .inner_margin(16.),
                )
                .show(ctx, |ui| {
                    ui.add_space(12.);
                    ui.strong("M  MEMONOKING");
                    ui.label(RichText::new("나의 메모와 하루").small().color(MUTED));
                    ui.add_space(20.);
                    if ui
                        .add_sized(
                            [188., 34.],
                            egui::Button::new(RichText::new("+ 작업 추가").color(ACCENT)),
                        )
                        .clicked()
                    {
                        self.capture = true;
                    }
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.search)
                                .id_salt("global_search")
                                .hint_text("제목 · 본문 검색"),
                        )
                        .changed()
                    {
                        self.extra.search_mode = !self.search.is_empty();
                        if self.extra.search_mode {
                            self.page = 0;
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui.small_button("상세 옵션").clicked() {
                            self.extra.search_options = !self.extra.search_options;
                        }
                        if ui.small_button("검색 해제").clicked() {
                            self.search.clear();
                            self.extra.search_mode = false;
                        }
                    });
                    ui.add_space(12.);
                    egui::ScrollArea::vertical()
                        .max_height((ui.available_height() - 80.).max(100.))
                        .show(ui, |ui| {
                            for (i, name) in [
                                "내 작업실",
                                "나만의 위키",
                                "오늘",
                                "다음",
                                "전체메모",
                                "캘린더",
                                "편의기능",
                            ]
                            .iter()
                            .enumerate()
                            {
                                if studio::nav_button(
                                    ui,
                                    name,
                                    self.page == i,
                                    [
                                        icons::Icon::Sticky,
                                        icons::Icon::Folder,
                                        icons::Icon::Calendar,
                                        icons::Icon::ChevronRight,
                                        icons::Icon::Folder,
                                        icons::Icon::Calendar,
                                        icons::Icon::Settings,
                                    ][i],
                                )
                                .clicked()
                                {
                                    self.page = i;
                                }
                            }
                            ui.add_space(20.);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("프로젝트").small().color(MUTED));
                                if ui.small_button("관리").clicked() {
                                    self.extra.project_manager = true;
                                }
                            });
                            for (i, p) in self.data.projects.iter().enumerate() {
                                if self.extra.hidden_projects.contains(&i)
                                    || self.extra.deleted_projects.contains(&i)
                                {
                                    continue;
                                }
                                if ui
                                    .selectable_label(
                                        self.project_filter == Some(i),
                                        format!("# {p}"),
                                    )
                                    .clicked()
                                {
                                    self.project_filter = Some(i);
                                    self.page = 0;
                                }
                            }
                            ui.add_space(14.);
                            ui.label(
                                RichText::new("내 메모 · 날짜로 드래그")
                                    .small()
                                    .color(MUTED),
                            );
                            for i in 0..self.data.notes.len() {
                                if !self.visible(i)
                                    || (!self.search.is_empty()
                                        && !self.data.notes[i].title.contains(&self.search)
                                        && (self.data.notes[i].locked
                                            || !self.data.notes[i]
                                                .document
                                                .plain_text(&self.data.notes[i].body)
                                                .contains(&self.search)))
                                {
                                    continue;
                                }
                                let r = ui.add(
                                    egui::Label::new(shorten(&self.data.notes[i].title, 15))
                                        .sense(egui::Sense::click_and_drag()),
                                );
                                r.dnd_set_drag_payload(NoteDrag);
                                self.drag_source(ui, &r, i, false);
                                if r.clicked() {
                                    self.open_note(i);
                                }
                            }
                        });
                    ui.separator();
                    if ui.button("⚙ 설정 · 단축키").clicked() {
                        self.settings = true;
                    }
                    ui.label(
                        RichText::new("미리보기 · 종료하면 초기화")
                            .small()
                            .color(MUTED),
                    );
                });
        }
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.small_button("☰").clicked() {
                    self.sidebar = !self.sidebar;
                }
                ui.label(RichText::new("개인 작업 공간").small().color(MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::icon_button(ui, icons::Icon::Help, "작업 공간 도움말").clicked()
                    {
                        self.studio.help = !self.studio.help;
                    }
                    if ui
                        .button("스티커 노트")
                        .on_hover_text(format!("새 스티커 · {}", self.shortcut_label(2)))
                        .clicked()
                    {
                        self.new_sticker();
                    }
                    if ui.button("캘린더 분리 ↗").clicked() {
                        self.floating = true;
                    }
                    if ui.button("+ 빠른 메모").clicked() {
                        self.capture = true;
                    }
                });
            });
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(RichText::new(&self.toast).small().color(MUTED));
        });
        if self.split && self.page == 0 {
            egui::SidePanel::right("split")
                .default_width(550.)
                .min_width(360.)
                .resizable(true)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add_space(15.);
                        self.calendar(ui);
                    });
                });
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(20.);
            if self.page == 4 {
                self.catalog_ui(ui);
            } else {
                egui::ScrollArea::vertical()
                    .id_salt(("page_scroll", self.page))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(20.);
                            ui.vertical(|ui| {
                                ui.set_min_width((ui.available_width() - 25.).max(200.));
                                match self.page {
                                    1 => self.wiki(ui),
                                    0 | 2 | 3 => self.agenda(ui, self.page),
                                    5 => self.calendar(ui),
                                    6 => self.utilities(ui),
                                    _ => self.collections(ui),
                                }
                            });
                        });
                    });
            }
        });
        if self.capture {
            self.capture_project_header();
        }
        let mut open = self.capture;
        egui::Window::new("빠른 메모")
            .open(&mut open)
            .default_width(430.)
            .show(ctx, |ui| {
                ui.label("떠오른 생각이나 새 할 일을 적으세요");
                ui.add(
                    egui::TextEdit::multiline(&mut self.capture_text)
                        .desired_width(f32::INFINITY)
                        .desired_rows(5),
                );
                if ui.button("수집함에 추가").clicked() && !self.capture_text.trim().is_empty()
                {
                    let i = self.data.notes.len();
                    let mut n = Note::new(
                        self.capture_text.lines().next().unwrap_or("새 메모"),
                        None,
                        i,
                    );
                    n.body = self.capture_text.clone();
                    n.project = self.extra.capture_project.take();
                    self.data.notes.push(n);
                    self.consume_project_header(i);
                    self.capture_text.clear();
                    self.toast = "수집함 보드에 추가했습니다".into();
                }
            });
        self.capture = open;
        let mut open = self.settings;
        egui::Window::new("설정")
            .open(&mut open)
            .default_width(780.)
            .show(ctx, |ui| self.ppt_settings(ui));
        self.settings = open;
        if self.studio.help {
            let mut open = true;
            egui::Window::new("작업 공간 도움말")
                .open(&mut open)
                .default_width(380.)
                .show(ctx, |ui| {
                    ui.heading("내 메모, 내 방식으로");
                    ui.label("메모 제목을 눌러 독립 편집창을 여세요.");
                    ui.label("스티커 노트는 제목과 본문만 간결하게 보여줍니다.");
                    ui.label("메모 제목을 달력 날짜에 끌면 일정이 지정됩니다.");
                    ui.label("수집함에서 관계를 연결하고 위키에서 지식을 분류하세요.");
                    ui.separator();
                    for (i, name) in ["빠른 메모", "분리 캘린더", "새 스티커"].iter().enumerate()
                    {
                        ui.label(format!("{name} · {}", self.shortcut_label(i)));
                    }
                    ui.label("단축키는 설정에서 바꿀 수 있습니다.");
                });
            self.studio.help = open;
        }
        if self.floating {
            let mut open = true;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("calendar"),
                egui::ViewportBuilder::default()
                    .with_title("MEMONOKING · 캘린더")
                    .with_inner_size([820., 760.])
                    .with_min_inner_size([480., 420.]),
                |ctx, _| {
                    self.theme(ctx);
                    self.drag_input(ctx);
                    if ctx.input(|i| i.viewport().close_requested()) {
                        open = false;
                    }
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| self.calendar(ui));
                    });
                },
            );
            self.floating = open;
        }
        self.detached_note(ctx);
        self.extra_windows(ctx);
        self.desktop_dialogs(ctx);
    }
}
fn shorten(s: &str, n: usize) -> String {
    let s = s.replace('\n', " ");
    if s.chars().count() > n {
        format!(
            "{}…",
            s.chars().take(n.saturating_sub(1)).collect::<String>()
        )
    } else {
        s
    }
}
fn main() -> eframe::Result {
    eframe::run_native(
        "MEMONOKING",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1440., 900.])
                .with_maximized(true)
                .with_clamp_size_to_monitor_size(true)
                .with_min_inner_size([1100., 720.]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
