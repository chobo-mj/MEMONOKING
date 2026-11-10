//! Let Windows own live window movement. Geometry requests are only transitions
//! or a single Shift-square correction after the native resize has finished.
use eframe::egui::{self, Pos2, Rect, Vec2};

#[derive(Default)]
pub struct State {
    command_time: Option<f64>,
    square: Option<Square>,
}
struct Square {
    left: bool,
    top: bool,
    fixed: Pos2,
    started: f64,
    released: Option<f64>,
    last_rect: Rect,
    last_time: f64,
    stable: u8,
}
impl State {
    pub fn start_drag(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        if self.command_time != Some(now) {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            self.command_time = Some(now);
        }
    }
    pub fn input(
        &mut self,
        ctx: &egui::Context,
        horizontal: bool,
        square_mode: bool,
        enabled: bool,
    ) {
        if !enabled {
            return;
        }
        let Some(inner) = ctx.input(|i| i.viewport().inner_rect) else {
            return;
        };
        let outer = ctx.input(|i| i.viewport().outer_rect).unwrap_or(inner);
        let now = ctx.input(|i| i.time);
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers,
                } => {
                    if let Some(direction) = direction(pos, inner.size(), horizontal) {
                        if self.command_time != Some(now) {
                            self.command_time = Some(now);
                            if square_mode && modifiers.shift {
                                let left = pos.x < inner.width() / 2.;
                                let top = pos.y < inner.height() / 2.;
                                self.square = Some(Square {
                                    left,
                                    top,
                                    fixed: Pos2::new(
                                        if left { outer.right() } else { outer.left() },
                                        if top { outer.bottom() } else { outer.top() },
                                    ),
                                    started: now,
                                    released: None,
                                    last_rect: outer,
                                    last_time: now,
                                    stable: 0,
                                });
                            }
                            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
                        }
                    }
                }
                egui::Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    ..
                } => {
                    if let Some(square) = &mut self.square {
                        square.released = Some(now);
                        square.stable = 0;
                    }
                }
                _ => {}
            }
        }
        if let Some((position, size)) =
            self.observe(outer, now, ctx.input(|i| i.pointer.primary_down()))
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }
        if self.square.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(35));
        }
        if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
            if let Some(dir) = direction(pos, inner.size(), horizontal) {
                ctx.set_cursor_icon(match dir {
                    egui::ResizeDirection::East | egui::ResizeDirection::West => {
                        egui::CursorIcon::ResizeHorizontal
                    }
                    egui::ResizeDirection::North | egui::ResizeDirection::South => {
                        egui::CursorIcon::ResizeVertical
                    }
                    egui::ResizeDirection::NorthWest | egui::ResizeDirection::SouthEast => {
                        egui::CursorIcon::ResizeNwSe
                    }
                    _ => egui::CursorIcon::ResizeNeSw,
                });
            }
        }
    }
    fn observe(&mut self, rect: Rect, now: f64, down: bool) -> Option<(Pos2, Vec2)> {
        let pending = self.square.as_mut()?;
        if pending.released.is_none() && !down && now - pending.started > 0.15 {
            pending.released = Some(now);
        }
        if now > pending.last_time {
            pending.stable = if pending.last_rect.min.distance(rect.min) < 0.5
                && (pending.last_rect.size() - rect.size()).length() < 0.5
            {
                pending.stable.saturating_add(1)
            } else {
                0
            };
            pending.last_rect = rect;
            pending.last_time = now;
        }
        if down || pending.stable < 2 || pending.released.is_none_or(|time| now - time < 0.06) {
            return None;
        }
        let pending = self.square.take().unwrap();
        let side = rect.width().max(rect.height()).clamp(24., 360.);
        Some((
            Pos2::new(
                if pending.left {
                    pending.fixed.x - side
                } else {
                    pending.fixed.x
                },
                if pending.top {
                    pending.fixed.y - side
                } else {
                    pending.fixed.y
                },
            ),
            Vec2::splat(side),
        ))
    }
    pub fn cancel(&mut self) {
        self.square = None;
    }
}
pub fn direction(pos: Pos2, size: Vec2, horizontal: bool) -> Option<egui::ResizeDirection> {
    let left = pos.x <= 7.;
    let right = pos.x >= size.x - 7.;
    let top = pos.y <= 7.;
    let bottom = pos.y >= size.y - 7.;
    if horizontal {
        return if left {
            Some(egui::ResizeDirection::West)
        } else if right {
            Some(egui::ResizeDirection::East)
        } else {
            None
        };
    }
    use egui::ResizeDirection::*;
    match (left, right, top, bottom) {
        (true, _, true, _) => Some(NorthWest),
        (_, true, true, _) => Some(NorthEast),
        (true, _, _, true) => Some(SouthWest),
        (_, true, _, true) => Some(SouthEast),
        (true, _, _, _) => Some(West),
        (_, true, _, _) => Some(East),
        (_, _, true, _) => Some(North),
        (_, _, _, true) => Some(South),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edges_choose_native_directions_and_middle_is_not_resize() {
        assert_eq!(
            direction(Pos2::new(2., 2.), Vec2::splat(360.), false),
            Some(egui::ResizeDirection::NorthWest)
        );
        assert_eq!(
            direction(Pos2::new(358., 358.), Vec2::splat(360.), false),
            Some(egui::ResizeDirection::SouthEast)
        );
        assert_eq!(
            direction(Pos2::new(2., 2.), Vec2::new(300., 44.), true),
            Some(egui::ResizeDirection::West)
        );
        assert_eq!(
            direction(Pos2::new(100., 100.), Vec2::splat(360.), false),
            None
        );
    }
    #[test]
    fn live_bounds_never_produce_geometry_commands_and_square_finishes_once() {
        let initial = Rect::from_min_size(Pos2::new(100., 100.), Vec2::new(100., 80.));
        let mut s = State {
            square: Some(Square {
                left: true,
                top: true,
                fixed: initial.max,
                started: 0.,
                released: None,
                last_rect: initial,
                last_time: 0.,
                stable: 0,
            }),
            ..Default::default()
        };
        for i in 1..10 {
            assert!(s
                .observe(
                    Rect::from_min_size(
                        Pos2::new(100. - i as f32, 100.),
                        Vec2::new(100. + i as f32, 80.)
                    ),
                    i as f64 * 0.03,
                    true
                )
                .is_none());
        }
        let final_rect = Rect::from_min_size(Pos2::new(90., 100.), Vec2::new(110., 80.));
        assert!(s.observe(final_rect, 0.4, false).is_none());
        assert!(s.observe(final_rect, 0.44, false).is_none());
        let (position, size) = s.observe(final_rect, 0.49, false).unwrap();
        assert_eq!(size, Vec2::splat(110.));
        assert_eq!(position + size, initial.max);
        assert!(s.observe(final_rect, 0.6, false).is_none());
    }
}
