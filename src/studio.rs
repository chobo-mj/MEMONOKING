use crate::icons::{icon_button, Icon};
use crate::*;
use egui::{Pos2, Rect, Sense};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Editor,
    Title,
    Swatch,
    Sticker,
}
pub struct Window {
    pub mode: Mode,
    pub sticker_preferred: bool,
    pub open: bool,
    pub occurrence: Option<NaiveDate>,
    pub schedule_start: Option<NaiveDate>,
    full_size: Vec2,
    title_width: f32,
    skip_title: bool,
    sticker_size: Vec2,
    swatch_size: Vec2,
    applied: Option<(Mode, bool)>,
    focus: bool,
    center: bool,
    help: bool,
    native: crate::native_window::State,
    related: bool,
}
impl Window {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            sticker_preferred: mode == Mode::Sticker,
            open: true,
            occurrence: None,
            schedule_start: None,
            full_size: Vec2::new(620., 720.),
            title_width: 360.,
            skip_title: false,
            sticker_size: Vec2::splat(360.),
            swatch_size: Vec2::splat(48.),
            applied: None,
            focus: true,
            center: true,
            help: false,
            native: Default::default(),
            related: false,
        }
    }
    fn reconcile(&mut self, n: &Note) {
        if let (Some(old), Some(new), Some(d)) = (self.schedule_start, n.date, self.occurrence) {
            if old != new {
                self.occurrence = Some(d + (new - old));
            }
        }
        self.schedule_start = n.date;
        if n.duration > 1 {
            if let (Some(d), Some(start)) = (self.occurrence, n.date) {
                let end = start + chrono::Duration::days(n.duration.max(1) as i64 - 1);
                self.occurrence = Some(d.clamp(start, end));
            }
        }
    }
    pub fn fold(&mut self) {
        self.mode = match self.mode {
            Mode::Editor | Mode::Sticker if self.skip_title => Mode::Swatch,
            Mode::Editor => Mode::Title,
            Mode::Sticker => Mode::Title,
            Mode::Title => Mode::Swatch,
            Mode::Swatch => {
                if self.sticker_preferred {
                    Mode::Sticker
                } else {
                    Mode::Editor
                }
            }
        };
    }
    pub fn sticker(&mut self) {
        self.sticker_preferred = true;
        self.mode = Mode::Sticker;
    }
    pub fn edit(&mut self) {
        self.mode = Mode::Editor;
    }
    fn size(&self) -> Vec2 {
        match self.mode {
            Mode::Editor => self.full_size,
            Mode::Title => Vec2::new(self.title_width, 44.),
            Mode::Swatch => self.swatch_size,
            Mode::Sticker => self.sticker_size,
        }
    }
}
#[derive(Default)]
pub struct Studio {
    pub windows: BTreeMap<usize, Window>,
    pub help: bool,
}
impl Studio {
    pub fn open(&mut self, i: usize, occurrence: Option<NaiveDate>) {
        let w = self
            .windows
            .entry(i)
            .or_insert_with(|| Window::new(Mode::Editor));
        if !w.open {
            w.applied = None;
            w.native.cancel();
        }
        w.open = true;
        w.mode = Mode::Editor;
        w.occurrence = occurrence;
        w.focus = true;
    }
}
pub fn viewport(i: usize) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("memo", i))
}
impl App {
    pub fn new_sticker(&mut self) -> usize {
        let i = self.data.notes.len();
        let mut n = Note::new("", None, i);
        n.body.clear();
        n.color = self.desktop.default_color;
        n.document.font_name = self.desktop.note_font.clone();
        n.document.font_size = self.desktop.note_font_size * 4. / 3.;
        n.document.opacity = self.desktop.opacity / 100.;
        n.document.image_scale = [0.25, 0.5, 0.75, 1., 1.][self.desktop.image_scale];
        n.document.image_fit = self.desktop.image_scale == 4;
        self.data.notes.push(n);
        self.studio.windows.insert(i, Window::new(Mode::Sticker));
        i
    }
    pub fn render_note_windows(&mut self, root: &egui::Context) {
        let ids: Vec<_> = self
            .studio
            .windows
            .iter()
            .filter(|(_, w)| w.open)
            .map(|(&i, _)| i)
            .collect();
        for i in ids {
            let Some(mut w) = self.studio.windows.remove(&i) else {
                continue;
            };
            if self.data.notes[i].deleted
                || (!self.visible(i) && !(self.extra.search_mode && self.extra.include_hidden))
            {
                w.open = false;
                self.studio.windows.insert(i, w);
                continue;
            }
            w.reconcile(&self.data.notes[i]);
            w.skip_title = self.desktop.skip_title;
            let locked = self.data.notes[i].locked;
            if locked {
                w.mode = Mode::Swatch;
            }
            if self.data.notes[i].duration > 1 && w.occurrence.is_none() {
                let today = Local::now().date_naive();
                w.occurrence = if self.data.notes[i].occurs(today) {
                    Some(today)
                } else {
                    self.data.notes[i].date
                };
            }
            let previous_active = self.active;
            let previous_occurrence = self.extra.edit_occurrence;
            self.active = i;
            self.extra.edit_occurrence = w.occurrence;
            let mode = w.mode;
            let key = (mode, locked);
            let size = if locked { Vec2::splat(64.) } else { w.size() };
            let min = if locked {
                size
            } else {
                match mode {
                    Mode::Editor => Vec2::new(480., 480.),
                    Mode::Sticker => Vec2::new(320., 220.),
                    Mode::Title => Vec2::new(180., 44.),
                    Mode::Swatch => Vec2::splat(24.),
                }
            };
            let max = if locked {
                size
            } else {
                match mode {
                    Mode::Title => Vec2::new(1600., 44.),
                    Mode::Swatch => Vec2::splat(360.),
                    _ => Vec2::splat(8192.),
                }
            };
            let top = self.data.notes[i].topmost
                && !(mode == Mode::Swatch && self.desktop.unpin_minimized);
            let title = if self.data.notes[i].title.is_empty() {
                "새 메모"
            } else {
                &self.data.notes[i].title
            };
            let mut builder = egui::ViewportBuilder::default()
                .with_title(title)
                .with_min_inner_size(min)
                .with_max_inner_size(max)
                .with_resizable(!locked)
                .with_transparent(true)
                .with_decorations(false)
                .with_window_level(if top {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                });
            // The builder is a creation request, never a live geometry feedback loop.
            if w.applied.is_none() {
                builder = builder.with_inner_size(size);
            }
            root.show_viewport_immediate(viewport(i),builder,|ctx,_|{
                self.theme(ctx);
                let maximized=ctx.input(|v|v.viewport().maximized.unwrap_or(false));
                if w.applied!=Some(key) {
                    w.native.cancel();
                    if maximized{ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));}
                    ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min));ctx.send_viewport_cmd(egui::ViewportCommand::MaxInnerSize(max));
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(!locked));
                    w.applied=Some(key);
                } else if !maximized {
                    if let Some(rect)=ctx.input(|v|v.viewport().inner_rect){
                        if rect.width()>=min.x && rect.height()>=min.y{
                            match mode{Mode::Editor=>w.full_size=rect.size(),Mode::Sticker=>w.sticker_size=rect.size(),Mode::Title=>w.title_width=rect.width(),Mode::Swatch=>w.swatch_size=rect.size()}
                        }
                    }
                }
                if w.center{if let Some(cmd)=egui::ViewportCommand::center_on_screen(ctx){ctx.send_viewport_cmd(cmd);}w.center=false;}
                if w.focus{ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));ctx.send_viewport_cmd(egui::ViewportCommand::Focus);w.focus=false;}
                if ctx.input(|v|v.viewport().close_requested()){w.open=false;}
                self.memo_shortcuts(ctx,i,&mut w);
                w.native.input(ctx,mode==Mode::Title,mode==Mode::Swatch,!locked && !maximized);
                let base=self.data.notes[i].color;
                let paper=mix(base,Color32::WHITE,(1.-self.data.notes[i].document.opacity).clamp(0.,0.9));
                if mode==Mode::Title {
                    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(mix(base,Color32::WHITE,0.2)).corner_radius(7.).inner_margin(6.)).show(ctx,|ui|{
                        paper_style(ui,paper);
                        let hover=!self.desktop.auto_toolbar||ui.rect_contains_pointer(ui.max_rect());
                        ui.horizontal(|ui|{
                            if hover && icon_button(ui,Icon::Fold,"색상 상자로 접기").clicked(){w.fold();}
                            let width=(ui.available_width()-if hover{32.}else{0.}).max(40.);
                            let r=ui.add_sized([width,28.],egui::Label::new(&self.data.notes[i].title).truncate().sense(Sense::click_and_drag())).on_hover_text("클릭: 스티커 · 드래그: 이동 · 양끝: 폭 조절");
                            if r.clicked(){w.sticker();} else if r.drag_started(){w.native.start_drag(ctx);}
                            if hover{pin_button(ui,&mut self.data.notes[i].topmost);}
                        });
                    });
                } else if mode==Mode::Swatch {
                    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(base).corner_radius(8.)).show(ctx,|ui|{
                        let rect=ui.max_rect();let r=ui.interact(rect.shrink(7.),ui.id().with("swatch"),Sense::click_and_drag());
                        if locked{crate::icons::paint_icon(ui.painter(),Rect::from_center_size(rect.center(),Vec2::splat(28.)),Icon::Lock,INK);}
                        if r.drag_started(){w.native.start_drag(ctx);}
                        if r.double_clicked(){if locked{self.ask_unlock(i,true);}else{w.sticker();}}
                        r.on_hover_cursor(egui::CursorIcon::Grab).on_hover_text(if locked{"잠긴 메모 · 두 번 클릭해 열기"}else{"두 번 클릭: 펼치기 · 드래그: 이동 · 모서리: 크기 · Shift: 정사각형"}).context_menu(|ui|{
                            if locked && ui.button("잠금 해제").clicked(){self.ask_unlock(i,true);ui.close_menu();}
                            if ui.button("스티커 표시 해제").clicked(){w.open=false;ui.close_menu();}
                        });
                    });
                } else {
                    let focused=ctx.input(|v|v.viewport().focused.unwrap_or(false));
                    let engaged=mode==Mode::Editor || !self.desktop.auto_toolbar || focused || ctx.input(|v|v.pointer.hover_pos().is_some());
                    egui::TopBottomPanel::top(egui::Id::new(("memo_header",i))).exact_height(36.).frame(egui::Frame::NONE.fill(if mode==Mode::Sticker{paper}else{ctx.style().visuals.panel_fill}).corner_radius(egui::CornerRadius{nw:12,ne:12,sw:0,se:0}).inner_margin(egui::Margin::symmetric(10,4))).show(ctx,|ui|{
                        if mode==Mode::Sticker{paper_style(ui,paper);}
                        self.memo_header(ui,i,&mut w,mode,engaged,maximized);
                    });
                    if self.active!=i {self.active=i;self.extra.edit_occurrence=w.occurrence;}
                    egui::TopBottomPanel::top(egui::Id::new(("memo_title",i))).frame(egui::Frame::NONE.fill(if mode==Mode::Sticker{paper}else{ctx.style().visuals.panel_fill}).inner_margin(egui::Margin::symmetric(12,5))).show(ctx,|ui|{
                        if mode==Mode::Sticker {
                            paper_style(ui,paper);
                            let font=egui::FontId::new(21.,crate::desktop::family(ui,&self.data.notes[i].document.font_name));
                            ui.add(egui::TextEdit::singleline(&mut self.data.notes[i].title).font(font).frame(false).desired_width(f32::INFINITY).hint_text("제목"));
                        }else{self.note_editor(ui);}
                    });
                    let width=ctx.input(|v|v.viewport().inner_rect.map(|r|r.width())).unwrap_or(size.x);
                    let status_height = if self.data.notes[i].document.status.is_empty() { 0. } else { let small_font = egui::TextStyle::Small.resolve(&ctx.style()); ctx.fonts(|f| f.row_height(&small_font)) + 4. };
                    let footer_height=(if engaged {if width>=354.{92.}else{122.}}else{48.}) + status_height;
                    egui::TopBottomPanel::bottom(egui::Id::new(("memo_footer",i))).exact_height(footer_height).frame(egui::Frame::NONE.fill(if mode==Mode::Sticker{paper}else{ctx.style().visuals.panel_fill}).corner_radius(egui::CornerRadius{nw:0,ne:0,sw:12,se:12}).inner_margin(egui::Margin::symmetric(12,6))).show(ctx,|ui|{
                        if mode==Mode::Sticker{paper_style(ui,paper);}
                        self.data.notes[i].document.toolbar_visible=engaged;
                        let rect=ui.max_rect();
                        crate::composer::footer(ui,&mut self.data.notes[i].document,("memo_document",i),if engaged{32.}else{0.});
                        if engaged{
                            let arrow=Rect::from_min_size(Pos2::new(rect.right()-28.,rect.bottom()-28.),Vec2::splat(28.));
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(arrow),|ui|{
                                if icon_button(ui,Icon::ChevronRight,"연결된 메모").clicked(){w.related = !w.related;}
                            });
                        }
                    });
                    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(if mode==Mode::Sticker{paper}else{ctx.style().visuals.panel_fill}).inner_margin(egui::Margin::symmetric(12,4))).show(ctx,|ui|{
                        if mode==Mode::Sticker{paper_style(ui,paper);}
                        let n=&mut self.data.notes[i];
                        crate::composer::content(ui,&mut n.body,&mut n.document,("memo_document",i));
                    });
                    self.consume_body_project(i);
                    if mode==Mode::Sticker && self.data.notes[i].document.active==0 && (!focused||self.data.notes[i].body.contains('\n')){
                        let n=&mut self.data.notes[i];crate::desktop::infer_title(&mut n.title,&mut n.body);
                    }
                }
                if w.mode!=mode || !w.open {
                    w.native.cancel();w.help=false;w.related=false;
                    if w.mode==Mode::Swatch && self.desktop.unpin_minimized{self.data.notes[i].topmost=false;}
                    if self.extra.picker.as_ref().is_some_and(|p|p.owner==ctx.viewport_id()){self.extra.picker=None;}
                    if self.extra.scope_owner==ctx.viewport_id(){self.extra.completion_scope=None;}
                }
                if self.active!=i{self.active=i;self.extra.edit_occurrence=w.occurrence;}
                self.picker_window(ctx);self.scope_dialog(ctx);
                if w.help {
                    let mut open=true;
                    egui::Window::new("메모 도움말").id(egui::Id::new(("memo_help",i))).open(&mut open).default_width(300.).show(ctx,|ui|{
                        ui.label("윗부분을 끌어 이동하고 모서리로 크기를 조절하세요.");ui.label("접기: 제목 한 줄, 다시 누르면 색상 상자입니다.");ui.label("색상 상자를 두 번 클릭하면 스티커로 돌아옵니다.");ui.label("탭은 본문 아래, 서식 도구는 창 맨 아래에 있습니다.");ui.label("더보기에서 일정 편집과 도움말을 엽니다.");ui.separator();ui.label(format!("새 스티커 · {}",self.shortcut_label(2)));ui.label("창을 닫아도 메모는 목록에 남습니다.");
                    });w.help=open;
                }
                if w.related {
                    let mut open=true;let related:Vec<_>=self.data.notes.iter().enumerate().filter(|(j,n)|self.visible(*j)&&*j!=i&&(n.parents.contains(&i)||self.data.notes[i].parents.contains(j)||self.data.links.contains(&(i.min(*j),i.max(*j))))).map(|(j,_)|j).collect();
                    egui::Window::new("연결된 메모").id(egui::Id::new(("memo_related",i))).open(&mut open).default_width(280.).default_pos(Pos2::new((ctx.screen_rect().width()-290.).max(8.),(ctx.screen_rect().height()-260.).max(8.))).show(ctx,|ui|{
                        if related.is_empty(){ui.label(RichText::new("연결된 메모가 없습니다").color(MUTED));}
                        egui::ScrollArea::vertical().max_height(200.).show(ui,|ui|{for j in related{if ui.link(&self.data.notes[j].title).clicked(){self.open_sticker(j,self.data.notes[j].date);w.related=false;}}});
                    });w.related&=open;
                }
            });
            if self.active == i {
                w.occurrence = self.extra.edit_occurrence;
            }
            self.active = previous_active;
            self.extra.edit_occurrence = previous_occurrence;
            self.studio.windows.insert(i, w);
        }
        self.editor = self
            .studio
            .windows
            .values()
            .any(|w| w.open && w.mode == Mode::Editor);
        self.sticky = self.studio.windows.values().any(|w| w.open);
    }
    fn memo_shortcuts(&mut self, ctx: &egui::Context, i: usize, w: &mut Window) {
        if ctx.input_mut(|v| v.consume_key(egui::Modifiers::CTRL, egui::Key::T)) {
            self.data.notes[i].topmost = !self.data.notes[i].topmost;
        }
        if ctx.input_mut(|v| {
            v.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::H)
        }) {
            w.open = false;
            for other in self.studio.windows.values_mut() {
                other.open = false;
            }
        }
        if ctx.input_mut(|v| v.consume_key(egui::Modifiers::CTRL, egui::Key::P)) {
            self.desktop.project_popup = true;
            self.desktop.focus_dialog = true;
        }
        if ctx.input_mut(|v| v.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            self.catalog.focus_find();
            self.catalog.tab = 1;
            self.page = 4;
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
        }
        if ctx.input_mut(|v| v.consume_key(egui::Modifiers::CTRL, egui::Key::H)) {
            self.catalog.replace_open = true;
            self.catalog.selected = Some(i);
            self.page = 4;
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
        }
        if ctx.input_mut(|v| v.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            if ctx.memory(|m| m.any_popup_open()) {
                ctx.memory_mut(|m| m.close_popup());
                return;
            }
            let now = ctx.input(|v| v.time);
            let old = self.desktop.last_escape.insert(ctx.viewport_id(), now);
            if old.is_some_and(|t| now - t < 0.8) {
                w.open = false;
            }
            w.help = false;
            w.related = false;
            self.extra.picker = None;
        }
    }
    fn memo_header(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        w: &mut Window,
        mode: Mode,
        engaged: bool,
        maximized: bool,
    ) {
        let context = ui.ctx().clone();
        let ctx = &context;
        ui.spacing_mut().item_spacing.x = 4.;
        ui.spacing_mut().button_padding = Vec2::new(6., 4.);
        ui.horizontal(|ui| {
            if mode == Mode::Sticker {
                if icon_button(ui, Icon::Plus, "새 스티커 메모").clicked() {
                    self.new_sticker();
                }
            } else {
                if icon_button(ui, Icon::Fold, "메모 접기").clicked() {
                    w.fold();
                }
                if ui.small_button("스티커 노트").clicked() {
                    w.sticker();
                }
            }
            let controls = if mode == Mode::Editor {
                6.
            } else if engaged {
                8.
            } else {
                3.
            };
            let gap = (ui.available_width() - controls * 32.).max(0.);
            let response = ui.add_sized(
                [gap, 28.],
                egui::Label::new(
                    RichText::new(if mode == Mode::Editor {
                        "메모 편집"
                    } else {
                        ""
                    })
                    .size(11.)
                    .color(MUTED),
                )
                .sense(Sense::drag()),
            );
            if response.drag_started() {
                w.native.start_drag(ctx);
            }
            if engaged {
                self.memo_more(ui, ctx, i, w);
            }
            if mode == Mode::Editor {
                let m = ui.menu_button("   ", |ui| {
                    let mut opacity = self.data.notes[i].document.opacity * 100.;
                    if ui
                        .add(
                            egui::Slider::new(&mut opacity, 25.0..=100.)
                                .integer()
                                .suffix("%"),
                        )
                        .changed()
                    {
                        self.data.notes[i].document.opacity = opacity / 100.;
                    }
                });
                crate::icons::paint_icon(
                    ui.painter(),
                    m.response.rect.shrink(5.),
                    Icon::Opacity,
                    ui.visuals().text_color(),
                );
                m.response.on_hover_text("메모 투명도");
            }
            if engaged && icon_button(ui, Icon::Alarm, "알람 날짜와 시간").clicked() {
                self.open_reminder(i);
            }
            if mode == Mode::Sticker && engaged {
                let m = ui.menu_button("   ", |ui| {
                    crate::desktop::color_choices(ui, &mut self.data.notes[i].color)
                });
                crate::icons::paint_icon(
                    ui.painter(),
                    m.response.rect.shrink(5.),
                    Icon::Palette,
                    INK,
                );
                m.response.on_hover_text("메모 색상");
            }
            pin_button(ui, &mut self.data.notes[i].topmost);
            if mode == Mode::Sticker {
                let mut done = self.data.notes[i]
                    .done_on(self.extra.edit_occurrence.or(self.data.notes[i].date));
                let changed = ui
                    .scope(|ui| {
                        ui.visuals_mut().widgets.inactive.bg_stroke =
                            egui::Stroke::new(1.0_f32, INK.gamma_multiply(0.4));
                        ui.checkbox(&mut done, "").on_hover_text("완료").changed()
                    })
                    .inner;
                if changed {
                    self.done_at(
                        i,
                        self.extra.edit_occurrence.or(self.data.notes[i].date),
                        done,
                    );
                    self.extra.scope_owner = ctx.viewport_id();
                }
                if icon_button(ui, Icon::Fold, "제목 한 줄로 접기").clicked() {
                    w.fold();
                }
            }
            if engaged {
                if icon_button(
                    ui,
                    if maximized {
                        Icon::Restore
                    } else {
                        Icon::Maximize
                    },
                    if maximized {
                        "원래 크기로 복원"
                    } else {
                        "창 최대화"
                    },
                )
                .clicked()
                {
                    w.native.cancel();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                if icon_button(ui, Icon::Close, "창 닫기 · 메모는 유지").clicked() {
                    w.open = false;
                }
            }
        });
    }
    fn memo_more(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, i: usize, w: &mut Window) {
        ui.menu_button("···", |ui| {
            if ui.button("일정과 상세 속성 편집").clicked() {
                w.edit();
                ui.close_menu();
            }
            if ui.button("도움말").clicked() {
                w.help = true;
                ui.close_menu();
            }
            if ui.button("메모 잠금 설정").clicked() {
                self.ask_lock(i);
                ui.close_menu();
            }
            ui.checkbox(&mut self.data.notes[i].wiki, "위키에 보관");
            ui.menu_button("프로젝트 설정", |ui| {
                ui.selectable_value(&mut self.data.notes[i].project, None, "프로젝트 없음");
                for (p, name) in self.data.projects.iter().enumerate() {
                    if !self.extra.hidden_projects.contains(&p)
                        && !self.extra.deleted_projects.contains(&p)
                    {
                        ui.selectable_value(&mut self.data.notes[i].project, Some(p), name);
                    }
                }
            });
            if ui.button("동일 프로젝트 보기").clicked() {
                self.project_filter = Some(self.data.notes[i].project.unwrap_or(NO_PROJECT));
                self.page = 0;
                ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
                ui.close_menu();
            }
            if ui.button("메모 복제본 생성").clicked() {
                self.duplicate_note(i);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("휴지통으로 이동").clicked() {
                self.trash_note(i);
                w.open = false;
                ui.close_menu();
            }
            if ui
                .button(RichText::new("영구삭제…").color(ACCENT))
                .clicked()
            {
                self.request_purge(i);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("메모창 설정").clicked() {
                self.settings = true;
                self.desktop.settings_tab = 2;
                ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
                ui.close_menu();
            }
        })
        .response
        .on_hover_text("더보기");
    }
}
fn paper_style(ui: &mut egui::Ui, paper: Color32) {
    let v = ui.visuals_mut();
    v.dark_mode = false;
    v.override_text_color = Some(INK);
    v.window_fill = Color32::from_rgb(255, 254, 250);
    v.panel_fill = paper;
    v.extreme_bg_color = paper;
    v.selection.bg_fill = Color32::from_rgb(250, 225, 180);
    v.widgets.inactive.bg_fill = paper;
    v.widgets.inactive.weak_bg_fill = Color32::from_white_alpha(65);
    v.widgets.hovered.weak_bg_fill = Color32::from_white_alpha(120);
    v.window_stroke = egui::Stroke::new(1.0_f32, Color32::from_gray(215));
}
fn pin_button(ui: &mut egui::Ui, value: &mut bool) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.), Sense::click());
    if *value || response.hovered() {
        ui.painter().rect_filled(
            rect,
            6.,
            Color32::from_black_alpha(if *value { 20 } else { 8 }),
        );
    }
    crate::icons::paint_icon(
        ui.painter(),
        rect.shrink(5.),
        Icon::Pin,
        if *value {
            INK
        } else {
            INK.gamma_multiply(0.35)
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, *value, "항상 위에")
    });
    if response.on_hover_text("항상 위에 · Ctrl+T").clicked() {
        *value = !*value;
    }
}
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(
        (a.r() as f32 * (1. - t) + b.r() as f32 * t) as u8,
        (a.g() as f32 * (1. - t) + b.g() as f32 * t) as u8,
        (a.b() as f32 * (1. - t) + b.b() as f32 * t) as u8,
    )
}
pub fn nav_button(ui: &mut egui::Ui, text: &str, selected: bool, icon: Icon) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.), Sense::click());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text));
    if selected || r.hovered() {
        ui.painter().rect_filled(
            rect,
            6.,
            if selected {
                if ui.visuals().dark_mode {
                    Color32::from_rgb(67, 49, 44)
                } else {
                    Color32::from_rgb(250, 235, 225)
                }
            } else {
                ui.visuals().widgets.hovered.weak_bg_fill
            },
        );
    }
    let color = if selected {
        ACCENT
    } else {
        ui.visuals().text_color().gamma_multiply(0.85)
    };
    crate::icons::paint_icon(
        ui.painter(),
        Rect::from_center_size(
            egui::pos2(rect.left() + 18., rect.center().y),
            Vec2::splat(18.),
        ),
        icon,
        color,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 38., rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(14.),
        color,
    );
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normal_fold_and_title_to_sticker_are_distinct() {
        let mut w = Window::new(Mode::Editor);
        w.fold();
        assert_eq!(w.mode, Mode::Title);
        w.fold();
        assert_eq!(w.mode, Mode::Swatch);
        w.fold();
        assert_eq!(w.mode, Mode::Editor);
        w.fold();
        w.sticker();
        assert_eq!(w.mode, Mode::Sticker);
        w.fold();
        assert_eq!(w.mode, Mode::Title);
        w.fold();
        assert_eq!(w.mode, Mode::Swatch);
        w.fold();
        assert_eq!(w.mode, Mode::Sticker);
    }
    #[test]
    fn explicit_editor_retains_sticker_restore_intent() {
        let mut w = Window::new(Mode::Sticker);
        w.skip_title = true;
        w.edit();
        assert_eq!(w.mode, Mode::Editor);
        w.fold();
        assert_eq!(w.mode, Mode::Swatch);
        w.fold();
        assert_eq!(w.mode, Mode::Sticker);
    }
    #[test]
    fn adding_stickers_keeps_existing_windows_and_notes_independent() {
        let mut a = App::sample(Local::now().date_naive());
        let old = a.data.notes.len();
        let first = a.new_sticker();
        let second = a.new_sticker();
        assert_ne!(viewport(first), viewport(second));
        a.data.notes[first].body = "첫 번째".into();
        a.data.notes[second].body = "두 번째".into();
        a.studio.windows.get_mut(&first).unwrap().fold();
        assert_eq!(a.studio.windows[&second].mode, Mode::Sticker);
        assert_eq!(a.data.notes.len(), old + 2);
        a.studio.windows.get_mut(&first).unwrap().open = false;
        assert_eq!(a.data.notes[first].body, "첫 번째");
        assert!(a.studio.windows[&second].open);
    }
    #[test]
    fn reopening_a_closed_viewport_reapplies_saved_size_without_resizing_live_windows() {
        let mut studio = Studio::default();
        studio.open(0, None);
        let w = studio.windows.get_mut(&0).unwrap();
        w.sticker();
        w.sticker_size = Vec2::new(410., 370.);
        w.applied = Some((Mode::Sticker, false));
        w.open = false;
        studio.open(0, None);
        let w = studio.windows.get_mut(&0).unwrap();
        w.sticker();
        assert!(w.applied.is_none());
        assert_eq!(w.size(), Vec2::new(410., 370.));
        w.applied = Some((Mode::Editor, false));
        studio.open(0, None);
        assert_eq!(studio.windows[&0].applied, Some((Mode::Editor, false)));
    }
    #[test]
    fn all_three_shortcuts_reject_duplicate_combinations() {
        let mut a = App::sample(Local::now().date_naive());
        a.shortcuts[2] = a.shortcuts[0].clone();
        a.apply_shortcuts();
        assert!(a.hotkey_status.contains("서로 다른"));
        a.shortcuts[2] = a.shortcuts[1].clone();
        a.apply_shortcuts();
        assert!(a.hotkey_status.contains("서로 다른"));
    }
    #[test]
    fn unchecked_single_note_cannot_expose_stale_completion_time() {
        let mut a = App::sample(Local::now().date_naive());
        a.data.notes[0].completed = Some(Local::now().naive_local());
        a.data.notes[0].done = false;
        assert!(a.editor_completion().is_none());
        a.data.notes[0].done = true;
        assert!(a.editor_completion().is_some());
    }
}
#[cfg(test)]
mod occurrence_window_tests {
    use super::*;
    #[test]
    fn open_editor_follows_rescheduled_period_occurrence() {
        let start = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let mut n = Note::new("기간", Some(start), 0);
        n.duration = 3;
        let selected = start + chrono::Duration::days(1);
        let mut w = Window::new(Mode::Editor);
        w.schedule_start = Some(start);
        w.occurrence = Some(selected);
        n.move_start(start + chrono::Duration::days(5));
        w.reconcile(&n);
        assert_eq!(w.occurrence, Some(selected + chrono::Duration::days(5)));
        n.duration = 1;
        w.reconcile(&n);
        assert_eq!(w.schedule_start, n.date);
    }
}
