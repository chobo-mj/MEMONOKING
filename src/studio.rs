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
    sticker_size: Vec2,
    swatch_size: Vec2,
    applied: Option<Mode>,
    focus: bool,
    center: bool,
    help: bool,
    custom_color: bool,
    gesture: Option<Gesture>,
    swatch_drag: Option<crate::refinements::SwatchDrag>,
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
            sticker_size: Vec2::splat(360.),
            swatch_size: Vec2::splat(48.),
            applied: None,
            focus: true,
            center: true,
            help: false,
            custom_color: false,
            gesture: None,
            swatch_drag: None,
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
                let end = start + chrono::Duration::days(n.duration.clamp(1, 5) as i64 - 1);
                self.occurrence = Some(d.clamp(start, end));
            }
        }
    }
    pub fn fold(&mut self) {
        self.mode = match self.mode {
            Mode::Editor if self.sticker_preferred => Mode::Swatch,
            Mode::Editor => Mode::Title,
            Mode::Title | Mode::Sticker => Mode::Swatch,
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
            Mode::Title => Vec2::new(360., 44.),
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
        w.open = true;
        w.mode = Mode::Editor;
        w.occurrence = occurrence;
        w.focus = true;
    }
}
pub fn viewport(i: usize) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("memo", i))
}
struct Gesture {
    pointer: Pos2,
    position: Pos2,
    size: Vec2,
    corner: Option<(bool, bool)>,
}
impl App {
    pub fn new_sticker(&mut self) -> usize {
        let i = self.data.notes.len();
        let mut n = Note::new("새 스티커 노트", None, i);
        n.body.clear();
        n.color = PALETTE[i % PALETTE.len()];
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
            let previous_active = self.active;
            let previous_occ = self.extra.edit_occurrence;
            let previous_custom = self.extra.custom_color;
            let previous_size = self.extra.swatch_size;
            let previous_drag = self.extra.swatch_drag.take();
            self.active = i;
            self.extra.edit_occurrence = w.occurrence;
            self.extra.custom_color = w.custom_color;
            self.extra.swatch_size = w.swatch_size;
            self.extra.swatch_drag = w.swatch_drag.take();
            let mode = w.mode;
            let size = w.size();
            let min = match mode {
                Mode::Editor => Vec2::new(420., 420.),
                Mode::Sticker => Vec2::new(260., 220.),
                Mode::Title => size,
                Mode::Swatch => Vec2::splat(24.),
            };
            let max = if mode == Mode::Title {
                size
            } else if mode == Mode::Swatch {
                Vec2::splat(360.)
            } else {
                Vec2::splat(4096.)
            };
            let top = self.data.notes[i].topmost;
            let title = self.data.notes[i].title.clone();
            root.show_viewport_immediate(
                viewport(i),
                egui::ViewportBuilder::default()
                    .with_title(if title.is_empty() {
                        "새 메모"
                    } else {
                        &title
                    })
                    .with_inner_size(size)
                    .with_min_inner_size(min)
                    .with_resizable(mode != Mode::Title)
                    .with_decorations(false)
                    .with_window_level(if top {
                        egui::WindowLevel::AlwaysOnTop
                    } else {
                        egui::WindowLevel::Normal
                    }),
                |ctx, _| {
                    self.theme(ctx);
                    if w.applied != Some(mode) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min));
                        ctx.send_viewport_cmd(egui::ViewportCommand::MaxInnerSize(max));
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(
                            mode != Mode::Title,
                        ));
                        w.applied = Some(mode);
                    } else if let Some(r) = ctx.input(|v| v.viewport().inner_rect) {
                        if r.size().x >= min.x && r.size().y >= min.y {
                            match mode {
                                Mode::Editor => w.full_size = r.size(),
                                Mode::Sticker => w.sticker_size = r.size(),
                                Mode::Swatch => self.extra.swatch_size = r.size(),
                                _ => {}
                            }
                        }
                    }
                    if w.center {
                        if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
                            ctx.send_viewport_cmd(cmd);
                        }
                        w.center = false;
                    }
                    if w.focus {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        w.focus = false;
                    }
                    if ctx.input(|v| v.viewport().close_requested()) {
                        w.open = false;
                    }
                    let base = self.data.notes[i].color;
                    let paper = mix(base, Color32::WHITE, 0.63);
                    let header = mix(base, Color32::WHITE, 0.22);
                    let frame = egui::Frame::NONE
                        .fill(if mode == Mode::Swatch {
                            base
                        } else if mode == Mode::Title {
                            header
                        } else if mode == Mode::Sticker {
                            paper
                        } else {
                            ctx.style().visuals.panel_fill
                        })
                        .inner_margin(if mode == Mode::Swatch {
                            0.
                        } else if mode == Mode::Title {
                            6.
                        } else {
                            14.
                        });
                    let mut drag_regions = vec![];
                    egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
                        if mode == Mode::Swatch {
                            self.data.notes[i].fold = 2;
                            self.swatch_ui(ctx, ui, i);
                            if self.data.notes[i].fold == 0 {
                                w.fold();
                            }
                            return;
                        }
                        if mode == Mode::Sticker || mode == Mode::Title {
                            ui.visuals_mut().override_text_color = Some(INK);
                            ui.visuals_mut().widgets.inactive.bg_fill = paper;
                            ui.visuals_mut().extreme_bg_color = paper;
                            ui.visuals_mut().selection.bg_fill = mix(base, Color32::WHITE, 0.05);
                        }
                        if mode == Mode::Title {
                            ui.horizontal(|ui| {
                                if icon_button(ui, Icon::Fold, "작은 색상 상자로 접기").clicked()
                                {
                                    w.fold();
                                }
                                let width = (ui.available_width() - 36.).max(80.);
                                let r = ui
                                    .add_sized(
                                        [width, 28.],
                                        egui::Label::new(
                                            RichText::new(shorten(&self.data.notes[i].title, 22))
                                                .color(INK)
                                                .size(14.),
                                        )
                                        .truncate()
                                        .sense(Sense::click_and_drag()),
                                    )
                                    .on_hover_text("클릭: 스티커 노트 · 드래그: 이동");
                                if r.clicked() {
                                    w.sticker();
                                }
                                drag_regions.push(r.rect);
                                if icon_button(ui, Icon::Edit, "전체 메모 편집").clicked() {
                                    w.edit();
                                }
                            });
                            return;
                        }
                        ui.horizontal(|ui| {
                            if icon_button(
                                ui,
                                Icon::Fold,
                                if mode == Mode::Sticker {
                                    "스티커 최소화"
                                } else {
                                    "메모 접기"
                                },
                            )
                            .clicked()
                            {
                                w.fold();
                                self.extra.picker = None;
                            }
                            if mode == Mode::Editor
                                && ui
                                    .add(
                                        egui::Button::new("스티커 노트")
                                            .small()
                                            .selected(w.sticker_preferred),
                                    )
                                    .on_hover_text("제목과 본문만 보이는 스티커로 전환")
                                    .clicked()
                            {
                                w.sticker();
                            }
                            let controls = if mode == Mode::Sticker { 4. } else { 2. };
                            let width = (ui.available_width() - controls * 38.).max(20.);
                            let r = ui.add_sized(
                                [width, 28.],
                                egui::Label::new(
                                    RichText::new(if mode == Mode::Sticker {
                                        "스티커 노트"
                                    } else {
                                        "메모 편집"
                                    })
                                    .size(12.)
                                    .color(MUTED),
                                )
                                .sense(Sense::drag()),
                            );
                            drag_regions.push(r.rect);
                            if mode == Mode::Sticker {
                                if icon_button(ui, Icon::Plus, "새 스티커 노트 추가").clicked()
                                {
                                    self.new_sticker();
                                }
                                if icon_button(ui, Icon::Edit, "일정과 상세 속성 편집").clicked()
                                {
                                    w.edit();
                                }
                            }
                            if icon_button(ui, Icon::Help, "이 창 사용 도움말").clicked() {
                                w.help = !w.help;
                            }
                            if icon_button(ui, Icon::Close, "창 닫기 · 메모는 유지").clicked()
                            {
                                w.open = false;
                            }
                        });
                        ui.add_space(10.);
                        if mode == Mode::Sticker {
                            let rows =
                                ((ui.available_height() - 65.) / 22.).clamp(4., 40.) as usize;
                            egui::ScrollArea::vertical()
                                .id_salt(("sticker_content", i))
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.data.notes[i].title)
                                            .font(egui::FontId::proportional(22.))
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(1)
                                            .frame(false)
                                            .hint_text("제목"),
                                    );
                                    ui.add_space(4.);
                                    ui.separator();
                                    ui.add_space(5.);
                                    crate::refinements::body_input(
                                        ui,
                                        &mut self.data.notes[i].body,
                                        rows,
                                        "여기에 메모하세요…",
                                    );
                                });
                        } else {
                            egui::ScrollArea::vertical()
                                .id_salt(("editor_body", i))
                                .show(ui, |ui| self.note_editor(ui));
                        }
                    });
                    if mode != Mode::Swatch {
                        window_gestures(ctx, &mut w, &drag_regions, min, max);
                    }
                    if w.mode != mode || !w.open {
                        w.help = false;
                        self.extra.custom_color = false;
                        if self
                            .extra
                            .picker
                            .as_ref()
                            .is_some_and(|p| p.owner == ctx.viewport_id())
                        {
                            self.extra.picker = None;
                        }
                        if self.extra.scope_owner == ctx.viewport_id() {
                            self.extra.completion_scope = None;
                        }
                    }
                    if let Some(p) = &mut self.extra.picker {
                        if p.owner == egui::ViewportId::from_hash_of("note_editor") {
                            p.owner = ctx.viewport_id();
                        }
                    }
                    self.picker_window(ctx);
                    self.scope_dialog(ctx);
                    if w.help {
                        let mut open = true;
                        egui::Window::new("메모 창 도움말")
                            .id(egui::Id::new(("help", i)))
                            .open(&mut open)
                            .default_width(320.)
                            .show(ctx, |ui| {
                                ui.strong("필요한 만큼, 가볍게");
                                ui.add_space(8.);
                                ui.label("— 접기: 작은 창으로 정리합니다.");
                                ui.label("연필: 일정과 상세 정보를 편집합니다.");
                                ui.label("제목 한 줄을 클릭하면 스티커가 됩니다.");
                                ui.label("스티커의 +: 새 메모를 별도 창으로 만듭니다.");
                                ui.label("윗부분을 끌어 이동하고 모서리로 크기를 조절하세요.");
                                ui.separator();
                                ui.label(format!("새 스티커 단축키 · {}", self.shortcut_label(2)));
                                ui.label("창을 닫아도 메모는 목록에 남습니다.");
                            });
                        w.help = open;
                    }
                    if self.extra.custom_color {
                        let mut open = true;
                        egui::Window::new("사용자 지정 색상")
                            .open(&mut open)
                            .default_width(280.)
                            .show(ctx, |ui| {
                                egui::color_picker::color_picker_color32(
                                    ui,
                                    &mut self.data.notes[i].color,
                                    egui::color_picker::Alpha::Opaque,
                                );
                            });
                        self.extra.custom_color = open;
                    }
                },
            );
            if self.active == i {
                w.occurrence = self.extra.edit_occurrence;
            }
            w.custom_color = self.extra.custom_color;
            w.swatch_size = self.extra.swatch_size;
            w.swatch_drag = self.extra.swatch_drag.take();
            self.active = previous_active;
            self.extra.edit_occurrence = previous_occ;
            self.extra.custom_color = previous_custom;
            self.extra.swatch_size = previous_size;
            self.extra.swatch_drag = previous_drag;
            self.studio.windows.insert(i, w);
        }
        self.editor = self
            .studio
            .windows
            .values()
            .any(|w| w.open && w.mode == Mode::Editor);
        self.sticky = self.studio.windows.values().any(|w| w.open);
    }
}
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(
        (a.r() as f32 * (1. - t) + b.r() as f32 * t) as u8,
        (a.g() as f32 * (1. - t) + b.g() as f32 * t) as u8,
        (a.b() as f32 * (1. - t) + b.b() as f32 * t) as u8,
    )
}
fn window_gestures(ctx: &egui::Context, w: &mut Window, drag: &[Rect], min: Vec2, max: Vec2) {
    let Some(rect) = ctx.input(|v| v.viewport().inner_rect) else {
        return;
    };
    let size = rect.size();
    let outer = ctx
        .input(|v| v.viewport().outer_rect)
        .map(|r| r.min)
        .unwrap_or(Pos2::ZERO);
    let local = Rect::from_min_size(Pos2::ZERO, size);
    for e in ctx.input(|v| v.events.clone()) {
        match e {
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                ..
            } => {
                let l = pos.x < 9.;
                let r = pos.x > size.x - 9.;
                let t = pos.y < 9.;
                let b = pos.y > size.y - 9.;
                let corner = if w.mode != Mode::Title && (l || r) && (t || b) {
                    Some((l, t))
                } else {
                    None
                };
                if corner.is_some() || drag.iter().any(|r| r.contains(pos)) {
                    w.gesture = Some(Gesture {
                        pointer: outer + pos.to_vec2(),
                        position: outer,
                        size,
                        corner,
                    });
                }
            }
            egui::Event::PointerMoved(pos) => {
                if let Some(g) = &w.gesture {
                    apply_gesture(ctx, g, outer + pos.to_vec2() - g.pointer, min, max);
                }
            }
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                ..
            } => {
                if let Some(g) = w.gesture.take() {
                    apply_gesture(ctx, &g, outer + pos.to_vec2() - g.pointer, min, max);
                }
            }
            _ => {}
        }
    }
    if w.mode != Mode::Title {
        if let Some(p) = ctx.input(|v| v.pointer.hover_pos()) {
            let nw = Rect::from_min_size(local.min, Vec2::splat(10.));
            let se = Rect::from_min_size(local.max - Vec2::splat(10.), Vec2::splat(10.));
            let ne = Rect::from_min_size(egui::pos2(local.max.x - 10., 0.), Vec2::splat(10.));
            let sw = Rect::from_min_size(egui::pos2(0., local.max.y - 10.), Vec2::splat(10.));
            if nw.contains(p) || se.contains(p) {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            } else if ne.contains(p) || sw.contains(p) {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeNeSw);
            }
        }
    }
}
fn apply_gesture(ctx: &egui::Context, g: &Gesture, delta: Vec2, min: Vec2, max: Vec2) {
    if delta.length() < 3. {
        return;
    }
    if let Some((left, top)) = g.corner {
        let size = Vec2::new(
            g.size.x + if left { -delta.x } else { delta.x },
            g.size.y + if top { -delta.y } else { delta.y },
        )
        .clamp(min, max);
        let pos = g.position
            + Vec2::new(
                if left { g.size.x - size.x } else { 0. },
                if top { g.size.y - size.y } else { 0. },
            );
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
    } else {
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(g.position + delta));
    }
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
        assert_eq!(w.mode, Mode::Swatch);
        w.fold();
        assert_eq!(w.mode, Mode::Sticker);
    }
    #[test]
    fn explicit_editor_retains_sticker_restore_intent() {
        let mut w = Window::new(Mode::Sticker);
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
