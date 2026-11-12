//! The detached calendar window, the color box it minimizes to, and the
//! standing calendar that sits on the desktop like wallpaper.
use crate::win_style::PxRect;
use crate::*;
use egui::{Pos2, Rect, Sense, Stroke};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DockState {
    Hidden,
    Window,
    Box,
}

/// An easy-on-the-eyes look for the detached calendar.
pub struct CalTheme {
    pub name: &'static str,
    pub dark: bool,
    pub bg: Color32,
    pub card: Color32,
    pub line: Color32,
    pub ink: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub select: Color32,
    pub today: Color32,
    pub sun: Color32,
    pub sat: Color32,
}

/// Index 0 follows the app's own look.
pub const THEMES: [CalTheme; 3] = [
    CalTheme {
        name: "종이",
        dark: false,
        bg: Color32::from_rgb(244, 239, 228),
        card: Color32::from_rgb(251, 248, 241),
        line: Color32::from_rgb(226, 217, 199),
        ink: Color32::from_rgb(72, 64, 54),
        muted: Color32::from_rgb(128, 118, 104),
        faint: Color32::from_rgb(170, 160, 145),
        select: Color32::from_rgb(240, 228, 206),
        today: Color32::from_rgb(190, 118, 82),
        sun: Color32::from_rgb(184, 98, 80),
        sat: Color32::from_rgb(86, 116, 156),
    },
    CalTheme {
        name: "세이지",
        dark: false,
        bg: Color32::from_rgb(230, 237, 229),
        card: Color32::from_rgb(242, 246, 241),
        line: Color32::from_rgb(205, 217, 205),
        ink: Color32::from_rgb(50, 68, 56),
        muted: Color32::from_rgb(104, 122, 108),
        faint: Color32::from_rgb(146, 162, 150),
        select: Color32::from_rgb(216, 229, 216),
        today: Color32::from_rgb(86, 136, 104),
        sun: Color32::from_rgb(172, 98, 88),
        sat: Color32::from_rgb(78, 114, 148),
    },
    CalTheme {
        name: "나이트",
        dark: true,
        bg: Color32::from_rgb(33, 37, 45),
        card: Color32::from_rgb(40, 45, 54),
        line: Color32::from_rgb(57, 63, 75),
        ink: Color32::from_rgb(212, 218, 226),
        muted: Color32::from_rgb(150, 158, 172),
        faint: Color32::from_rgb(112, 120, 134),
        select: Color32::from_rgb(52, 60, 74),
        today: Color32::from_rgb(226, 156, 92),
        sun: Color32::from_rgb(222, 128, 116),
        sat: Color32::from_rgb(128, 168, 218),
    },
];

struct Standing {
    hwnd: isize,
    /// Client area on screen, physical pixels.
    rect: PxRect,
    /// The whole window (with title bar) before it was pinned.
    outer: PxRect,
    /// The standing button on screen, physical pixels.
    button: PxRect,
    pinned: bool,
    /// Frames to wait for the title bar to go before moving to the desktop.
    wait: u8,
    click: Option<crate::win_style::DesktopClick>,
}

pub struct Dock {
    pub state: DockState,
    /// Outer position and inner size a re-created calendar window opens at.
    open_at: Option<(Pos2, Vec2)>,
    window_at: Option<(Pos2, Vec2)>,
    box_spawn: Pos2,
    box_at: Option<Pos2>,
    pub box_size: f32,
    pub box_color: Color32,
    box_resize: Option<(f32, Pos2)>,
    /// Native window moves of the box (also after very fast drags).
    box_native: crate::native_window::State,
    box_menu: Option<(Pos2, bool)>,
    /// 0 follows the app; 1.. picks `THEMES[theme - 1]`.
    pub theme: usize,
    /// True while the calendar grid is drawn inside the calendar window.
    pub painting: bool,
    standing: Option<Standing>,
    /// Where the right-click view menu (month / 1 week / 2 weeks) is open.
    view_menu: Option<Pos2>,
    /// True: the detached window shows the grid and the day's memos side by
    /// side instead of stacked.
    pub horizontal: bool,
    /// Fraction of the horizontal layout's width given to the day-detail
    /// pane on the right. Kept as a fraction (not pixels) so the split stays
    /// proportional as the window is resized.
    split_frac: f32,
}

