//! Shared visual language: colors, type scale, compact controls and
//! dialogs that open as real windows instead of being clipped by a parent.
use crate::icons::{paint_icon, Icon};
use eframe::egui::{
    self, Color32, CornerRadius, FontId, Id, Pos2, Rect, Response, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2, ViewportBuilder, ViewportClass, ViewportId,
};
use std::hash::Hash;

pub const ACCENT: Color32 = Color32::from_rgb(217, 87, 56);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(252, 237, 231);
pub const BLUE: Color32 = Color32::from_rgb(56, 110, 196);

/// Color tokens resolved for the current light or dark theme.
#[derive(Clone, Copy)]
pub struct Tone {
    pub bg: Color32,
    pub sidebar: Color32,
    pub field: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub line: Color32,
    pub line_strong: Color32,
    pub ink: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub accent_soft: Color32,
}
pub fn tone(dark: bool) -> Tone {
    if dark {
        Tone {
            bg: Color32::from_rgb(30, 30, 29),
            sidebar: Color32::from_rgb(25, 25, 24),
            field: Color32::from_rgb(43, 43, 41),
            hover: Color32::from_rgb(50, 50, 48),
            pressed: Color32::from_rgb(60, 60, 57),
            line: Color32::from_rgb(52, 52, 50),
            line_strong: Color32::from_rgb(70, 70, 67),
            ink: Color32::from_rgb(234, 233, 229),
            muted: Color32::from_rgb(158, 157, 152),
            faint: Color32::from_rgb(110, 109, 105),
            accent_soft: Color32::from_rgb(70, 42, 34),
        }
    } else {
        Tone {
            bg: Color32::WHITE,
            sidebar: Color32::from_rgb(248, 247, 245),
            field: Color32::from_rgb(242, 241, 238),
            hover: Color32::from_rgb(235, 234, 231),
            pressed: Color32::from_rgb(225, 224, 220),
            line: Color32::from_rgb(234, 233, 230),
            line_strong: Color32::from_rgb(214, 213, 209),
            ink: Color32::from_rgb(44, 43, 40),
            muted: Color32::from_rgb(120, 119, 115),
            faint: Color32::from_rgb(166, 165, 160),
            accent_soft: ACCENT_SOFT,
        }
    }
}
pub fn t(ui: &Ui) -> Tone {
    tone(ui.visuals().dark_mode)
}

/// Install visuals and spacing for a whole context. `body` is the body size in points.
pub fn apply(ctx: &egui::Context, dark: bool, family: egui::FontFamily, body: f32) {
    let c = tone(dark);
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = c.bg;
    v.window_fill = c.bg;
    v.faint_bg_color = if dark {
        Color32::from_rgb(35, 35, 34)
    } else {
        Color32::from_rgb(250, 250, 248)
    };
    v.extreme_bg_color = c.field;
    v.code_bg_color = c.field;
    v.override_text_color = Some(c.ink);
    v.hyperlink_color = BLUE;
    v.selection.bg_fill = c.accent_soft;
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.window_stroke = Stroke::new(1.0_f32, c.line);
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 6],
        blur: 22,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 90 } else { 28 }),
    };
    v.popup_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 14,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 90 } else { 24 }),
    };
    v.striped = false;
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Circle;
    let radius = CornerRadius::same(6);
    let w = &mut v.widgets;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, c.line);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, c.ink);
    w.noninteractive.corner_radius = radius;
    for (state, fill) in [
        (&mut w.inactive, c.field),
        (&mut w.hovered, c.hover),
        (&mut w.active, c.pressed),
        (&mut w.open, c.hover),
    ] {
        state.weak_bg_fill = fill;
        state.bg_fill = fill;
        state.bg_stroke = Stroke::NONE;
        state.corner_radius = radius;
        state.expansion = 0.;
        state.fg_stroke = Stroke::new(1.2_f32, c.ink);
    }
    w.inactive.bg_fill = c.line_strong;
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(6., 6.);
        s.spacing.button_padding = Vec2::new(9., 4.);
        s.spacing.interact_size = Vec2::new(28., 26.);
        s.spacing.icon_width = 15.;
        s.spacing.icon_width_inner = 8.;
        s.spacing.icon_spacing = 6.;
        s.spacing.menu_margin = egui::Margin::same(6);
        s.spacing.window_margin = egui::Margin::same(14);
        s.spacing.combo_width = 120.;
        s.spacing.indent = 16.;
        s.spacing.slider_width = 160.;
        s.spacing.scroll = egui::style::ScrollStyle::floating();
        s.spacing.scroll.bar_width = 8.;
        s.spacing.scroll.floating_allocated_width = 0.;
        s.spacing.scroll.dormant_handle_opacity = 0.;
        s.spacing.scroll.dormant_background_opacity = 0.;
        s.interaction.selectable_labels = false;
        let sizes = [
            (egui::TextStyle::Small, body * 0.86),
            (egui::TextStyle::Body, body),
            (egui::TextStyle::Button, body),
            (egui::TextStyle::Heading, body * 1.5),
            (egui::TextStyle::Monospace, body * 0.92),
        ];
        for (style, size) in sizes {
            s.text_styles.insert(
                style.clone(),
                FontId::new(
                    size,
                    if style == egui::TextStyle::Monospace {
                        egui::FontFamily::Monospace
                    } else {
                        family.clone()
                    },
                ),
            );
        }
    });
}

