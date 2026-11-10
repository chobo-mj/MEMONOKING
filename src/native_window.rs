//! Let Windows own live window movement. Geometry requests are only transitions
//! or a single correction after a completed gesture. Native motion is observed,
//! never fed back as live position/size requests.
use eframe::egui::{self, Pos2, Rect, Vec2};

#[derive(Default)]
pub struct State {
    command_time: Option<f64>,
    square: Option<Square>,
    attempt: Option<Attempt>,
    moving: Option<Attempt>,
    press: Option<(Rect, Pos2)>,
}
struct Attempt {
    bounds: Rect,
    origin: Pos2,
    end: Pos2,
    shift: bool,
    released: Option<f64>,
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
        if self.command_time == Some(now) || self.moving.is_some() {
            return;
        }
        let (down, latest, maximized) = ctx.input(|i| {
            (
                i.pointer.primary_down(),
                i.pointer.latest_pos(),
                i.viewport().maximized.unwrap_or(false),
            )
        });
        if !maximized {
            if let Some((bounds, origin)) = self.press {
                self.moving = Some(Attempt {
                    bounds,
                    origin,
                    end: latest.unwrap_or(origin),
                    shift: false,
                    released: if down { None } else { Some(now) },
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(35));
            }
        }
        if down {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        self.command_time = Some(now);
    }
    fn move_input(&mut self, ctx: &egui::Context) {
        let Some(bounds) = ctx.input(|i| i.viewport().outer_rect.or(i.viewport().inner_rect))
        else {
            return;
        };
        let now = ctx.input(|i| i.time);
        for event in ctx.input(|i| i.events.clone()) {
            match event {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    ..
                } => self.press = Some((bounds, pos)),
                egui::Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    ..
                } => {
                    if let Some(moving) = &mut self.moving {
                        moving.released = Some(now);
                    }
                }
                egui::Event::PointerMoved(pos) => {
                    if let Some(moving) = &mut self.moving {
                        if moving.released.is_none_or(|t| t == now) {
                            moving.end = pos;
                        }
                    }
                }
                egui::Event::WindowFocused(false) => {
                    self.cancel();
                    return;
                }
                _ => {}
            }
        }
        if let Some(position) = self.observe_move(bounds, now) {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
        }
        if self.moving.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(35));
        }
    }
    fn observe_move(&mut self, bounds: Rect, now: f64) -> Option<Pos2> {
        let moving = self.moving.as_ref()?;
        if moving.bounds.min.distance(bounds.min) > 0.5
            || (moving.bounds.size() - bounds.size()).length() > 0.5
        {
            self.moving = None;
            return None;
        }
        if moving.released.is_none_or(|t| now - t < 0.08) {
            return None;
        }
        let moving = self.moving.take().unwrap();
        let delta = moving.end - moving.origin;
        if delta.length() > 0.5 {
            Some(moving.bounds.min + delta)
        } else {
            None
        }
    }
    pub fn input(
        &mut self,
        ctx: &egui::Context,
        horizontal: bool,
        square_mode: bool,
        enabled: bool,
        min: Vec2,
        max: Vec2,
    ) {
        self.move_input(ctx);
        if !enabled || ctx.input(|i| i.viewport().focused == Some(false)) {
            return;
        }
        let Some(inner) = ctx.input(|i| i.viewport().inner_rect) else {
            return;
        };
        let outer = ctx.input(|i| i.viewport().outer_rect).unwrap_or(inner);
        let now = ctx.input(|i| i.time);
        let events = ctx.input(|i| i.events.clone());
        // A press and release may arrive in one frame before Windows can enter
        // its modal resize loop. Apply that already-finished gesture once.
        if let Some(rect) = completed_resize(&events, outer, horizontal, square_mode, min, max) {
            self.command_time = Some(now);
            self.square = None;
            self.attempt = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(rect.min));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(rect.size()));
            return;
        }
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
                            self.attempt = Some(Attempt {
                                bounds: outer,
                                origin: pos,
                                end: pos,
                                shift: modifiers.shift,
                                released: None,
                            });
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
                    if let Some(attempt) = &mut self.attempt {
                        attempt.released = Some(now);
                    }
                    if let Some(square) = &mut self.square {
                        square.released = Some(now);
                        square.stable = 0;
                    }
                }
                egui::Event::PointerMoved(pos) => {
                    if let Some(attempt) = &mut self.attempt {
                        if attempt.released.is_none_or(|time| time == now) {
                            attempt.end = pos;
                        }
                    }
                }
                _ => {}
            }
        }
        if self.attempt.as_ref().is_some_and(|a| {
            a.bounds.min.distance(outer.min) > 0.5
                || (a.bounds.size() - outer.size()).length() > 0.5
        }) {
            self.attempt = None;
        }
        if self
            .attempt
            .as_ref()
            .is_some_and(|a| a.released.is_some_and(|time| now - time >= 0.08))
        {
            let attempt = self.attempt.take().unwrap();
            let modifiers = if attempt.shift {
                egui::Modifiers::SHIFT
            } else {
                egui::Modifiers::NONE
            };
            let events = [
                egui::Event::PointerButton {
                    pos: attempt.origin,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers,
                },
                egui::Event::PointerButton {
                    pos: attempt.end,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers,
                },
            ];
            if let Some(rect) =
                completed_resize(&events, attempt.bounds, horizontal, square_mode, min, max)
            {
                self.square = None;
                if (rect.size() - outer.size()).length() > 0.5 || rect.min.distance(outer.min) > 0.5
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(rect.min));
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(rect.size()));
                }
            }
        }
        if let Some((position, size)) =
            self.observe(outer, now, ctx.input(|i| i.pointer.primary_down()))
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }
        if self.square.is_some() || self.attempt.is_some() {
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
        self.attempt = None;
        self.moving = None;
        self.press = None;
    }
}
fn completed_resize(
    events: &[egui::Event],
    bounds: Rect,
    horizontal: bool,
    square: bool,
    min: Vec2,
    max: Vec2,
) -> Option<Rect> {
    let mut start = None;
    for event in events {
        if let egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        } = event
        {
            if *pressed {
                start =
                    direction(*pos, bounds.size(), horizontal).map(|d| (*pos, d, modifiers.shift));
            } else if let Some((origin, direction, shift)) = start.take() {
                use egui::ResizeDirection::*;
                let left = matches!(direction, West | NorthWest | SouthWest);
                let right = matches!(direction, East | NorthEast | SouthEast);
                let top = matches!(direction, North | NorthWest | NorthEast);
                let bottom = matches!(direction, South | SouthWest | SouthEast);
                let delta = *pos - origin;
                let width = (bounds.width()
                    + if left {
                        -delta.x
                    } else if right {
                        delta.x
                    } else {
                        0.
                    })
                .clamp(min.x, max.x);
                let height = (bounds.height()
                    + if top {
                        -delta.y
                    } else if bottom {
                        delta.y
                    } else {
                        0.
                    })
                .clamp(min.y, max.y);
                let mut size = Vec2::new(width, height);
                if square && shift {
                    size = Vec2::splat(width.max(height).clamp(min.x.max(min.y), max.x.min(max.y)));
                }
                let position = Pos2::new(
                    if left {
                        bounds.right() - size.x
                    } else {
                        bounds.left()
                    },
                    if top {
                        bounds.bottom() - size.y
                    } else {
                        bounds.top()
                    },
                );
                return Some(Rect::from_min_size(position, size));
            }
        }
    }
    None
}