impl Default for Dock {
    fn default() -> Self {
        Self {
            state: DockState::Hidden,
            open_at: None,
            window_at: None,
            box_spawn: Pos2::new(120., 120.),
            box_at: None,
            box_size: 56.,
            box_color: PALETTE[6],
            box_resize: None,
            box_native: Default::default(),
            box_menu: None,
            theme: 0,
            painting: false,
            standing: None,
            view_menu: None,
            horizontal: false,
            split_frac: 1. / 3.,
        }
    }
}

impl Dock {
    /// The theme the calendar grid uses right now, if not the app's own.
    pub fn active(&self) -> Option<&'static CalTheme> {
        if self.painting && self.theme > 0 {
            THEMES.get(self.theme - 1)
        } else {
            None
        }
    }
    /// Minimize the calendar window into its color box.
    fn minimize_to_box(&mut self) {
        self.open_at = self.window_at;
        self.box_spawn = self
            .box_at
            .or(self.window_at.map(|(p, _)| p))
            .unwrap_or(self.box_spawn);
        self.state = DockState::Box;
    }
}

fn luminance(c: Color32) -> f32 {
    (0.299 * f32::from(c.r()) + 0.587 * f32::from(c.g()) + 0.114 * f32::from(c.b())) / 255.
}
fn px(r: Rect, ppp: f32) -> PxRect {
    [
        (r.min.x * ppp).round() as i32,
        (r.min.y * ppp).round() as i32,
        (r.max.x * ppp).round() as i32,
        (r.max.y * ppp).round() as i32,
    ]
}