/// Muted helper text.
pub fn hint(ui: &mut Ui, text: impl Into<String>) -> Response {
    let c = t(ui).muted;
    ui.label(RichText::new(text.into()).small().color(c))
}

/// Small uppercase-style group label used in the sidebar and settings.
pub fn caption(ui: &mut Ui, text: &str) -> Response {
    let c = t(ui).faint;
    ui.label(RichText::new(text).size(11.5).color(c).strong())
}

/// Filled accent button for the single primary action of a view.
pub fn primary(ui: &mut Ui, text: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(Color32::WHITE).strong())
            .fill(ACCENT)
            .corner_radius(6.),
    )
    .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Menu opened from an icon-only button.
pub fn icon_menu(ui: &mut Ui, icon: Icon, tooltip: &str, add: impl FnOnce(&mut Ui)) -> Response {
    let button = egui::Button::new(RichText::new("  ").size(12.))
        .frame(false)
        .min_size(Vec2::splat(28.));
    let r = egui::menu::menu_custom_button(ui, button, add).response;
    if r.hovered() {
        ui.painter().rect_filled(
            r.rect,
            6.,
            if ui.visuals().dark_mode {
                Color32::from_white_alpha(18)
            } else {
                Color32::from_black_alpha(10)
            },
        );
    }
    let color = ui
        .visuals()
        .text_color()
        .gamma_multiply(if r.hovered() { 1. } else { 0.78 });
    paint_icon(ui.painter(), r.rect.shrink(5.5), icon, color);
    r.on_hover_text(tooltip)
}