/// The monitor containing this viewport, excluding task bars. Work-area and
/// viewport coordinates use the same logical-pixel scale, including negatives.
pub fn work_area(ctx: &egui::Context) -> Option<Rect> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::POINT,
            Graphics::Gdi::{
                GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
            },
        };
        let center = ctx.input(|i| {
            i.viewport()
                .outer_rect
                .or(i.viewport().inner_rect)
                .map(|r| r.center())
        });
        if let Some(center) = center {
            let scale = ctx.pixels_per_point();
            let point = POINT {
                x: (center.x * scale).round() as i32,
                y: (center.y * scale).round() as i32,
            };
            // Both APIs only read monitor geometry; the sized stack record is
            // valid for the entire call and contains no owned pointers.
            unsafe {
                let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
                let mut info: MONITORINFO = std::mem::zeroed();
                info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                if !monitor.is_null() && GetMonitorInfoW(monitor, &mut info) != 0 {
                    let r = info.rcWork;
                    return Some(Rect::from_min_max(
                        Pos2::new(r.left as f32 / scale, r.top as f32 / scale),
                        Pos2::new(r.right as f32 / scale, r.bottom as f32 / scale),
                    ));
                }
            }
        }
    }
    ctx.input(|i| {
        i.viewport()
            .monitor_size
            .map(|size| Rect::from_min_size(Pos2::ZERO, size))
    })
}
pub fn fit_position(position: Pos2, size: Vec2, area: Rect) -> Pos2 {
    Pos2::new(
        position
            .x
            .clamp(area.left(), (area.right() - size.x).max(area.left())),
        position
            .y
            .clamp(area.top(), (area.bottom() - size.y).max(area.top())),
    )
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
    fn restored_window_fits_work_area_and_preserves_negative_monitor_origins() {
        let primary = Rect::from_min_max(Pos2::ZERO, Pos2::new(1440., 852.));
        assert_eq!(
            fit_position(Pos2::new(586., 316.), Vec2::new(620., 720.), primary),
            Pos2::new(586., 132.)
        );
        let left = Rect::from_min_max(Pos2::new(-1280., -200.), Pos2::new(0., 500.));
        assert_eq!(
            fit_position(Pos2::new(-200., 450.), Vec2::new(360., 360.), left),
            Pos2::new(-360., 140.)
        );
        assert_eq!(
            fit_position(Pos2::new(-1400., -300.), Vec2::new(360., 360.), left),
            left.min
        );
    }
    #[test]
    fn moving_fallback_waits_for_release_and_never_overrides_native_movement() {
        let bounds = Rect::from_min_size(Pos2::new(100., 100.), Vec2::splat(48.));
        let attempt = || Attempt {
            bounds,
            origin: Pos2::new(12., 12.),
            end: Pos2::new(32., 28.),
            shift: false,
            released: None,
        };
        let mut state = State {
            moving: Some(attempt()),
            ..Default::default()
        };
        assert!(state.observe_move(bounds, 1.).is_none());
        state.moving.as_mut().unwrap().released = Some(1.);
        assert!(state.observe_move(bounds, 1.05).is_none());
        assert_eq!(state.observe_move(bounds, 1.1), Some(Pos2::new(120., 116.)));
        assert!(state.observe_move(bounds, 1.2).is_none());
        state.moving = Some(attempt());
        assert!(state
            .observe_move(bounds.translate(Vec2::new(10., 0.)), 2.)
            .is_none());
        assert!(state.moving.is_none());
    }
    #[test]
    fn a_finished_fast_resize_is_applied_once_and_keeps_the_opposite_corner() {
        let bounds = Rect::from_min_size(Pos2::new(100., 100.), Vec2::splat(360.));
        let pressed = egui::Event::PointerButton {
            pos: Pos2::new(354., 354.),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        };
        let released = egui::Event::PointerButton {
            pos: Pos2::new(330., 294.),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        };
        assert!(completed_resize(
            std::slice::from_ref(&pressed),
            bounds,
            false,
            false,
            Vec2::new(320., 220.),
            Vec2::splat(8192.)
        )
        .is_none());
        let rect = completed_resize(
            &[pressed, released.clone()],
            bounds,
            false,
            false,
            Vec2::new(320., 220.),
            Vec2::splat(8192.),
        )
        .unwrap();
        assert_eq!(rect.min, bounds.min);
        assert_eq!(rect.size(), Vec2::new(336., 300.));
        assert!(completed_resize(
            &[released],
            bounds,
            false,
            false,
            Vec2::splat(24.),
            Vec2::splat(360.)
        )
        .is_none());
        let events = [
            egui::Event::PointerButton {
                pos: Pos2::new(3., 3.),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::SHIFT,
            },
            egui::Event::PointerButton {
                pos: Pos2::new(33., 53.),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::SHIFT,
            },
        ];
        let rect = completed_resize(
            &events,
            bounds,
            false,
            true,
            Vec2::splat(24.),
            Vec2::splat(360.),
        )
        .unwrap();
        assert_eq!(rect.max, bounds.max);
        assert_eq!(rect.size(), Vec2::splat(330.));
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
