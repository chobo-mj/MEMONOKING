#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui::{self, Color32, RichText, Vec2};

const INK: Color32 = Color32::from_rgb(48, 47, 44);
const MUTED: Color32 = Color32::from_rgb(145, 143, 138);
const PAPER: Color32 = Color32::from_rgb(255, 254, 252);
const SIDE: Color32 = Color32::from_rgb(247, 247, 245);

struct Note {
    title: String,
    body: String,
    done: bool,
    checks: [bool; 3],
}
struct App {
    page: usize,
    selected: usize,
    notes: Vec<Note>,
    search: String,
    sidebar: bool,
    split: bool,
    capture: bool,
    capture_text: String,
    sticky: bool,
    calendar_window: bool,
    settings: bool,
    dark: bool,
    month: i32,
    day: usize,
    label: usize,
    toast: String,
    view: usize,
}
impl App {
    fn visuals(dark: bool) -> egui::Visuals {
        let mut v = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        v.panel_fill = if dark {
            Color32::from_rgb(30, 31, 30)
        } else {
            PAPER
        };
        v.override_text_color = Some(if dark {
            Color32::from_rgb(232, 232, 227)
        } else {
            INK
        });
        v.selection.bg_fill = if dark {
            Color32::from_rgb(59, 75, 65)
        } else {
            Color32::from_rgb(231, 233, 227)
        };
        v.selection.stroke =
            egui::Stroke::new(1.0_f32, if dark { Color32::LIGHT_GRAY } else { INK });
        v.widgets.inactive.bg_fill = if dark {
            Color32::from_rgb(43, 44, 42)
        } else {
            SIDE
        };
        v.widgets.hovered.bg_fill = if dark {
            Color32::from_rgb(62, 64, 59)
        } else {
            Color32::from_rgb(237, 238, 233)
        };
        v.widgets.active.bg_fill = v.selection.bg_fill;
        v.widgets.active.weak_bg_fill = v.selection.bg_fill;
        v.widgets.hovered.weak_bg_fill = v.widgets.hovered.bg_fill;
        v
    }
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        for path in ["C:/Windows/Fonts/malgun.ttf", "C:/Windows/Fonts/arial.ttf"] {
            if let Ok(bytes) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("local-ui".into(), egui::FontData::from_owned(bytes).into());
                fonts
                    .families
                    .get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "local-ui".into());
                break;
            }
        }
        cc.egui_ctx.set_fonts(fonts);
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(10., 10.);
        style.spacing.button_padding = Vec2::new(10., 7.);
        style.spacing.interact_size.y = 28.;
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(16.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(15.));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(12.));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(24.));
        style.visuals = Self::visuals(false);
        cc.egui_ctx.set_style(style);
        Self { page:0, selected:0, notes:vec![
            Note{title:"작은 기록이 큰 흐름이 되도록".into(),body:"이번 주의 생각과 할 일을 한곳에서 정리하세요.\n\n메모를 쓰고, 일정을 살펴보고, 작은 일부터 하나씩.\n이 공간은 나의 속도로 일하기 위한 작업실입니다.\n\n오늘의 메모\n  1. 중요한 일 한 가지에 집중하기\n    a) 필요한 자료 먼저 준비하기\n  2. 여유를 두고 일정 정리하기".into(),done:false,checks:[true,false,false]},
            Note{title:"11월 프로젝트 노트".into(),body:"프로젝트의 목표와 아이디어를 적어보세요.\n\n목표 → 초안 → 검토 → 완성".into(),done:false,checks:[false;3]},
            Note{title:"주간 회의 준비".into(),body:"논의할 내용\n\n• 지난주 진행 상황\n• 이번 주 우선순위\n• 함께 결정할 사항".into(),done:false,checks:[false;3]},
        ],search:String::new(),sidebar:true,split:false,capture:false,capture_text:String::new(),sticky:false,calendar_window:false,settings:false,dark:false,month:11,day:8,label:0,toast:String::new(),view:0 }
    }
    fn calendar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(format!("2026년 {}월", self.month));
            if ui.small_button("〈").clicked() {
                self.month = if self.month == 1 { 12 } else { self.month - 1 };
            }
            if ui.small_button("〉").clicked() {
                self.month = if self.month == 12 { 1 } else { self.month + 1 };
            }
            if ui.small_button("오늘").clicked() {
                self.month = 11;
                self.day = 8;
            }
        });
        ui.horizontal(|ui| {
            for (i, s) in ["월", "주", "일"].iter().enumerate() {
                ui.selectable_value(&mut self.view, i, *s);
            }
        });
        ui.add_space(12.);
        let days = match self.month {
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        self.day = self.day.min(days as usize);
        let cell_width = ((ui.available_width() - 36.) / 7.).clamp(32., 110.);
        let names = ["일", "월", "화", "수", "목", "금", "토"];
        egui::Grid::new(ui.id().with("calendar"))
            .num_columns(7)
            .spacing([6., 6.])
            .show(ui, |ui| {
                for name in names {
                    ui.add_sized(
                        [cell_width, 24.],
                        egui::Label::new(RichText::new(name).color(MUTED)),
                    );
                }
                ui.end_row();
                let first = (0..self.month - 1)
                    .map(|m| [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][m as usize])
                    .sum::<i32>();
                let offset = (4 + first) % 7;
                for slot in 0..((days + offset + 6) / 7) * 7 {
                    let day = slot - offset + 1;
                    if day < 1 || day > days {
                        ui.label("");
                    } else {
                        let selected = self.day == day as usize;
                        let label = if day == 8 || day == 16 || day == 23 {
                            format!("{}\n● 메모", day)
                        } else {
                            format!("{}\n", day)
                        };
                        if ui
                            .add_sized(
                                [cell_width, if self.view == 0 { 66. } else { 90. }],
                                egui::Button::new(label).selected(selected),
                            )
                            .clicked()
                        {
                            self.day = day as usize;
                            self.toast =
                                format!("{}월 {}일 선택 · 일정 편집 미리보기", self.month, day);
                        }
                    }
                    if slot % 7 == 6 {
                        ui.end_row();
                    }
                }
            });
        ui.separator();
        ui.label(format!("{}월 {}일", self.month, self.day));
        if ui.button("+ 이 날짜에 메모 추가").clicked() {
            self.capture = true;
        }
        ui.label(
            RichText::new("일정은 화면 시연용입니다")
                .small()
                .color(MUTED),
        );
    }
    fn editor(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_width();
        let width = (available - 40.).clamp(240., 860.).min(available);
        ui.horizontal(|ui| {
            ui.add_space(((available - width) / 2.).max(0.));
            ui.allocate_ui_with_layout(
                Vec2::new(width, 0.),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    self.editor_contents(ui);
                },
            );
        });
    }
    fn editor_contents(&mut self, ui: &mut egui::Ui) {
        let compact = ui.available_width() < 600.;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("개인 공간  /  메모  /  오늘의 기록")
                    .small()
                    .color(MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("↗ 분리").clicked() {
                    self.sticky = true;
                }
                if ui.small_button("◫ 나란히").clicked() {
                    self.split = !self.split;
                }
            });
        });
        ui.add_space(if compact { 12. } else { 24. });
        egui::Frame::NONE
            .fill(if self.dark {
                Color32::from_rgb(43, 57, 47)
            } else {
                Color32::from_rgb(232, 237, 226)
            })
            .inner_margin(20.)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(
                    RichText::new("MAKE ROOM FOR YOUR IDEAS")
                        .small()
                        .color(if self.dark {
                            Color32::from_rgb(171, 191, 160)
                        } else {
                            Color32::from_rgb(91, 111, 83)
                        }),
                );
                ui.label(RichText::new("기록하고, 연결하고, 가볍게 시작하세요.").size(20.));
            });
        ui.add_space(22.);
        ui.label(RichText::new("▤").size(36.));
        let n = &mut self.notes[self.selected];
        ui.add(
            egui::TextEdit::multiline(&mut n.title)
                .font(egui::FontId::proportional(if compact { 26. } else { 32. }))
                .desired_rows(1)
                .frame(false)
                .desired_width(f32::INFINITY),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("상태").color(MUTED));
            ui.checkbox(&mut n.done, "완료");
            egui::ComboBox::from_id_salt("label")
                .selected_text(["개인", "프로젝트", "회의"][self.label])
                .show_ui(ui, |ui| {
                    for (i, s) in ["개인", "프로젝트", "회의"].iter().enumerate() {
                        ui.selectable_value(&mut self.label, i, *s);
                    }
                });
            ui.label(RichText::new("2026. 11. 08").small().color(MUTED));
        });
        ui.separator();
        ui.add(
            egui::TextEdit::multiline(&mut n.body)
                .frame(false)
                .desired_width(f32::INFINITY)
                .desired_rows(13),
        );
        ui.add_space(8.);
        ui.label(RichText::new("오늘의 작은 약속").strong());
        for (i, text) in [
            "이번 주 우선순위 정리",
            "프로젝트 초안 검토",
            "내일을 위한 메모 남기기",
        ]
        .iter()
        .enumerate()
        {
            ui.checkbox(&mut n.checks[i], *text);
        }
        ui.add_space(16.);
        egui::CollapsingHeader::new("하위 메모 · 2")
            .default_open(true)
            .show(ui, |ui| {
                ui.label("  ▤ 참고 자료 정리");
                ui.label("  ▤ 다음 단계 아이디어");
                if ui.small_button("+ 하위 메모").clicked() {
                    self.toast = "하위 메모 추가 동작 미리보기".into();
                }
            });
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.set_visuals(Self::visuals(self.dark));
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.capture = false;
            self.settings = false;
        }
        if self.sidebar {
            egui::SidePanel::left("navigation")
                .default_width(232.)
                .resizable(true)
                .frame(
                    egui::Frame::NONE
                        .fill(if self.dark {
                            Color32::from_rgb(25, 26, 25)
                        } else {
                            SIDE
                        })
                        .inner_margin(16.),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("sidebar_scroll")
                        .max_height((ui.available_height() - 106.).max(120.))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("M").size(24.).strong());
                                ui.vertical(|ui| {
                                    ui.strong("MEMONOKING");
                                    ui.label(
                                        RichText::new("나만의 작업 공간").small().color(MUTED),
                                    );
                                });
                            });
                            ui.add_space(24.);
                            ui.add(
                                egui::TextEdit::singleline(&mut self.search)
                                    .hint_text("검색...")
                                    .desired_width(f32::INFINITY),
                            );
                            ui.add_space(12.);
                            for (i, name) in [
                                "⌂  내 작업실",
                                "▣  수집함",
                                "▦  캘린더",
                                "▤  표 모아보기",
                                "▧  이미지 모아보기",
                                "↗  링크 · 즐겨찾기",
                                "☷  업무일지",
                            ]
                            .iter()
                            .enumerate()
                            {
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 33.],
                                        egui::Button::new(*name)
                                            .selected(self.page == i)
                                            .frame(self.page == i),
                                    )
                                    .clicked()
                                {
                                    self.page = i;
                                }
                            }
                            ui.add_space(22.);
                            ui.label(RichText::new("내 메모").small().color(MUTED));
                            for i in 0..self.notes.len() {
                                if !self.search.is_empty()
                                    && !self.notes[i].title.contains(&self.search)
                                {
                                    continue;
                                }
                                if ui
                                    .selectable_label(
                                        self.selected == i && self.page == 0,
                                        format!("▤  {}", self.notes[i].title),
                                    )
                                    .clicked()
                                {
                                    self.selected = i;
                                    self.page = 0;
                                }
                            }
                            if ui.button("+ 새 메모").clicked() {
                                self.notes.push(Note {
                                    title: "제목 없는 메모".into(),
                                    body: String::new(),
                                    done: false,
                                    checks: [false; 3],
                                });
                                self.selected = self.notes.len() - 1;
                                self.page = 0;
                            }
                            ui.add_space(25.);
                            ui.label(RichText::new("라벨").small().color(MUTED));
                            for (i, s) in ["●  개인", "●  프로젝트", "●  회의"].iter().enumerate()
                            {
                                if ui.selectable_label(self.label == i, *s).clicked() {
                                    self.label = i;
                                }
                            }
                        });
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        if ui.button("⚙ 설정").clicked() {
                            self.settings = true;
                        }
                        if ui.button("+ 빠른 메모").clicked() {
                            self.capture = true;
                        }
                        ui.label(
                            RichText::new("UI PREVIEW · 저장되지 않음")
                                .small()
                                .color(MUTED),
                        );
                    });
                });
        }
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.small_button("☰").clicked() {
                    self.sidebar = !self.sidebar;
                }
                ui.label(RichText::new("개인 공간").color(MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("+ 빠른 메모").clicked() {
                        self.capture = true;
                    }
                    if ui.small_button("캘린더 분리 ↗").clicked() {
                        self.calendar_window = true;
                    }
                });
            });
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("● 로컬 디자인 미리보기").small().color(MUTED));
                ui.label(RichText::new(&self.toast).small());
            });
        });
        if self.split && self.page == 0 {
            egui::SidePanel::right("split_calendar")
                .default_width(440.)
                .resizable(true)
                .show(ctx, |ui| {
                    self.calendar(ui);
                });
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(16.);
                match self.page {
                    0 => self.editor(ui),
                    2 => self.calendar(ui),
                    1 => {
                        ui.heading("수집함");
                        ui.label("떠오르는 생각을 먼저 모아두세요.");
                        ui.add_space(20.);
                        for n in &self.notes {
                            ui.group(|ui| {
                                ui.label(format!("▤  {}", n.title));
                                ui.label(RichText::new("미정리 · 오늘").small().color(MUTED));
                            });
                        }
                        if ui.button("+ 빠르게 기록하기").clicked() {
                            self.capture = true;
                        }
                    }
                    3 => {
                        ui.heading("표 모아보기");
                        ui.label("메모 속 정보를 한눈에.");
                        ui.add_space(20.);
                        egui::Grid::new("sample_table")
                            .striped(true)
                            .min_col_width(160.)
                            .show(ui, |ui| {
                                for row in [
                                    ["작업", "상태", "일정"],
                                    ["디자인 방향 정리", "진행 중", "11월 8일"],
                                    ["화면 흐름 검토", "회신 대기", "11월 10일"],
                                    ["프로토타입 확인", "할 일", "11월 12일"],
                                ] {
                                    for cell in row {
                                        ui.label(cell);
                                    }
                                    ui.end_row();
                                }
                            });
                    }
                    4 => {
                        ui.heading("이미지 모아보기");
                        ui.label("아이디어를 위한 시각적 공간");
                        ui.add_space(20.);
                        ui.horizontal_wrapped(|ui| {
                            for (c, s) in [
                                (Color32::from_rgb(222, 230, 215), "FOREST / 영감"),
                                (Color32::from_rgb(234, 223, 208), "PAPER / 기록"),
                                (Color32::from_rgb(218, 225, 233), "SKY / 생각"),
                            ] {
                                egui::Frame::NONE.fill(c).inner_margin(35.).show(ui, |ui| {
                                    ui.set_min_size(Vec2::new(180., 130.));
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(40.);
                                        ui.label(RichText::new(s).color(INK));
                                    });
                                });
                            }
                        });
                    }
                    5 => {
                        ui.heading("링크 · 즐겨찾기");
                        ui.label("자주 찾는 곳을 가까이에");
                        for s in [
                            "↗ 프로젝트 참고 자료",
                            "↗ 사내 업무 포털",
                            "↗ 디자인 아이디어",
                        ] {
                            if ui.add_sized([400., 52.], egui::Button::new(s)).clicked() {
                                self.toast = "외부 연결 없이 클릭 반응만 시연합니다".into();
                            }
                        }
                    }
                    _ => {
                        ui.heading("하루 업무일지");
                        ui.label("2026년 11월 8일 · 일요일");
                        ui.add_space(20.);
                        ui.heading("오늘 완료한 일");
                        ui.label("✓ 작업 공간 디자인 방향 정리");
                        ui.heading("내일 이어갈 일");
                        ui.label("○ 화면 흐름을 직접 사용하며 확인하기");
                        if ui.button("초안 만들기").clicked() {
                            self.toast = "업무일지 초안 미리보기입니다".into();
                        }
                    }
                }
            });
        });
        egui::Window::new("빠른 메모")
            .open(&mut self.capture)
            .default_size([420., 250.])
            .show(ctx, |ui| {
                ui.label("지금 떠오른 생각은 무엇인가요?");
                ui.add(
                    egui::TextEdit::multiline(&mut self.capture_text)
                        .hint_text("가볍게 적어두세요...")
                        .desired_rows(5),
                );
                if ui.button("수집함에 추가").clicked() && !self.capture_text.trim().is_empty()
                {
                    self.notes.push(Note {
                        title: self.capture_text.lines().next().unwrap_or("새 메모").into(),
                        body: self.capture_text.clone(),
                        done: false,
                        checks: [false; 3],
                    });
                    self.capture_text.clear();
                    self.toast = "화면 메모에 추가했습니다 · 종료하면 초기화됩니다".into();
                }
            });
        egui::Window::new("화면 설정")
            .open(&mut self.settings)
            .show(ctx, |ui| {
                if ui.checkbox(&mut self.dark, "다크 모드").changed() {
                    ctx.set_visuals(Self::visuals(self.dark));
                }
                ui.checkbox(&mut self.split, "메모와 캘린더 나란히");
                ui.label("이 프로토타입의 모든 변경은 메모리에서만 유지됩니다.");
            });
        if self.sticky {
            let mut open = true;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("sticky"),
                egui::ViewportBuilder::default()
                    .with_title("MEMONOKING · 분리 메모")
                    .with_inner_size([520., 640.])
                    .with_min_inner_size([350., 320.]),
                |ctx, _| {
                    if ctx.input(|i| i.viewport().close_requested()) {
                        open = false;
                    }
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("sticky_scroll")
                            .show(ui, |ui| self.editor(ui));
                    });
                },
            );
            self.sticky = open;
        }
        if self.calendar_window {
            let mut open = true;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("floating_calendar"),
                egui::ViewportBuilder::default()
                    .with_title("MEMONOKING · 캘린더")
                    .with_inner_size([720., 650.]),
                |ctx, _| {
                    if ctx.input(|i| i.viewport().close_requested()) {
                        open = false;
                    }
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| self.calendar(ui));
                    });
                },
            );
            self.calendar_window = open;
        }
    }
}
fn main() -> eframe::Result {
    eframe::run_native(
        "MEMONOKING",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1180., 760.])
                .with_min_inner_size([820., 540.]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
