#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod dragging;
mod model;
mod views;
use chrono::{Datelike, Local, NaiveDate};
use eframe::egui::{self, Color32, RichText, Vec2};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use model::{Note, Workspace};
const INK: Color32 = Color32::from_rgb(48, 47, 44);
const MUTED: Color32 = Color32::from_rgb(133, 130, 124);
const ACCENT: Color32 = Color32::from_rgb(184, 72, 53);
const LABELS: [&str; 3] = ["개인", "프로젝트", "회의"];
const COLORS: [Color32; 3] = [
    Color32::from_rgb(117, 150, 125),
    Color32::from_rgb(133, 121, 177),
    Color32::from_rgb(201, 151, 84),
];
#[derive(Clone)]
struct NoteDrag(usize);
struct App {
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
    label_filter: usize,
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
    shortcuts: [String; 2],
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
        cc.egui_ctx.set_fonts(fonts);
        let mut a = Self::sample(Local::now().date_naive());
        a.hotkeys = GlobalHotKeyManager::new().ok();
        a.apply_shortcuts();
        a
    }
    fn sample(today: NaiveDate) -> Self {
        Self {
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
            label_filter: 0,
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
            shortcuts: ["Ctrl+Alt+N".into(), "Ctrl+Alt+C".into()],
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
        ctx.set_visuals(v);
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
        if new[0].id() == new[1].id() {
            self.hotkey_status = "두 동작에 서로 다른 단축키를 지정하세요".into();
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
    fn open_note(&mut self, i: usize) {
        self.active = i;
        self.editor = true;
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
        self.data.complete(i, value, Local::now());
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
            self.link_source = None;
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
                    ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("검색"));
                    ui.add_space(12.);
                    egui::ScrollArea::vertical()
                        .max_height((ui.available_height() - 80.).max(100.))
                        .show(ui, |ui| {
                            for (i, name) in [
                                "내 작업실",
                                "관리함",
                                "오늘",
                                "다음",
                                "수집함 · 보드",
                                "캘린더",
                                "업무일지",
                                "표 모아보기",
                                "이미지 모아보기",
                                "링크 · 즐겨찾기",
                            ]
                            .iter()
                            .enumerate()
                            {
                                if ui
                                    .add_sized(
                                        [188., 34.],
                                        egui::Button::new(*name)
                                            .selected(self.page == i)
                                            .frame(self.page == i),
                                    )
                                    .clicked()
                                {
                                    self.page = i;
                                }
                            }
                            ui.add_space(20.);
                            ui.label(RichText::new("프로젝트").small().color(MUTED));
                            for (i, p) in self.data.projects.iter().enumerate() {
                                if ui
                                    .selectable_label(
                                        self.project_filter == Some(i),
                                        format!("# {p}"),
                                    )
                                    .clicked()
                                {
                                    self.project_filter = Some(i);
                                    self.page = 1;
                                }
                            }
                            ui.add_space(14.);
                            ui.label(
                                RichText::new("내 메모 · 날짜로 드래그")
                                    .small()
                                    .color(MUTED),
                            );
                            for i in 0..self.data.notes.len() {
                                if !self.search.is_empty()
                                    && !self.data.notes[i].title.contains(&self.search)
                                {
                                    continue;
                                }
                                let r = ui.add(
                                    egui::Label::new(shorten(&self.data.notes[i].title, 15))
                                        .sense(egui::Sense::click_and_drag()),
                                );
                                r.dnd_set_drag_payload(NoteDrag(i));
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
                self.board(ui);
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(20.);
                        ui.vertical(|ui| {
                            ui.set_min_width((ui.available_width() - 25.).max(200.));
                            match self.page {
                                0..=3 => self.agenda(ui, self.page),
                                5 => self.calendar(ui),
                                6 => self.journal(ui),
                                _ => self.collections(ui),
                            }
                        });
                    });
                });
            }
        });
        let mut open = self.editor;
        egui::Window::new("메모 편집")
            .open(&mut open)
            .default_size([600., 540.])
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.note_editor(ui));
            });
        self.editor = open;
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
                    self.data.notes.push(n);
                    self.capture_text.clear();
                    self.toast = "수집함 보드에 추가했습니다".into();
                }
            });
        self.capture = open;
        let mut open = self.settings;
        egui::Window::new("설정")
            .open(&mut open)
            .default_width(520.)
            .show(ctx, |ui| self.settings_ui(ui));
        self.settings = open;
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
        if self.sticky {
            let mut open = true;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("sticky"),
                egui::ViewportBuilder::default()
                    .with_title("MEMONOKING · 분리 메모")
                    .with_inner_size([540., 650.])
                    .with_min_inner_size([360., 320.]),
                |ctx, _| {
                    self.theme(ctx);
                    self.drag_input(ctx);
                    if ctx.input(|i| i.viewport().close_requested()) {
                        open = false;
                    }
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| self.note_editor(ui));
                    });
                },
            );
            self.sticky = open;
        }
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
                .with_inner_size([1180., 760.])
                .with_min_inner_size([960., 640.]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
