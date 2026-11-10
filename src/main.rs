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
mod repeat;
mod schedule;
mod studio;
mod ui;
mod views;
mod win_style;
use chrono::{Datelike, Local, NaiveDate};
use eframe::egui::{self, Color32, RichText, Vec2};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use model::{Note, Workspace};
const PALETTE: [Color32; 10] = [
    Color32::from_rgb(255, 240, 166),
    Color32::from_rgb(255, 216, 186),
    Color32::from_rgb(255, 207, 214),
    Color32::from_rgb(226, 214, 250),
    Color32::from_rgb(203, 226, 250),
    Color32::from_rgb(204, 236, 214),
    Color32::from_rgb(224, 232, 196),
    Color32::from_rgb(240, 228, 206),
    Color32::from_rgb(228, 228, 226),
    Color32::from_rgb(250, 250, 248),
];
const NO_PROJECT: usize = usize::MAX;
const INK: Color32 = Color32::from_rgb(44, 43, 40);
const MUTED: Color32 = Color32::from_rgb(120, 119, 115);
const ACCENT: Color32 = ui::ACCENT;
pub use ui::main_viewport;
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
    toast_shown: (String, f64),
    arrange: Option<studio::Arrange>,
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
            toast_shown: (String::new(), 0.),
            arrange: None,
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
        let selected = egui::FontFamily::Name(self.desktop.font.clone().into());
        let family = if ctx.fonts(|f| f.families().contains(&selected)) {
            selected
        } else {
            egui::FontFamily::Proportional
        };
        ui::apply(ctx, self.dark, family, self.desktop.font_size * 4. / 3.);
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
            if self.desktop.always_sticker {
                w.sticker();
            }
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
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        while let Ok(e) = GlobalHotKeyEvent::receiver().try_recv() {
            if e.state == HotKeyState::Pressed {
                if self.registered.first().is_some_and(|h| h.id() == e.id) {
                    self.capture = true;
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
        // The host itself paints nothing.
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |_| {});
        self.main_window(ctx);
        if self.floating {
            let mut open = true;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("calendar"),
                egui::ViewportBuilder::default()
                    .with_title("MEMONOKING · 캘린더")
                    .with_inner_size([860., 760.])
                    .with_min_inner_size([480., 420.]),
                |ctx, _| {
                    self.theme(ctx);
                    self.drag_input(ctx);
                    if ctx.input(|i| i.viewport().close_requested()) {
                        open = false;
                    }
                    egui::CentralPanel::default()
                        .frame(
                            egui::Frame::NONE
                                .fill(ctx.style().visuals.panel_fill)
                                .inner_margin(egui::Margin::symmetric(20, 16)),
                        )
                        .show(ctx, |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| self.calendar(ui));
                        });
                },
            );
            self.floating = open;
        }
        self.apply_arrangement(ctx);
        self.detached_note(ctx);
        self.global_dialogs(ctx);
        self.extra_windows(ctx);
        self.desktop_dialogs(ctx);
    }
}
impl App {
    fn main_window(&mut self, root: &egui::Context) {
        let builder = egui::ViewportBuilder::default()
            .with_title("MEMONOKING")
            .with_inner_size([1440., 900.])
            .with_min_inner_size([980., 640.])
            .with_maximized(true);
        root.show_viewport_immediate(main_viewport(), builder, |ctx, _| {
            self.theme(ctx);
            self.drag_input(ctx);
            if ctx.input(|i| i.viewport().close_requested()) {
                root.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
            }
            self.main_shortcuts(ctx);
            self.main_ui(ctx);
            self.picker_window(ctx);
            self.scope_dialog(ctx);
            self.toast_overlay(ctx);
        });
    }
    fn main_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
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
    }
    fn main_ui(&mut self, ctx: &egui::Context) {
        let tone = ui::tone(self.dark);
        if self.sidebar {
            egui::SidePanel::left("sidebar")
                .exact_width(236.)
                .resizable(false)
                .show_separator_line(false)
                .frame(
                    egui::Frame::NONE
                        .fill(tone.sidebar)
                        .inner_margin(egui::Margin {
                            left: 12,
                            right: 12,
                            top: 14,
                            bottom: 12,
                        }),
                )
                .show(ctx, |ui| self.sidebar_ui(ui));
        }
        if self.split && self.page == 0 {
            egui::SidePanel::right("split")
                .default_width(560.)
                .min_width(380.)
                .max_width(ctx.screen_rect().width() * 0.62)
                .resizable(true)
                .show_separator_line(true)
                .frame(egui::Frame::NONE.fill(tone.bg).inner_margin(egui::Margin {
                    left: 20,
                    right: 20,
                    top: 18,
                    bottom: 8,
                }))
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("split_calendar")
                        .show(ui, |ui| self.calendar(ui));
                });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(tone.bg).inner_margin(egui::Margin {
                left: 28,
                right: 24,
                top: 18,
                bottom: 8,
            }))
            .show(ctx, |ui| {
                self.page_bar(ui);
                ui.add_space(10.);
                if self.page == 4 || (self.page == 0 && !self.split) {
                    if self.page == 4 {
                        self.catalog_ui(ui);
                    } else {
                        self.agenda(ui, 0);
                    }
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt(("page_scroll", self.page))
                        .show(ui, |ui| {
                            ui.set_min_width((ui.available_width() - 8.).max(200.));
                            match self.page {
                                1 => self.wiki(ui),
                                0 | 2 | 3 => self.agenda(ui, self.page),
                                5 => self.calendar(ui),
                                6 => self.utilities(ui),
                                _ => self.collections(ui),
                            }
                        });
                }
            });
    }
    /// Page title on the left and global window actions on the right.
    fn page_bar(&mut self, ui: &mut egui::Ui) {
        let title = if self.extra.search_mode && matches!(self.page, 0 | 2 | 3) {
            "검색 결과"
        } else {
            PAGES
                .iter()
                .find(|(p, _, _)| *p == self.page)
                .map(|(_, name, _)| *name)
                .unwrap_or("MEMONOKING")
        };
        ui.horizontal(|ui| {
            ui.set_min_height(34.);
            if !self.sidebar
                && icons::icon_button(ui, icons::Icon::Sidebar, "사이드바 열기").clicked()
            {
                self.sidebar = true;
            }
            ui.label(RichText::new(title).size(20.).strong());
            if self.extra.search_mode {
                ui.add_space(6.);
                if ui::pill(
                    ui,
                    true,
                    &format!("‘{}’ 검색 해제", shorten(&self.search, 12)),
                )
                .clicked()
                {
                    self.search.clear();
                    self.extra.search_mode = false;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::icon_button(ui, icons::Icon::Detach, "캘린더 분리창").clicked() {
                    self.floating = true;
                }
                if icons::icon_button(
                    ui,
                    icons::Icon::StickyPlus,
                    &format!("새 스티커 · {}", self.shortcut_label(2)),
                )
                .clicked()
                {
                    self.new_sticker();
                }
                if self.page == 0 {
                    let label = if self.split {
                        "캘린더 닫기 · 자세히 보기"
                    } else {
                        "캘린더 함께 보기"
                    };
                    if icons::toggle_icon(ui, icons::Icon::Calendar, label, self.split).clicked() {
                        self.split = !self.split;
                    }
                }
            });
        });
    }
    fn sidebar_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let (mark, _) = ui.allocate_exact_size(Vec2::splat(22.), egui::Sense::hover());
            ui.painter().rect_filled(mark, 6., ACCENT);
            ui.painter().text(
                mark.center(),
                egui::Align2::CENTER_CENTER,
                "M",
                egui::FontId::proportional(13.),
                Color32::WHITE,
            );
            ui.label(RichText::new("MEMONOKING").strong().size(14.));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::icon_button(ui, icons::Icon::Sidebar, "사이드바 접기").clicked() {
                    self.sidebar = false;
                }
            });
        });
        ui.add_space(12.);
        ui.horizontal(|ui| {
            let width = ui.available_width() - 34.;
            let r = ui::search_field(ui, &mut self.search, "global_search", "검색", width);
            if r.changed() {
                self.extra.search_mode = !self.search.is_empty();
                if self.extra.search_mode && !matches!(self.page, 0 | 2 | 3) {
                    self.page = 0;
                }
            }
            if icons::toggle_icon(
                ui,
                icons::Icon::Filter,
                "상세 검색",
                self.extra.search_options,
            )
            .clicked()
            {
                self.extra.search_options = !self.extra.search_options;
            }
        });
        ui.add_space(8.);
        if studio::nav_button(ui, "빠른 메모", false, icons::Icon::PlusCircle, None, true).clicked()
        {
            self.capture = true;
        }
        ui.add_space(6.);
        let today = Local::now().date_naive();
        let today_count = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(i, n)| self.visible(*i) && n.occurs(today) && !n.done_on(Some(today)))
            .count();
        for (page, name, icon) in PAGES {
            let badge = (page == 2 && today_count > 0).then(|| today_count.to_string());
            if studio::nav_button(ui, name, self.page == page, icon, badge, false).clicked() {
                self.page = page;
            }
        }
        ui.add_space(18.);
        ui.horizontal(|ui| {
            ui.add_space(8.);
            ui::caption(ui, "프로젝트");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::small_icon(ui, icons::Icon::Settings, "프로젝트 관리").clicked() {
                    self.extra.project_manager = true;
                }
            });
        });
        ui.add_space(2.);
        egui::ScrollArea::vertical()
            .id_salt("sidebar_projects")
            .max_height((ui.available_height() - 44.).max(60.))
            .show(ui, |ui| {
                let mut any = false;
                for (i, p) in self.data.projects.iter().enumerate() {
                    if self.extra.hidden_projects.contains(&i)
                        || self.extra.deleted_projects.contains(&i)
                    {
                        continue;
                    }
                    any = true;
                    let selected = self.project_filter == Some(i);
                    if studio::project_button(ui, p, selected).clicked() {
                        self.project_filter = if selected { None } else { Some(i) };
                        if !matches!(self.page, 0 | 2 | 3) {
                            self.page = 0;
                        }
                    }
                }
                if !any {
                    ui.horizontal(|ui| {
                        ui.add_space(8.);
                        ui::hint(ui, "본문 첫 줄에 #이름 을 입력하면 생성됩니다");
                    });
                }
            });
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            if studio::nav_button(ui, "설정", false, icons::Icon::Settings, None, false).clicked()
            {
                self.settings = true;
                ui::raise(ui.ctx(), "settings");
            }
        });
    }
    fn toast_overlay(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        if self.toast.is_empty() {
            return;
        }
        if self.toast_shown.0 != self.toast {
            self.toast_shown = (self.toast.clone(), now);
        }
        let age = now - self.toast_shown.1;
        if age > 3.2 {
            self.toast.clear();
            return;
        }
        let alpha = ((3.2 - age) / 0.4).clamp(0., 1.) as f32;
        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("toast"))
            .order(egui::Order::Tooltip)
            .fixed_pos(egui::pos2(screen.center().x, screen.bottom() - 28.))
            .pivot(egui::Align2::CENTER_BOTTOM)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(
                        40,
                        39,
                        37,
                        (235. * alpha) as u8,
                    ))
                    .corner_radius(8.)
                    .inner_margin(egui::Margin::symmetric(14, 8))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(&self.toast)
                                .color(Color32::from_white_alpha((255. * alpha) as u8)),
                        );
                    });
            });
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
    /// Dialogs that belong to the application rather than to one window.
    fn global_dialogs(&mut self, ctx: &egui::Context) {
        if self.capture {
            self.capture_project_header();
        }
        let mut open = self.capture;
        ui::dialog(
            ctx,
            "capture",
            "빠른 메모",
            [460., 300.],
            true,
            &mut open,
            |ui| {
                ui.label(RichText::new("빠른 메모").size(16.).strong());
                ui::hint(
                    ui,
                    "첫 줄은 제목이 됩니다 · #프로젝트명 으로 시작하면 분류됩니다",
                );
                ui.add_space(8.);
                let r = ui.add(
                    egui::TextEdit::multiline(&mut self.capture_text)
                        .id_salt("capture_text")
                        .desired_width(f32::INFINITY)
                        .desired_rows(6)
                        .margin(egui::Margin::same(10))
                        .hint_text("떠오른 생각이나 할 일을 적으세요"),
                );
                if !r.has_focus() && self.capture_text.is_empty() {
                    r.request_focus();
                }
                let submit = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter));
                let (save, _) = ui::dialog_buttons(ui, "추가  Ctrl+Enter", "");
                if (save || submit) && !self.capture_text.trim().is_empty() {
                    let i = self.data.notes.len();
                    let mut n = Note::new(
                        self.capture_text.lines().next().unwrap_or("새 메모"),
                        None,
                        i,
                    );
                    n.body = self
                        .capture_text
                        .split_once('\n')
                        .map(|(_, rest)| rest.to_owned())
                        .unwrap_or_default();
                    n.project = self.extra.capture_project.take();
                    self.data.notes.push(n);
                    self.consume_project_header(i);
                    self.capture_text.clear();
                    self.toast = "메모를 추가했습니다".into();
                }
            },
        );
        self.capture = open;
        let mut open = self.settings;
        ui::dialog(
            ctx,
            "settings",
            "설정",
            [860., 620.],
            false,
            &mut open,
            |ui| self.ppt_settings(ui),
        );
        self.settings = open;
        let mut open = self.studio.help;
        ui::dialog(
            ctx,
            "help",
            "도움말",
            [420., 380.],
            false,
            &mut open,
            |ui| {
                ui.label(RichText::new("MEMONOKING 사용법").size(16.).strong());
                ui.add_space(8.);
                for line in [
                    "메모 제목을 누르면 편집창이 열립니다.",
                    "메모를 달력 날짜로 끌면 일정이 지정됩니다.",
                    "스티커를 우클릭하면 바탕화면 정렬 메뉴가 나옵니다.",
                    "접기: 스티커 → 제목 한 줄 → 색상 상자 · 상자를 두 번 클릭하면 복원",
                    "이미지와 표는 클립보드에서 본문에 붙여 넣습니다.",
                ] {
                    ui.label(format!("·  {line}"));
                }
                ui.add_space(10.);
                ui::divider(ui);
                ui.add_space(10.);
                for (i, name) in ["빠른 메모", "분리 캘린더", "새 스티커"].iter().enumerate()
                {
                    ui::field(ui, name, |ui| {
                        ui.monospace(self.shortcut_label(i));
                    });
                }
            },
        );
        self.studio.help = open;
    }
}
/// Navigation pages: (page index, label, icon) in display order.
const PAGES: [(usize, &str, icons::Icon); 7] = [
    (0, "내 작업실", icons::Icon::Home),
    (2, "오늘", icons::Icon::Today),
    (3, "다음", icons::Icon::Upcoming),
    (5, "캘린더", icons::Icon::Calendar),
    (4, "전체메모", icons::Icon::Notes),
    (1, "나만의 위키", icons::Icon::Book),
    (6, "편의기능", icons::Icon::Grid),
];
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
    // The root window is an invisible, never-minimized host. Every visible
    // window (main, calendar, stickers, dialogs) is an independent child, so
    // minimizing the main window leaves stickers alive on the desktop and the
    // shared GL configuration supports real per-window transparency.
    eframe::run_native(
        "MEMONOKING",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("MEMONOKING")
                .with_inner_size([1., 1.])
                .with_position([0., 0.])
                .with_decorations(false)
                .with_resizable(false)
                .with_transparent(true)
                .with_taskbar(false),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