/// iOS-like segmented control. Returns true when the value changed.
pub fn segmented(ui: &mut Ui, value: &mut usize, labels: &[&str]) -> bool {
    let c = t(ui);
    let font = FontId::new(
        ui.style().text_styles[&egui::TextStyle::Button].size * 0.95,
        ui.style().text_styles[&egui::TextStyle::Button]
            .family
            .clone(),
    );
    let galleys: Vec<_> = labels
        .iter()
        .map(|l| {
            ui.painter()
                .layout_no_wrap((*l).into(), font.clone(), c.ink)
        })
        .collect();
    let widths: Vec<f32> = galleys.iter().map(|g| g.size().x + 22.).collect();
    let height = 28.;
    let base = ui.next_auto_id();
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(widths.iter().sum::<f32>() + 4., height),
        Sense::hover(),
    );
    ui.painter().rect_filled(rect, 7., c.field);
    let mut x = rect.left() + 2.;
    let mut changed = false;
    for (i, (galley, w)) in galleys.into_iter().zip(widths).enumerate() {
        let r = Rect::from_min_size(Pos2::new(x, rect.top() + 2.), Vec2::new(w, height - 4.));
        let resp = ui
            .interact(r, base.with(("seg", i)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let selected = *value == i;
        if selected {
            ui.painter().rect_filled(
                r.translate(Vec2::new(0., 1.)),
                6.,
                Color32::from_black_alpha(14),
            );
            ui.painter().rect_filled(
                r,
                6.,
                if ui.visuals().dark_mode {
                    c.pressed
                } else {
                    Color32::WHITE
                },
            );
        } else if resp.hovered() {
            ui.painter().rect_filled(r, 6., c.hover);
        }
        ui.painter().galley(
            r.center() - galley.size() / 2.,
            galley,
            if selected { c.ink } else { c.muted },
        );
        if resp.clicked() && !selected {
            *value = i;
            changed = true;
        }
        x += w;
    }
    changed
}

/// Text tabs with an accent underline for the selected one.
pub fn tabs(ui: &mut Ui, value: &mut usize, labels: &[&str]) -> bool {
    let c = t(ui);
    let mut changed = false;
    let base = ui.next_auto_id();
    let (bar, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.), Sense::hover());
    ui.painter().line_segment(
        [bar.left_bottom(), bar.right_bottom()],
        Stroke::new(1.0_f32, c.line),
    );
    let font = egui::TextStyle::Button.resolve(ui.style());
    let mut x = bar.left();
    for (i, label) in labels.iter().enumerate() {
        let selected = *value == i;
        let galley = ui.painter().layout_no_wrap(
            (*label).into(),
            font.clone(),
            if selected { c.ink } else { c.muted },
        );
        let r = Rect::from_min_size(
            Pos2::new(x, bar.top()),
            Vec2::new(galley.size().x + 20., 34.),
        );
        let resp = ui
            .interact(r, base.with(("tab", i)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if resp.hovered() && !selected {
            ui.painter()
                .rect_filled(r.shrink2(Vec2::new(2., 4.)), 6., c.hover);
        }
        ui.painter()
            .galley(r.center() - galley.size() / 2., galley, c.ink);
        if selected {
            ui.painter().line_segment(
                [
                    Pos2::new(r.left() + 6., r.bottom() - 1.),
                    Pos2::new(r.right() - 6., r.bottom() - 1.),
                ],
                Stroke::new(2.0_f32, ACCENT),
            );
        }
        if resp.clicked() && !selected {
            *value = i;
            changed = true;
        }
        x = r.right() + 4.;
    }
    changed
}

/// A compact on/off switch.
pub fn switch(ui: &mut Ui, on: &mut bool) -> Response {
    let size = Vec2::new(32., 18.);
    let (rect, mut r) = ui.allocate_exact_size(size, Sense::click());
    if r.clicked() {
        *on = !*on;
        r.mark_changed();
    }
    let how_on = ui.ctx().animate_bool_responsive(r.id, *on);
    let c = t(ui);
    ui.painter()
        .rect_filled(rect, 9., lerp_color(c.line_strong, ACCENT, how_on));
    let x = egui::lerp((rect.left() + 9.)..=(rect.right() - 9.), how_on);
    ui.painter()
        .circle_filled(Pos2::new(x, rect.center().y), 7., Color32::WHITE);
    r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *on, ""));
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A settings row: label (and optional description) on the left, switch on the right.
pub fn toggle_row(ui: &mut Ui, on: &mut bool, label: &str, description: &str) -> Response {
    let mut out = None;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - 48.).max(120.));
            ui.label(label);
            if !description.is_empty() {
                hint(ui, description);
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            out = Some(switch(ui, on));
        });
    });
    ui.add_space(4.);
    out.unwrap()
}

