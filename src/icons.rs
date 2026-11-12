//! Small vector icons shared by native memo windows and the main application.
use eframe::egui::{
    self, Color32, CursorIcon, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
    WidgetInfo, WidgetType,
};

// The palette is intentionally reusable even when a given screen uses only a subset.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Close,
    Help,
    Edit,
    Plus,
    Fold,
    Minus,
    Sticky,
    Search,
    Calendar,
    Folder,
    Settings,
    ChevronLeft,
    ChevronRight,
    Pin,
    Alarm,
    Palette,
    Lock,
    Maximize,
    Restore,
    Star,
    Opacity,
    Menu,
    Sidebar,
    Detach,
    StickyPlus,
    Filter,
    PlusCircle,
    Home,
    Today,
    Upcoming,
    Notes,
    Book,
    Grid,
    More,
    Trash,
    Arrange,
    ChevronDown,
    ChevronUp,
    Check,
    Eye,
    Bell,
    Restore2,
    Stand,
    SplitView,
}

/// An icon-only control with a consistent hit area, keyboard focus and a spoken label.
pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), tooltip));
    if response.hovered() || response.has_focus() {
        let fill = if ui.visuals().dark_mode {
            Color32::from_white_alpha(18)
        } else {
            Color32::from_black_alpha(10)
        };
        ui.painter().rect_filled(rect, 6.0, fill);
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            5.0,
            Stroke::new(1.0_f32, ui.visuals().selection.stroke.color),
            StrokeKind::Inside,
        );
    }
    let color = if ui.is_enabled() {
        ui.visuals()
            .text_color()
            .gamma_multiply(if response.hovered() { 1.0 } else { 0.78 })
    } else {
        ui.visuals().weak_text_color()
    };
    paint_icon(ui.painter(), rect.shrink(5.5), icon, color);
    response
        .on_hover_cursor(if ui.is_enabled() {
            CursorIcon::PointingHand
        } else {
            CursorIcon::Default
        })
        .on_hover_text(tooltip)
}

/// Icon button that shows a persistent "on" state.
pub fn toggle_icon(ui: &mut Ui, icon: Icon, tooltip: &str, on: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, tooltip));
    if on {
        ui.painter()
            .rect_filled(rect, 6.0, ui.visuals().selection.bg_fill);
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect,
            6.0,
            if ui.visuals().dark_mode {
                Color32::from_white_alpha(18)
            } else {
                Color32::from_black_alpha(10)
            },
        );
    }
    let color = if on {
        crate::ui::ACCENT
    } else {
        ui.visuals()
            .text_color()
            .gamma_multiply(if response.hovered() { 1.0 } else { 0.7 })
    };
    paint_icon(ui.painter(), rect.shrink(5.5), icon, color);
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

/// A 22 px icon button for dense rows.
pub fn small_icon(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tooltip));
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            5.0,
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
        .gamma_multiply(if response.hovered() { 0.95 } else { 0.55 });
    paint_icon(ui.painter(), rect.shrink(4.0), icon, color);
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

pub fn text_button(ui: &mut Ui, icon: Icon, label: &str, tooltip: &str) -> Response {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.into(), font, ui.visuals().text_color());
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(galley.size().x + 43., 30.), Sense::click());
    let fill = if response.hovered() {
        ui.visuals().widgets.hovered.weak_bg_fill
    } else {
        ui.visuals().widgets.inactive.weak_bg_fill
    };
    ui.painter().rect_filled(rect, 6., fill);
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            6.,
            Stroke::new(1.0_f32, ui.visuals().selection.stroke.color),
            StrokeKind::Inside,
        );
    }
    paint_icon(
        ui.painter(),
        Rect::from_center_size(
            Pos2::new(rect.left() + 16., rect.center().y),
            Vec2::splat(17.),
        ),
        icon,
        ui.visuals().weak_text_color(),
    );
    ui.painter().galley(
        Pos2::new(rect.left() + 31., rect.center().y - galley.size().y / 2.),
        galley,
        ui.visuals().text_color(),
    );
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), label));
    response
        .on_hover_text(tooltip)
        .on_hover_cursor(CursorIcon::PointingHand)
}