impl App {
    /// Show the calendar window (from the main window or the shortcut).
    pub fn show_calendar(&mut self) {
        self.cal.state = DockState::Window;
    }
    pub fn calendar_dock(&mut self, ctx: &egui::Context) {
        match self.cal.state {
            DockState::Window => self.calendar_window(ctx),
            DockState::Box => self.calendar_box(ctx),
            DockState::Hidden => {}
        }
    }
    fn calendar_window(&mut self, ctx: &egui::Context) {
        let standing = self.cal.standing.is_some();
        let mut builder = egui::ViewportBuilder::default()
            .with_title("OmniMemoJang · 캘린더")
            .with_inner_size([860., 760.])
            .with_min_inner_size([480., 420.])
            .with_decorations(!standing);
        if let Some((at, size)) = self.cal.open_at {
            builder = builder.with_position(at).with_inner_size(size);
        }
        let mut minimize = false;
        let mut stand = false;
        let mut stand_rect = Rect::NOTHING;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("calendar"),
            builder,
            |ctx, _| {
                let theme = self.cal.theme.checked_sub(1).and_then(|t| THEMES.get(t));
                self.theme_with(ctx, theme.map_or(self.dark, |t| t.dark));
                self.drag_input(ctx);
                if ctx.input(|i| i.viewport().close_requested()) {
                    // Closing returns the calendar to its color box.
                    minimize = true;
                }
                let (outer, inner, ppp) = ctx.input(|i| {
                    let v = i.viewport();
                    (
                        v.outer_rect,
                        v.inner_rect,
                        v.native_pixels_per_point.unwrap_or(1.),
                    )
                });
                if !standing {
                    if let (Some(o), Some(i)) = (outer, inner) {
                        self.cal.window_at = Some((o.min, i.size()));
                    }
                }
                let fill = theme.map_or(ctx.style().visuals.panel_fill, |t| t.bg);
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::NONE
                            .fill(fill)
                            .inner_margin(egui::Margin::symmetric(20, 14)),
                    )
                    .show(ctx, |ui| {
                        let (pressed, button) = self.calendar_toolbar(ui, &mut minimize);
                        stand_rect = button;
                        if pressed {
                            stand = true;
                        }
                        ui.add_space(6.);
                        self.cal.painting = true;
                        if self.cal.horizontal {
                            self.calendar_horizontal(ui);
                        } else {
                            egui::ScrollArea::vertical().show(ui, |ui| self.calendar(ui));
                        }
                        self.cal.painting = false;
                    });
                if !standing && self.calendar_view_menu(ctx) {
                    stand = true;
                }
                // The calendar keeps its current place on the desktop, so
                // the stand button's screen rect is known from here. Either
                // the toolbar icon or the right-click "캘린더 분리하기" menu
                // entry can trigger this.
                if stand && !standing {
                    if let (Some(hwnd), Some(i), Some(o)) =
                        (crate::win_style::hwnd(ctx), inner, outer)
                    {
                        self.cal.standing = Some(Standing {
                            hwnd,
                            rect: px(i, ppp),
                            outer: px(o, ppp),
                            button: px(stand_rect.translate(i.min.to_vec2()), ppp),
                            pinned: false,
                            wait: 3,
                            click: None,
                        });
                    }
                }
            },
        );
        self.standing_frame(stand);
        if minimize && self.cal.standing.is_none() {
            self.cal.minimize_to_box();
        }
    }
    /// The grid on the left and the selected day's memos on the right,
    /// separated by a draggable splitter. `self.cal.split_frac` is a
    /// fraction of the available width, not a pixel count, so dragging the
    /// window itself keeps both panes' proportions rather than clipping one.
    fn calendar_horizontal(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let height = rect.height();
        let total = rect.width();
        let handle_w = 12.;
        let min_right = 220.0_f32.min((total - handle_w) * 0.5);
        let max_right = (total - handle_w - 320.).max(min_right);
        let right_w = (total * self.cal.split_frac).clamp(min_right, max_right);
        let left_w = (total - handle_w - right_w).max(240.);

        let left_rect = Rect::from_min_size(rect.min, Vec2::new(left_w, height));
        let handle = Rect::from_min_max(
            egui::pos2(left_rect.right(), rect.top()),
            egui::pos2(left_rect.right() + handle_w, rect.bottom()),
        );
        let right_rect = Rect::from_min_size(
            egui::pos2(handle.right(), rect.top()),
            Vec2::new(total - left_w - handle_w, height),
        );

        let mut left = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("cal_grid_pane")
                .max_rect(left_rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        left.set_clip_rect(ui.clip_rect().intersect(left_rect));
        egui::ScrollArea::vertical()
            .id_salt("cal_grid_scroll")
            .show(&mut left, |ui| self.calendar_grid(ui));

        let drag = ui
            .interact(
                handle,
                ui.id().with("cal_horizontal_splitter"),
                Sense::drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if drag.hovered() || drag.dragged() {
            ui.painter().line_segment(
                [handle.center_top(), handle.center_bottom()],
                Stroke::new(2.0_f32, ACCENT.gamma_multiply(0.6)),
            );
        }
        if drag.dragged() {
            let new_right = (right_w - drag.drag_delta().x).clamp(min_right, max_right);
            self.cal.split_frac = (new_right / total).clamp(0.15, 0.75);
        }

        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("cal_detail_pane")
                .max_rect(right_rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        right.set_clip_rect(ui.clip_rect().intersect(right_rect));
        egui::ScrollArea::vertical()
            .id_salt("cal_detail_scroll")
            .show(&mut right, |ui| self.calendar_day_detail(ui));

        ui.allocate_rect(rect, Sense::hover());
    }
    /// Pin, poll and unpin the standing calendar.
    fn standing_frame(&mut self, button_clicked: bool) {
        let Some(s) = self.cal.standing.as_mut() else {
            return;
        };
        if !s.pinned {
            // Wait until the title bar is gone, then move to the desktop.
            if button_clicked || s.wait > 0 {
                s.wait = s.wait.saturating_sub(1);
                return;
            }
            s.pinned = crate::win_style::pin_to_desktop(s.hwnd, s.rect);
            if !s.pinned {
                self.cal.standing = None;
                return;
            }
            s.click = Some(crate::win_style::DesktopClick::watch(s.button));
            return;
        }
        if s.click.as_ref().is_some_and(|c| c.take()) {
            // Back where it was, title bar included.
            crate::win_style::unpin_from_desktop(s.hwnd, s.outer);
            self.cal.standing = None;
        }
    }
    /// Right-click anywhere in the calendar window: month, 1-week or
    /// 2-week view (the same setting as Settings › Calendar), plus
    /// "캘린더 분리하기" to pin the calendar to the desktop (the same action
    /// as the toolbar's stand icon). Returns whether that entry was clicked,
    /// so the caller can pin the window the same way the toolbar does.
    fn calendar_view_menu(&mut self, ctx: &egui::Context) -> bool {
        let (secondary, pressed, pointer) = ctx.input(|i| {
            (
                i.pointer.secondary_clicked(),
                i.pointer.any_pressed(),
                i.pointer.interact_pos(),
            )
        });
        if secondary {
            self.cal.view_menu = pointer;
            return false;
        }
        let Some(at) = self.cal.view_menu else {
            return false;
        };
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut separate = false;
        // Right-clicking near the window's bottom or right edge would
        // otherwise open this menu partly off-screen.
        let at = ui::clamp_popup_pos(ctx, at, Vec2::new(160., 200.));
        let shown = egui::Area::new(egui::Id::new("calendar_view_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(150.);
                    ui.label(RichText::new("보기 단위").size(12.).color(ui::t(ui).muted));
                    for (k, label) in ["한 달 보기", "1주 보기", "2주 보기"]
                        .into_iter()
                        .enumerate()
                    {
                        let on = self.prefs.cal_weeks == k;
                        let text = if on {
                            RichText::new(format!("✓  {label}")).color(ACCENT)
                        } else {
                            RichText::new(format!("     {label}"))
                        };
                        if ui
                            .add(
                                egui::Button::new(text)
                                    .frame(false)
                                    .min_size(Vec2::new(140., 26.)),
                            )
                            .clicked()
                        {
                            self.prefs.cal_weeks = k;
                            close = true;
                        }
                    }
                    ui.separator();
                    if ui
                        .add(
                            egui::Button::new("캘린더 분리하기")
                                .frame(false)
                                .min_size(Vec2::new(140., 26.)),
                        )
                        .on_hover_text("바탕화면에 고정 · 도구줄의 스탠딩 아이콘과 같은 동작")
                        .clicked()
                    {
                        separate = true;
                        close = true;
                    }
                });
            });
        // A click outside the menu closes it.
        if pressed && pointer.is_some_and(|p| !shown.response.rect.contains(p)) {
            close = true;
        }
        if close {
            self.cal.view_menu = None;
        }
        separate
    }
    /// Theme picker, standing toggle and the big minimize button. Returns
    /// whether the standing button was pressed and its rect.
    fn calendar_toolbar(&mut self, ui: &mut egui::Ui, minimize: &mut bool) -> (bool, Rect) {
        let mut stand = false;
        let mut stand_rect = Rect::NOTHING;
        let standing = self.cal.standing.is_some();
        ui.horizontal(|ui| {
            ui.set_min_height(40.);
            let tone = ui::t(ui);
            ui.label(RichText::new("테마").color(tone.muted).size(12.));
            let follow = ui.visuals().panel_fill;
            for k in 0..=THEMES.len() {
                let (color, name) = match k {
                    0 => (follow, "기본"),
                    _ => (THEMES[k - 1].bg, THEMES[k - 1].name),
                };
                let (rect, r) = ui.allocate_exact_size(Vec2::splat(24.), Sense::click());
                let ring = if self.cal.theme == k {
                    ACCENT
                } else {
                    tone.line
                };
                ui.painter().circle_filled(rect.center(), 9., color);
                ui.painter().circle_stroke(
                    rect.center(),
                    10.,
                    Stroke::new(if self.cal.theme == k { 2.0_f32 } else { 1.0 }, ring),
                );
                if r.on_hover_text(name).clicked() {
                    self.cal.theme = k;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (rect, r) = ui.allocate_exact_size(Vec2::new(96., 38.), Sense::click());
                let fill = if r.hovered() {
                    ui::lerp_color(tone.accent_soft, ACCENT, 0.25)
                } else {
                    tone.accent_soft
                };
                ui.painter().rect_filled(rect, 10., fill);
                icons::paint_icon(
                    ui.painter(),
                    Rect::from_center_size(
                        rect.left_center() + Vec2::new(22., 0.),
                        Vec2::splat(20.),
                    ),
                    icons::Icon::Minus,
                    ACCENT,
                );
                ui.painter().text(
                    rect.left_center() + Vec2::new(38., 0.),
                    egui::Align2::LEFT_CENTER,
                    "최소화",
                    egui::FontId::proportional(14.),
                    ACCENT,
                );
                if r.on_hover_text("작은 캘린더 상자로 접기")
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                    && !standing
                {
                    *minimize = true;
                }
                ui.add_space(4.);
                let horizontal = self.cal.horizontal;
                if icons::toggle_icon(
                    ui,
                    icons::Icon::SplitView,
                    if horizontal {
                        "세로 배치로 전환"
                    } else {
                        "가로 배치로 전환 · 달력과 메모를 나란히"
                    },
                    horizontal,
                )
                .clicked()
                {
                    self.cal.horizontal = !horizontal;
                }
                ui.add_space(4.);
                let r = icons::toggle_icon(
                    ui,
                    icons::Icon::Stand,
                    if standing {
                        "스탠딩 해제 · 이 아이콘을 다시 누르세요"
                    } else {
                        "스탠딩 캘린더 · 바탕화면에 고정"
                    },
                    standing,
                );
                stand_rect = r.rect;
                stand = r.clicked() && !standing;
                if standing {
                    ui.label(
                        RichText::new("바탕화면에 고정됨 · 아이콘을 다시 누르면 해제")
                            .color(tone.muted)
                            .size(12.),
                    );
                }
            });
        });
        (stand, stand_rect)
    }
    fn calendar_box(&mut self, ctx: &egui::Context) {
        let size = Vec2::splat(self.cal.box_size);
        let builder = egui::ViewportBuilder::default()
            .with_title("OmniMemoJang · 캘린더 상자")
            .with_position(self.cal.box_spawn)
            .with_inner_size(size)
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_transparent(true);
        let mut open = false;
        let mut menu_at = None;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("calendar_box"),
            builder,
            |ctx, _| {
                self.theme(ctx);
                crate::win_style::shape(ctx, 1.);
                let (outer, inner) =
                    ctx.input(|i| (i.viewport().outer_rect, i.viewport().inner_rect));
                if let Some(o) = outer {
                    self.cal.box_at = Some(o.min);
                }
                self.cal
                    .box_native
                    .input(ctx, false, false, false, size, size);
                let color = self.cal.box_color;
                let radius = if crate::win_style::NATIVE { 0 } else { 10 };
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(color).corner_radius(radius))
                    .show(ctx, |ui| {
                        let rect = ui.max_rect();
                        let resizing = self.box_resize(ctx, rect.size());
                        let ink = if luminance(color) > 0.55 {
                            INK
                        } else {
                            Color32::WHITE
                        };
                        let icon = rect.size().min_elem() * 0.56;
                        icons::paint_icon(
                            ui.painter(),
                            Rect::from_center_size(rect.center(), Vec2::splat(icon)),
                            icons::Icon::Today,
                            ink,
                        );
                        let r = ui
                            .interact(
                                rect.shrink(6.),
                                ui.id().with("box"),
                                Sense::click_and_drag(),
                            )
                            .on_hover_text("누르면 캘린더 열기 · 끌어서 이동 · 우클릭: 색/닫기")
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if (r.drag_started() || r.drag_stopped()) && !resizing {
                            self.cal.box_native.start_drag(ctx);
                        }
                        if r.clicked() {
                            open = true;
                        }
                        if r.secondary_clicked() {
                            if let (Some(i), Some(p)) = (inner, r.interact_pointer_pos()) {
                                menu_at = Some(i.min + p.to_vec2());
                            }
                        }
                    });
            },
        );
        if let Some(p) = menu_at {
            self.cal.box_menu = Some((p, false));
        }
        self.calendar_box_menu(ctx);
        if open {
            self.cal.box_menu = None;
            self.cal.state = DockState::Window;
        }
    }
    /// Resize the box from its right or bottom edge. Returns true while resizing.
    fn box_resize(&mut self, ctx: &egui::Context, size: Vec2) -> bool {
        let (pointer, down, pressed) = ctx.input(|v| {
            (
                v.pointer.latest_pos(),
                v.pointer.primary_down(),
                v.pointer.primary_pressed(),
            )
        });
        let band = 8.;
        if let Some(p) = pointer {
            let edge = p.x >= size.x - band || p.y >= size.y - band;
            if edge || self.cal.box_resize.is_some() {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            }
            if pressed && edge {
                self.cal.box_resize = Some((self.cal.box_size, p));
            }
            if let Some((start, origin)) = self.cal.box_resize {
                let delta = (p - origin).max_elem();
                let next = (start + delta).clamp(32., 200.);
                if (next - self.cal.box_size).abs() > 0.5 {
                    self.cal.box_size = next;
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::splat(next)));
                }
            }
        }
        if !down {
            self.cal.box_resize = None;
        }
        self.cal.box_resize.is_some()
    }
    fn calendar_box_menu(&mut self, ctx: &egui::Context) {
        let Some((at, mut seen)) = self.cal.box_menu else {
            return;
        };
        let mut done = false;
        let mut close_all = false;
        let mut open = false;
        let shown = crate::studio::menu_window(
            ctx,
            egui::ViewportId::from_hash_of("calendar_box_menu"),
            at,
            Vec2::new(220., 120.),
            None,
            |ui| {
                ui.label(RichText::new("상자 색").size(12.).color(ui::t(ui).muted));
                ui.horizontal_wrapped(|ui| {
                    for c in PALETTE {
                        let (rect, r) = ui.allocate_exact_size(Vec2::splat(22.), Sense::click());
                        ui.painter().circle_filled(rect.center(), 9., c);
                        if c == self.cal.box_color {
                            ui.painter().circle_stroke(
                                rect.center(),
                                10.5,
                                Stroke::new(2.0_f32, ACCENT),
                            );
                        }
                        if r.clicked() {
                            self.cal.box_color = c;
                        }
                    }
                });
                ui.separator();
                if ui.button("캘린더 열기").clicked() {
                    open = true;
                    done = true;
                }
                if ui.button("캘린더 상자 닫기").clicked() {
                    close_all = true;
                    done = true;
                }
            },
        );
        seen |= shown.focused;
        if shown.escape || (seen && !shown.focused) {
            done = true;
        }
        self.cal.box_menu = (!done).then_some((at, seen));
        if open {
            self.cal.state = DockState::Window;
        }
        if close_all {
            self.cal.state = DockState::Hidden;
        }
    }
}