/// Round task checkbox in the style of to-do apps.
pub fn round_check(ui: &mut Ui, checked: &mut bool, tint: Option<Color32>) -> Response {
    let (rect, mut r) = ui.allocate_exact_size(Vec2::splat(20.), Sense::click());
    if r.clicked() {
        *checked = !*checked;
        r.mark_changed();
    }
    let c = t(ui);
    let color = tint.unwrap_or(c.faint);
    let center = rect.center();
    if *checked {
        ui.painter().circle_filled(center, 8., ACCENT);
        check_mark(ui.painter(), center, Color32::WHITE);
    } else {
        ui.painter()
            .circle_stroke(center, 7.5, Stroke::new(1.4_f32, color));
        if r.hovered() {
            check_mark(ui.painter(), center, color);
        }
    }
    r.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *checked, "완료")
    });
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}
fn check_mark(p: &egui::Painter, c: Pos2, color: Color32) {
    p.add(egui::Shape::line(
        vec![
            c + Vec2::new(-3.6, 0.2),
            c + Vec2::new(-1., 2.8),
            c + Vec2::new(3.8, -2.4),
        ],
        Stroke::new(1.6_f32, color),
    ));
}

/// Small pill used for project tags and meta information.
pub fn chip(ui: &mut Ui, text: &str, fill: Color32, color: Color32) -> Response {
    let font = FontId::proportional(11.5);
    let galley = ui.painter().layout_no_wrap(text.into(), font, color);
    let (rect, r) = ui.allocate_exact_size(galley.size() + Vec2::new(14., 6.), Sense::hover());
    ui.painter().rect_filled(rect, 9., fill);
    ui.painter()
        .galley(rect.center() - galley.size() / 2., galley, color);
    r
}

/// Selectable pill for filter toggles.
pub fn pill(ui: &mut Ui, selected: bool, text: &str) -> Response {
    let c = t(ui);
    let font = FontId::new(
        ui.style().text_styles[&egui::TextStyle::Button].size * 0.93,
        ui.style().text_styles[&egui::TextStyle::Button]
            .family
            .clone(),
    );
    let galley =
        ui.painter()
            .layout_no_wrap(text.into(), font, if selected { ACCENT } else { c.muted });
    let (rect, r) = ui.allocate_exact_size(galley.size() + Vec2::new(20., 10.), Sense::click());
    let fill = if selected {
        c.accent_soft
    } else if r.hovered() {
        c.hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 14., fill);
    if !selected {
        ui.painter().rect_stroke(
            rect,
            14.,
            Stroke::new(1.0_f32, c.line_strong),
            StrokeKind::Inside,
        );
    }
    ui.painter()
        .galley(rect.center() - galley.size() / 2., galley, c.ink);
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Heading of a list section: icon, title and an optional count.
pub fn section_header(ui: &mut Ui, icon: Icon, title: &str, count: Option<usize>) {
    let c = t(ui);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.), Sense::hover());
        paint_icon(ui.painter(), rect, icon, c.muted);
        ui.label(RichText::new(title).size(13.).strong().color(c.ink));
        if let Some(n) = count {
            ui.label(RichText::new(n.to_string()).size(12.).color(c.faint));
        }
    });
    ui.add_space(4.);
}

/// A rounded card that groups a list.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let c = t(ui);
    egui::Frame::NONE
        .fill(c.bg)
        .stroke(Stroke::new(1.0_f32, c.line))
        .corner_radius(10.)
        .inner_margin(egui::Margin::symmetric(8, 8))
        .show(ui, add)
        .inner
}

/// One full-width list row: a color dot, the title and right-aligned meta
/// text. The row is clickable and draggable.
pub fn list_row(
    ui: &mut Ui,
    id: Id,
    selected: bool,
    dot: Option<Color32>,
    title: &str,
    meta: &str,
    height: f32,
) -> Response {
    let c = t(ui);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let r = ui.interact(rect, id, Sense::click_and_drag());
    if selected {
        ui.painter().rect_filled(rect, 7., c.accent_soft);
    } else if r.hovered() {
        ui.painter().rect_filled(rect, 7., c.hover);
    }
    let mut x = rect.left() + 10.;
    if let Some(color) = dot {
        ui.painter()
            .circle_filled(egui::pos2(x + 4., rect.center().y), 4.5, color);
        x += 16.;
    }
    let meta_galley = ui
        .painter()
        .layout_no_wrap(meta.into(), FontId::proportional(11.5), c.faint);
    let meta_w = if meta.is_empty() {
        0.
    } else {
        meta_galley.size().x + 12.
    };
    let title_galley = ui.painter().layout(
        title.into(),
        FontId::proportional(13.5),
        if selected { ACCENT } else { c.ink },
        f32::INFINITY,
    );
    let room = (rect.right() - 10. - meta_w - x).max(10.);
    let clip = egui::Rect::from_min_max(
        egui::pos2(x, rect.top()),
        egui::pos2(x + room, rect.bottom()),
    );
    ui.painter().with_clip_rect(clip).galley(
        egui::pos2(x, rect.center().y - title_galley.size().y / 2.),
        title_galley,
        c.ink,
    );
    if !meta.is_empty() {
        ui.painter().galley(
            egui::pos2(
                rect.right() - 10. - meta_galley.size().x,
                rect.center().y - meta_galley.size().y / 2.,
            ),
            meta_galley,
            c.faint,
        );
    }
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, title));
    r
}