/// Draw an icon into any rectangle. Geometry uses a 24 by 24 design grid.
/// Strokes get round caps and joins (small discs at every vertex) so the
/// line icons read cleanly at 12–20 px; everything is vector, no images.
pub fn paint_icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let scale = rect.width().min(rect.height()) / 24.0;
    let origin = rect.center() - Vec2::splat(12.0 * scale);
    let p = |x: f32, y: f32| origin + Vec2::new(x, y) * scale;
    let width = (1.7 * scale).max(1.0);
    let stroke = Stroke::new(width, color);
    let cap = |q: Pos2| {
        painter.circle_filled(q, width / 2., color);
    };
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
        cap(p(a.0, a.1));
        cap(p(b.0, b.1));
    };
    let path = |points: &[(f32, f32)]| {
        let pts: Vec<Pos2> = points.iter().map(|&(x, y)| p(x, y)).collect();
        for &q in &pts {
            cap(q);
        }
        painter.add(Shape::line(pts, stroke));
    };
    let closed = |points: &[(f32, f32)]| {
        let pts: Vec<Pos2> = points.iter().map(|&(x, y)| p(x, y)).collect();
        for &q in &pts {
            cap(q);
        }
        painter.add(Shape::closed_line(pts, stroke));
    };
    let arc = |c: (f32, f32), r: f32, from: f32, to: f32| {
        let n = 18;
        let pts: Vec<Pos2> = (0..=n)
            .map(|k| {
                let a = (from + (to - from) * k as f32 / n as f32).to_radians();
                p(c.0 + r * a.cos(), c.1 + r * a.sin())
            })
            .collect();
        cap(pts[0]);
        cap(pts[n]);
        painter.add(Shape::line(pts, stroke));
    };
    let ring = |c: (f32, f32), r: f32| {
        painter.circle_stroke(p(c.0, c.1), r * scale, stroke);
    };
    let dot = |c: (f32, f32), r: f32| {
        painter.circle_filled(p(c.0, c.1), r * scale, color);
    };
    let rounded = |a: (f32, f32), b: (f32, f32), radius: f32| {
        painter.rect_stroke(
            Rect::from_min_max(p(a.0, a.1), p(b.0, b.1)),
            radius * scale,
            stroke,
            StrokeKind::Middle,
        );
    };
    match icon {
        Icon::Close => {
            line((6.5, 6.5), (17.5, 17.5));
            line((17.5, 6.5), (6.5, 17.5));
        }
        Icon::Plus => {
            line((5.5, 12.), (18.5, 12.));
            line((12., 5.5), (12., 18.5));
        }
        Icon::Minus => line((5.5, 12.), (18.5, 12.)),
        Icon::Check => path(&[(5., 12.5), (9.8, 17.), (19., 7.)]),
        Icon::ChevronDown => path(&[(6., 9.5), (12., 15.5), (18., 9.5)]),
        Icon::ChevronUp => path(&[(6., 14.5), (12., 8.5), (18., 14.5)]),
        Icon::ChevronLeft => path(&[(14.5, 6.), (8.5, 12.), (14.5, 18.)]),
        Icon::ChevronRight => path(&[(9.5, 6.), (15.5, 12.), (9.5, 18.)]),
        Icon::Help => {
            ring((12., 12.), 9.);
            arc((12., 9.6), 2.6, 190., 395.);
            path(&[(13.9, 11.4), (12., 12.8), (12., 13.8)]);
            dot((12., 16.9), 1.05);
        }
        Icon::Edit => {
            closed(&[
                (5., 19.),
                (5.8, 15.2),
                (15.6, 5.4),
                (18.6, 8.4),
                (8.8, 18.2),
            ]);
            line((13.6, 7.4), (16.6, 10.4));
        }
        Icon::Fold => {
            path(&[(7., 8.), (12., 13.), (17., 8.)]);
            line((6., 18.), (18., 18.));
        }
        Icon::Sticky => {
            closed(&[(5., 4.5), (19., 4.5), (19., 14.), (14., 19.5), (5., 19.5)]);
            path(&[(14., 19.5), (14., 14.), (19., 14.)]);
        }
        Icon::StickyPlus => {
            closed(&[(5., 4.5), (19., 4.5), (19., 14.), (14., 19.5), (5., 19.5)]);
            path(&[(14., 19.5), (14., 14.), (19., 14.)]);
            line((9., 9.5), (14., 9.5));
            line((11.5, 7.), (11.5, 12.));
        }
        Icon::Search => {
            ring((10.8, 10.8), 6.3);
            line((15.5, 15.5), (19.5, 19.5));
        }
        Icon::Calendar | Icon::Upcoming | Icon::Today => {
            rounded((4., 5.5), (20., 20.), 2.5);
            line((8.5, 3.5), (8.5, 7.));
            line((15.5, 3.5), (15.5, 7.));
            match icon {
                Icon::Today => {
                    painter.text(
                        p(12., 14.3),
                        egui::Align2::CENTER_CENTER,
                        chrono::Local::now().format("%-d").to_string(),
                        egui::FontId::proportional(8.5 * scale),
                        color,
                    );
                }
                Icon::Upcoming => {
                    line((4., 10.), (20., 10.));
                    path(&[(10.5, 12.5), (13.5, 15.), (10.5, 17.5)]);
                }
                _ => {
                    line((4., 10.), (20., 10.));
                    for (x, y) in [(8.5, 13.8), (12., 13.8), (15.5, 13.8), (8.5, 17.)] {
                        dot((x, y), 0.95);
                    }
                }
            }
        }
        Icon::Folder => {
            closed(&[
                (3.5, 6.5),
                (3.5, 18.5),
                (20.5, 18.5),
                (20.5, 8.5),
                (12., 8.5),
                (10., 5.),
                (4.5, 5.),
            ]);
        }
        Icon::Settings => {
            ring((12., 12.), 2.8);
            ring((12., 12.), 6.6);
            for k in 0..8 {
                let a = (k as f32 * 45.).to_radians();
                line(
                    (12. + 6.6 * a.cos(), 12. + 6.6 * a.sin()),
                    (12. + 8.9 * a.cos(), 12. + 8.9 * a.sin()),
                );
            }
        }
        Icon::Stand => {
            rounded((4.5, 3.5), (19.5, 15.), 2.);
            line((4.5, 7.), (19.5, 7.));
            line((9., 15.), (6.5, 20.5));
            line((15., 15.), (17.5, 20.5));
            line((12., 15.), (12., 20.5));
        }
        Icon::Pin => {
            closed(&[
                (9., 4.),
                (15., 4.),
                (14.2, 9.),
                (17., 12.5),
                (7., 12.5),
                (9.8, 9.),
            ]);
            line((12., 12.5), (12., 20.));
        }
        Icon::Alarm => {
            ring((12., 13.), 7.3);
            path(&[(12., 9.3), (12., 13.), (14.6, 14.6)]);
            line((4.2, 6.2), (6.8, 3.8));
            line((19.8, 6.2), (17.2, 3.8));
        }
        Icon::Palette => {
            ring((12., 12.), 8.8);
            for (x, y) in [(8.3, 9.), (12., 7.2), (15.7, 9.), (8.6, 13.4)] {
                dot((x, y), 1.25);
            }
            ring((14.6, 14.6), 1.9);
        }
        Icon::Lock => {
            rounded((5., 11.), (19., 20.), 2.2);
            arc((12., 8.), 3.8, 180., 360.);
            line((8.2, 8.), (8.2, 11.));
            line((15.8, 8.), (15.8, 11.));
            dot((12., 15.5), 1.2);
        }
        Icon::Maximize => rounded((5., 5.), (19., 19.), 2.2),
        Icon::Restore => {
            rounded((5., 9.), (15., 19.), 2.);
            path(&[
                (9., 6.5),
                (9.5, 5.),
                (17., 5.),
                (19., 7.),
                (19., 14.5),
                (17.5, 15.),
            ]);
        }
        Icon::Star => closed(&[
            (12., 3.8),
            (14.5, 8.9),
            (20., 9.6),
            (16., 13.4),
            (17., 18.9),
            (12., 16.3),
            (7., 18.9),
            (8., 13.4),
            (4., 9.6),
            (9.5, 8.9),
        ]),
        Icon::Opacity => {
            ring((12., 12.), 8.3);
            let half: Vec<Pos2> = (0..=18)
                .map(|k| {
                    let a = (90. + 180. * k as f32 / 18.).to_radians();
                    p(12. + 8.3 * a.cos(), 12. + 8.3 * a.sin())
                })
                .collect();
            painter.add(Shape::convex_polygon(half, color, Stroke::NONE));
        }
        Icon::Menu => {
            line((4.5, 7.), (19.5, 7.));
            line((4.5, 12.), (19.5, 12.));
            line((4.5, 17.), (19.5, 17.));
        }
        Icon::Sidebar => {
            rounded((3.5, 4.5), (20.5, 19.5), 2.5);
            line((9.5, 4.5), (9.5, 19.5));
        }
        Icon::SplitView => {
            rounded((3.5, 4.5), (20.5, 19.5), 2.5);
            line((14.5, 4.5), (14.5, 19.5));
        }
        Icon::Detach => {
            path(&[(14., 4.), (20., 4.), (20., 10.)]);
            line((20., 4.), (11.5, 12.5));
            path(&[
                (17.5, 13.5),
                (17.5, 18.),
                (16., 19.5),
                (6., 19.5),
                (4.5, 18.),
                (4.5, 8.),
                (6., 6.5),
                (10.5, 6.5),
            ]);
        }
        Icon::Filter => {
            line((4.5, 7.), (19.5, 7.));
            line((7.5, 12.), (16.5, 12.));
            line((10.5, 17.), (13.5, 17.));
        }
        Icon::PlusCircle => {
            painter.circle_filled(p(12., 12.), 9.5 * scale, crate::ui::ACCENT);
            let s = Stroke::new((1.8 * scale).max(1.0), Color32::WHITE);
            painter.line_segment([p(8., 12.), p(16., 12.)], s);
            painter.line_segment([p(12., 8.), p(12., 16.)], s);
        }
        Icon::Home => {
            path(&[(4., 11.), (12., 4.5), (20., 11.)]);
            path(&[(6.5, 9.5), (6.5, 19.5), (17.5, 19.5), (17.5, 9.5)]);
            path(&[(10., 19.5), (10., 14.5), (14., 14.5), (14., 19.5)]);
        }
        Icon::Notes => {
            rounded((5., 3.5), (19., 20.5), 2.5);
            line((8.5, 8.5), (15.5, 8.5));
            line((8.5, 12.), (15.5, 12.));
            line((8.5, 15.5), (12.5, 15.5));
        }
        Icon::Book => {
            closed(&[
                (12., 6.5),
                (8.5, 5.),
                (4., 5.),
                (4., 18.),
                (8.5, 18.),
                (12., 19.5),
            ]);
            closed(&[
                (12., 6.5),
                (15.5, 5.),
                (20., 5.),
                (20., 18.),
                (15.5, 18.),
                (12., 19.5),
            ]);
        }
        Icon::Grid => {
            for (x, y) in [(4.5, 4.5), (13., 4.5), (4.5, 13.), (13., 13.)] {
                rounded((x, y), (x + 6.5, y + 6.5), 1.6);
            }
        }
        Icon::More => {
            for x in [6., 12., 18.] {
                dot((x, 12.), 1.55);
            }
        }
        Icon::Trash => {
            line((4.5, 7.), (19.5, 7.));
            path(&[(9.5, 7.), (9.5, 4.5), (14.5, 4.5), (14.5, 7.)]);
            path(&[(6.5, 7.), (7.3, 19.5), (16.7, 19.5), (17.5, 7.)]);
            line((10.3, 11.), (10.3, 16.));
            line((13.7, 11.), (13.7, 16.));
        }
        Icon::Eye => {
            let lid = |sign: f32| -> Vec<(f32, f32)> {
                (0..=16)
                    .map(|k| {
                        let t = k as f32 / 16.;
                        let x = 2.5 + 19. * t;
                        (x, 12. + sign * 6.2 * (std::f32::consts::PI * t).sin())
                    })
                    .collect()
            };
            path(&lid(-1.));
            path(&lid(1.));
            ring((12., 12.), 2.9);
        }
        Icon::Bell => {
            path(&[
                (6., 16.5),
                (6., 11.),
                (7.2, 7.4),
                (9.6, 5.4),
                (12., 4.8),
                (14.4, 5.4),
                (16.8, 7.4),
                (18., 11.),
                (18., 16.5),
            ]);
            line((4.5, 16.5), (19.5, 16.5));
            arc((12., 18.6), 2., 0., 180.);
        }
        Icon::Restore2 => {
            arc((12.5, 12.5), 7.5, 200., 520.);
            path(&[(4.2, 5.5), (4.6, 10.), (9., 9.6)]);
        }
        Icon::Arrange => {
            rounded((4., 4.5), (10.5, 10.5), 1.6);
            rounded((4., 13.5), (10.5, 19.5), 1.6);
            line((14., 7.5), (20., 7.5));
            line((14., 16.5), (20., 16.5));
        }
    }
}
