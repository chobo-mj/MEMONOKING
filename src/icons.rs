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
}

/// An icon-only control with a consistent hit area, keyboard focus and a spoken label.
pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), tooltip));
    if response.hovered() || response.has_focus() {
        let fill = if ui.visuals().dark_mode {
            Color32::from_white_alpha(18)
        } else {
            Color32::from_black_alpha(12)
        };
        ui.painter().rect_filled(rect, 5.0, fill);
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
    paint_icon(ui.painter(), rect.shrink(4.0), icon, color);
    response
        .on_hover_cursor(if ui.is_enabled() {
            CursorIcon::PointingHand
        } else {
            CursorIcon::Default
        })
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
pub fn paint_icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let scale = rect.width().min(rect.height()) / 24.0;
    let origin = rect.center() - Vec2::splat(12.0 * scale);
    let p = |x: f32, y: f32| origin + Vec2::new(x, y) * scale;
    let stroke = Stroke::new((1.6 * scale).max(1.0), color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
    };
    let path = |points: &[(f32, f32)]| {
        painter.add(Shape::line(
            points.iter().map(|&(x, y)| p(x, y)).collect(),
            stroke,
        ));
    };
    let box_outline = |a: (f32, f32), b: (f32, f32), radius: f32| {
        painter.rect_stroke(
            Rect::from_min_max(p(a.0, a.1), p(b.0, b.1)),
            radius * scale,
            stroke,
            StrokeKind::Inside,
        );
    };
    match icon {
        Icon::Maximize => box_outline((4., 4.), (20., 20.), 1.5),
        Icon::Restore => {
            box_outline((4., 8.), (16., 20.), 1.);
            path(&[(8., 8.), (8., 4.), (20., 4.), (20., 16.), (16., 16.)]);
        }
        Icon::Star => path(&[
            (12., 3.),
            (14.8, 8.7),
            (21., 9.5),
            (16.5, 14.),
            (17.7, 20.5),
            (12., 17.4),
            (6.3, 20.5),
            (7.5, 14.),
            (3., 9.5),
            (9.2, 8.7),
            (12., 3.),
        ]),
        Icon::Opacity => {
            painter.circle_stroke(p(12., 12.), 9. * scale, stroke);
            path(&[(12., 3.), (12., 21.)]);
            for x in [14., 16., 18.] {
                line((x, 6. + (x - 14.) * 0.8), (x, 18. - (x - 14.) * 0.8));
            }
        }
        Icon::Menu => {
            line((4., 6.), (20., 6.));
            line((4., 12.), (20., 12.));
            line((4., 18.), (20., 18.));
        }
        Icon::Alarm => {
            painter.circle_stroke(p(12., 13.), 7. * scale, stroke);
            line((12., 8.), (12., 13.));
            line((12., 13.), (16., 15.));
            line((4., 5.), (7., 3.));
            line((17., 3.), (20., 5.));
            line((7., 20.), (5., 22.));
            line((17., 20.), (19., 22.));
        }
        Icon::Palette => {
            painter.circle_stroke(p(12., 12.), 9. * scale, stroke);
            for (x, y) in [(8., 7.), (15., 7.), (7., 13.), (16., 14.)] {
                painter.circle_filled(p(x, y), 1.3 * scale, color);
            }
        }
        Icon::Lock => {
            box_outline((5., 10.), (19., 21.), 2.);
            path(&[
                (8., 10.),
                (8., 6.),
                (10., 3.),
                (14., 3.),
                (16., 6.),
                (16., 10.),
            ]);
            line((12., 14.), (12., 17.));
        }
        Icon::Close => {
            line((6.5, 6.5), (17.5, 17.5));
            line((17.5, 6.5), (6.5, 17.5));
        }
        Icon::Help => {
            painter.circle_stroke(p(12.0, 12.0), 9.0 * scale, stroke);
            path(&[
                (8.7, 8.0),
                (9.7, 6.8),
                (13.0, 6.8),
                (15.2, 8.3),
                (15.2, 10.0),
                (12.0, 12.3),
                (12.0, 13.5),
            ]);
            painter.circle_filled(p(12.0, 17.0), 0.9 * scale, color);
        }
        Icon::Edit => {
            path(&[
                (4.5, 19.5),
                (6.0, 14.3),
                (16.3, 4.0),
                (20.0, 7.7),
                (9.7, 18.0),
                (4.5, 19.5),
            ]);
            line((14.8, 5.5), (18.5, 9.2));
            line((6.0, 14.3), (9.7, 18.0));
        }
        Icon::Plus => {
            line((5.0, 12.0), (19.0, 12.0));
            line((12.0, 5.0), (12.0, 19.0));
        }
        Icon::Minus => line((5.0, 12.0), (19.0, 12.0)),
        Icon::Fold => {
            path(&[(6.0, 8.0), (12.0, 14.0), (18.0, 8.0)]);
            line((6.0, 19.0), (18.0, 19.0));
        }
        Icon::Sticky => {
            path(&[
                (4.0, 3.0),
                (20.0, 3.0),
                (20.0, 15.0),
                (14.0, 21.0),
                (4.0, 21.0),
                (4.0, 3.0),
            ]);
            path(&[(14.0, 21.0), (14.0, 15.0), (20.0, 15.0)]);
            line((8.0, 8.0), (16.0, 8.0));
            line((8.0, 11.5), (14.0, 11.5));
        }
        Icon::Search => {
            painter.circle_stroke(p(10.0, 10.0), 5.7 * scale, stroke);
            line((14.3, 14.3), (20.0, 20.0));
        }
        Icon::Calendar => {
            box_outline((3.0, 5.0), (21.0, 21.0), 2.0);
            line((3.0, 10.0), (21.0, 10.0));
            line((8.0, 3.0), (8.0, 7.5));
            line((16.0, 3.0), (16.0, 7.5));
            for (x, y) in [(8.0, 14.0), (14.0, 14.0), (8.0, 17.5)] {
                painter.rect_filled(
                    Rect::from_center_size(p(x, y), Vec2::splat(1.9 * scale)),
                    0.3,
                    color,
                );
            }
        }
        Icon::Folder => {
            path(&[
                (3.0, 7.0),
                (3.0, 20.0),
                (21.0, 20.0),
                (21.0, 7.0),
                (12.0, 7.0),
                (10.0, 4.0),
                (3.0, 4.0),
                (3.0, 7.0),
            ]);
        }
        Icon::Settings => {
            let mut points: Vec<Pos2> = (0..32)
                .map(|i| {
                    let angle = i as f32 * std::f32::consts::TAU / 32.0;
                    let radius = if i % 4 < 2 { 9.3 } else { 7.1 };
                    p(12.0 + angle.cos() * radius, 12.0 + angle.sin() * radius)
                })
                .collect();
            points.push(points[0]);
            painter.add(Shape::line(points, stroke));
            painter.circle_stroke(p(12.0, 12.0), 3.0 * scale, stroke);
        }
        Icon::ChevronLeft => path(&[(14.5, 5.5), (8.0, 12.0), (14.5, 18.5)]),
        Icon::ChevronRight => path(&[(9.5, 5.5), (16.0, 12.0), (9.5, 18.5)]),
        Icon::Pin => {
            path(&[
                (8.0, 4.0),
                (16.0, 4.0),
                (15.0, 8.0),
                (18.0, 12.0),
                (6.0, 12.0),
                (9.0, 8.0),
                (8.0, 4.0),
            ]);
            line((12.0, 12.0), (12.0, 21.0));
        }
    }
}