/// A folder row in a tree: chevron, folder icon, title and a count.
/// Returns the row response and whether the chevron was clicked.
pub fn tree_row(
    ui: &mut Ui,
    id: Id,
    selected: bool,
    open: Option<bool>,
    icon: Icon,
    title: &str,
    count: usize,
) -> (Response, bool) {
    let c = t(ui);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.), Sense::hover());
    let r = ui.interact(rect, id, Sense::click_and_drag());
    if selected {
        ui.painter().rect_filled(rect, 7., c.accent_soft);
    } else if r.hovered() {
        ui.painter().rect_filled(rect, 7., c.hover);
    }
    let chevron = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 13., rect.center().y),
        Vec2::splat(18.),
    );
    if let Some(open) = open {
        paint_icon(
            ui.painter(),
            chevron.shrink(4.),
            if open {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            },
            c.muted,
        );
    }
    paint_icon(
        ui.painter(),
        egui::Rect::from_center_size(
            egui::pos2(rect.left() + 34., rect.center().y),
            Vec2::splat(15.),
        ),
        icon,
        if selected { ACCENT } else { c.muted },
    );
    let title_galley = ui.painter().layout_no_wrap(
        title.into(),
        FontId::proportional(13.5),
        if selected { ACCENT } else { c.ink },
    );
    let clip = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 48., rect.top()),
        egui::pos2(rect.right() - 34., rect.bottom()),
    );
    ui.painter().with_clip_rect(clip).galley(
        egui::pos2(
            rect.left() + 48.,
            rect.center().y - title_galley.size().y / 2.,
        ),
        title_galley,
        c.ink,
    );
    if count > 0 {
        ui.painter().text(
            egui::pos2(rect.right() - 10., rect.center().y),
            egui::Align2::RIGHT_CENTER,
            count.to_string(),
            FontId::proportional(11.5),
            c.faint,
        );
    }
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, title));
    let toggled = open.is_some()
        && r.clicked()
        && r.interact_pointer_pos()
            .is_some_and(|p| chevron.expand(4.).contains(p));
    (r, toggled)
}

/// A quiet, centered placeholder for an empty list.
pub fn empty_state(ui: &mut Ui, icon: Icon, title: &str, hint_text: &str) {
    let c = t(ui);
    ui.vertical_centered(|ui| {
        ui.add_space(18.);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(26.), Sense::hover());
        paint_icon(ui.painter(), rect, icon, c.faint);
        ui.add_space(6.);
        ui.label(RichText::new(title).size(13.).color(c.muted));
        if !hint_text.is_empty() {
            ui.label(RichText::new(hint_text).size(11.5).color(c.faint));
        }
        ui.add_space(18.);
    });
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

/// A thin full-width divider.
pub fn divider(ui: &mut Ui) {
    let c = t(ui).line;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0_f32, c),
    );
}

pub fn weekday(d: chrono::NaiveDate) -> &'static str {
    use chrono::Datelike;
    ["일", "월", "화", "수", "목", "금", "토"][d.weekday().num_days_from_sunday() as usize]
}

pub fn main_viewport() -> ViewportId {
    ViewportId::from_hash_of("memonoking_main")
}

fn raise_key(vid: ViewportId) -> Id {
    Id::new(("dialog_raise", vid))
}
/// Bring the dialog `id` (as passed to [`dialog`]) in front of other windows
/// the next time it is shown, e.g. when its button is pressed while it is
/// already open behind the main window.
pub fn raise(ctx: &egui::Context, id: impl Hash) {
    let vid = ViewportId::from_hash_of(("dialog", id));
    ctx.data_mut(|d| d.insert_temp(raise_key(vid), true));
}
/// Show `add` in a separate native window, so it is never limited to the
/// bounds of the window that opened it. Closing the window clears `open`.
pub fn dialog<R>(
    ctx: &egui::Context,
    id: impl Hash,
    title: &str,
    size: [f32; 2],
    on_top: bool,
    open: &mut bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    if !*open {
        return None;
    }
    let vid = ViewportId::from_hash_of(("dialog", id));
    let key = Id::new(("dialog_position", vid));
    let size = Vec2::from(size);
    let position = ctx.data(|d| d.get_temp::<Pos2>(key)).unwrap_or_else(|| {
        let (outer, monitor) = ctx.input(|i| (i.viewport().outer_rect, i.viewport().monitor_size));
        let area = outer
            .filter(|r| r.width() > 120. && r.min.x > -10000. && r.min.y > -10000.)
            .or_else(|| monitor.map(|m| Rect::from_min_size(Pos2::ZERO, m)))
            .unwrap_or(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280., 800.)));
        let p = area.center() - size * 0.5;
        let p = Pos2::new(p.x.max(area.left()), p.y.max(area.top()));
        ctx.data_mut(|d| d.insert_temp(key, p));
        p
    });
    let builder = ViewportBuilder::default()
        .with_title(title)
        .with_inner_size(size)
        .with_position(position)
        .with_min_inner_size([220., 120.])
        .with_taskbar(false)
        .with_window_level(if on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        });
    let mut add = Some(add);
    let mut out = None;
    let mut close = false;
    let raise = ctx.data_mut(|d| d.remove_temp::<bool>(raise_key(vid)).unwrap_or(false));
    ctx.show_viewport_immediate(vid, builder, |ctx, class| {
        if raise {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        if class == ViewportClass::Embedded {
            let mut keep = true;
            egui::Window::new(title)
                .id(Id::new(("embedded_dialog", vid)))
                .open(&mut keep)
                .default_width(size.x)
                .show(ctx, |ui| {
                    if let Some(f) = add.take() {
                        out = Some(f(ui));
                    }
                });
            close = !keep;
            return;
        }
        let fill = ctx.style().visuals.window_fill;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(fill)
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ctx, |ui| {
                if let Some(f) = add.take() {
                    out = Some(f(ui));
                }
            });
        if ctx.input(|i| i.viewport().close_requested())
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            close = true;
        }
    });
    if close {
        *open = false;
    }
    if !*open {
        ctx.data_mut(|d| d.remove::<Pos2>(key));
    }
    out
}

/// Rounded search field with a magnifier icon.
pub fn search_field(
    ui: &mut Ui,
    text: &mut String,
    id: impl Hash,
    hint: &str,
    width: f32,
) -> Response {
    let c = t(ui);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 30.), Sense::hover());
    let edit_id = Id::new(("search_field", id));
    let focused = ui.memory(|m| m.has_focus(edit_id));
    ui.painter().rect_filled(rect, 7., c.field);
    if focused {
        ui.painter().rect_stroke(
            rect,
            7.,
            Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.7)),
            StrokeKind::Inside,
        );
    }
    paint_icon(
        ui.painter(),
        Rect::from_center_size(rect.left_center() + Vec2::new(15., 0.), Vec2::splat(15.)),
        Icon::Search,
        c.muted,
    );
    let inner = Rect::from_min_max(rect.min + Vec2::new(26., 0.), rect.max - Vec2::new(6., 0.));
    ui.put(
        inner,
        egui::TextEdit::singleline(text)
            .id(edit_id)
            .frame(false)
            .vertical_align(egui::Align::Center)
            .hint_text(hint)
            .desired_width(inner.width()),
    )
}

/// Two buttons aligned right at the bottom of a dialog. Returns (confirm, cancel).
pub fn dialog_buttons(ui: &mut Ui, confirm: &str, cancel: &str) -> (bool, bool) {
    let mut out = (false, false);
    ui.add_space(8.);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        out.0 = primary(ui, confirm).clicked();
        if !cancel.is_empty() {
            out.1 = ui.button(cancel).clicked();
        }
    });
    out
}

/// Label + control on one aligned row.
pub fn field(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        let c = t(ui).muted;
        ui.add_sized([96., 26.], egui::Label::new(RichText::new(label).color(c)));
        add(ui);
    });
}

/// A compact month grid used by the date and alarm dialogs.
pub fn month_grid(ui: &mut Ui, month: &mut chrono::NaiveDate, day: &mut chrono::NaiveDate) {
    use chrono::{Datelike, Duration};
    let c = t(ui);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(month.format("%Y년 %-m월").to_string())
                .strong()
                .size(15.),
        );
        ui.add_space(4.);
        if crate::icons::icon_button(ui, Icon::ChevronLeft, "이전 달").clicked() {
            *month = (*month - Duration::days(1)).with_day(1).unwrap();
        }
        if crate::icons::icon_button(ui, Icon::ChevronRight, "다음 달").clicked() {
            *month = (*month + Duration::days(32)).with_day(1).unwrap();
        }
    });
    ui.add_space(4.);
    // The grid spans the window width; rows keep a compact height.
    let cell_w = (ui.available_width() / 7.).max(34.);
    let cell_h = 34.;
    let (head, _) = ui.allocate_exact_size(Vec2::new(cell_w * 7., 20.), Sense::hover());
    for (i, s) in ["일", "월", "화", "수", "목", "금", "토"]
        .iter()
        .enumerate()
    {
        ui.painter().text(
            Pos2::new(head.left() + cell_w * (i as f32 + 0.5), head.center().y),
            egui::Align2::CENTER_CENTER,
            *s,
            FontId::proportional(11.5),
            match i {
                0 => ACCENT,
                6 => BLUE,
                _ => c.faint,
            },
        );
    }
    let offset = month.weekday().num_days_from_sunday() as i64;
    let today = chrono::Local::now().date_naive();
    let (grid, _) = ui.allocate_exact_size(Vec2::new(cell_w * 7., cell_h * 6.), Sense::hover());
    for slot in 0..42 {
        let d = *month + Duration::days(slot - offset);
        if d.month() != month.month() {
            continue;
        }
        let r = Rect::from_min_size(
            grid.min + Vec2::new((slot % 7) as f32 * cell_w, (slot / 7) as f32 * cell_h),
            Vec2::new(cell_w, cell_h),
        )
        .shrink(2.);
        let resp = ui
            .interact(r, ui.id().with(("month_grid", d)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let selected = *day == d;
        if selected {
            ui.painter().circle_filled(r.center(), 14., ACCENT);
        } else if resp.hovered() {
            ui.painter().circle_filled(r.center(), 14., c.hover);
        } else if d == today {
            ui.painter()
                .circle_stroke(r.center(), 13.5, Stroke::new(1.0_f32, ACCENT));
        }
        let wd = d.weekday().num_days_from_sunday();
        ui.painter().text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            d.day().to_string(),
            FontId::proportional(13.),
            if selected {
                Color32::WHITE
            } else if wd == 0 {
                ACCENT
            } else if wd == 6 {
                BLUE
            } else {
                c.ink
            },
        );
        if resp.clicked() {
            *day = d;
        }
    }
}

/// Hour and minute steppers in one row.
pub fn time_row(ui: &mut Ui, hour: &mut u32, minute: &mut u32) {
    ui.horizontal(|ui| {
        let c = t(ui).muted;
        ui.label(RichText::new("시간").color(c));
        ui.add_space(8.);
        time_input(ui, "time_row", hour, minute);
    });
}

/// Step a time by `delta` minutes, wrapping around midnight.
pub fn step_minutes(hour: u32, minute: u32, delta: i32) -> (u32, u32) {
    let total = (hour as i32 * 60 + minute as i32 + delta).rem_euclid(24 * 60);
    ((total / 60) as u32, (total % 60) as u32)
}

/// Hour and minute boxes. A click opens a picker of whole hours and
/// 5-minute steps; the arrows beside the minutes step by one minute.
pub fn time_input(ui: &mut Ui, id: impl Hash, hour: &mut u32, minute: &mut u32) {
    let c = t(ui);
    let popup = ui.make_persistent_id(("time_input", id));
    let height = 28.;
    ui.spacing_mut().item_spacing.x = 0.;
    let field = |ui: &mut Ui, text: String| {
        let (rect, r) = ui.allocate_exact_size(Vec2::new(38., height), Sense::click());
        let fill = if r.hovered() { c.hover } else { c.field };
        ui.painter().rect_filled(rect, 6., fill);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            FontId::monospace(14.),
            c.ink,
        );
        r.on_hover_cursor(egui::CursorIcon::PointingHand)
    };
    let h = field(ui, format!("{:02}", *hour));
    ui.add_sized([12., height], egui::Label::new(RichText::new(":").strong()));
    let m = field(ui, format!("{:02}", *minute));
    // Two half-height arrows stuck to the minute box: ±1 minute.
    let (spin, _) = ui.allocate_exact_size(Vec2::new(18., height), Sense::hover());
    for (k, up) in [(0., true), (1., false)] {
        let r = Rect::from_min_size(
            spin.min + Vec2::new(1., k * height / 2.),
            Vec2::new(17., height / 2.),
        );
        let resp = ui
            .interact(r, popup.with(("spin", up)), Sense::click())
            .on_hover_text(if up {
                "1분 늘리기"
            } else {
                "1분 줄이기"
            });
        ui.painter().rect_filled(
            r.shrink(0.5),
            3.,
            if resp.hovered() { c.hover } else { c.field },
        );
        crate::icons::paint_icon(
            ui.painter(),
            Rect::from_center_size(r.center(), Vec2::splat(9.)),
            if up {
                Icon::ChevronUp
            } else {
                Icon::ChevronDown
            },
            c.muted,
        );
        if resp.clicked() {
            (*hour, *minute) = step_minutes(*hour, *minute, if up { 1 } else { -1 });
        }
    }
    if h.clicked() || m.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup));
    }
    let anchor = h.union(m);
    egui::popup_below_widget(
        ui,
        popup,
        &anchor,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_width(6. * 38.);
            ui.spacing_mut().item_spacing = Vec2::new(2., 2.);
            caption(ui, "시");
            let mut close = false;
            egui::Grid::new(popup.with("hours"))
                .spacing([2., 2.])
                .show(ui, |ui| {
                    for v in 0..24u32 {
                        if pick_cell(ui, v == *hour, format!("{v:02}")).clicked() {
                            *hour = v;
                        }
                        if v % 6 == 5 {
                            ui.end_row();
                        }
                    }
                });
            ui.add_space(6.);
            caption(ui, "분");
            egui::Grid::new(popup.with("minutes"))
                .spacing([2., 2.])
                .show(ui, |ui| {
                    for k in 0..12u32 {
                        let v = k * 5;
                        if pick_cell(ui, v == *minute, format!("{v:02}")).clicked() {
                            *minute = v;
                            close = true;
                        }
                        if k % 6 == 5 {
                            ui.end_row();
                        }
                    }
                });
            if close {
                ui.memory_mut(|mem| mem.close_popup());
            }
        },
    );
}
fn pick_cell(ui: &mut Ui, selected: bool, text: String) -> Response {
    let c = t(ui);
    let (rect, r) = ui.allocate_exact_size(Vec2::new(36., 26.), Sense::click());
    let fill = if selected {
        ACCENT
    } else if r.hovered() {
        c.hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6., fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        FontId::monospace(13.),
        if selected { Color32::WHITE } else { c.ink },
    );
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}
