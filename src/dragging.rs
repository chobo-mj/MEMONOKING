use crate::*;
use egui::{Pos2, Rect, ViewportId};
#[derive(Clone)]
pub struct Source {
    pub viewport: ViewportId,
    pub rect: Rect,
    pub note: usize,
    pub card: bool,
}
#[derive(Clone)]
pub enum Destination {
    Date(NaiveDate),
    Parent(usize),
}
#[derive(Clone)]
pub struct Target {
    pub viewport: ViewportId,
    pub rect: Rect,
    pub kind: Destination,
}
pub struct Drag {
    source: Source,
    origin: Pos2,
    initial: Vec2,
    current: Pos2,
}
impl App {
    pub fn drag_source(&mut self, ui: &egui::Ui, r: &egui::Response, note: usize, card: bool) {
        self.sources.push(Source {
            viewport: ui.ctx().viewport_id(),
            rect: r.rect.intersect(ui.clip_rect()),
            note,
            card,
        });
    }
    pub fn drag_input(&mut self, ctx: &egui::Context) {
        let viewport = ctx.viewport_id();
        let events = ctx.input(|i| i.events.clone());
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if let Some(d) = self.drag.take() {
                if d.source.card {
                    self.data.notes[d.source.note].pos = d.initial;
                }
            }
        }
        for e in events {
            match e {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    ..
                } => {
                    if !self.editor && !self.capture && !self.settings {
                        if let Some(source) = self
                            .sources
                            .iter()
                            .rev()
                            .find(|s| s.viewport == viewport && s.rect.contains(pos))
                            .cloned()
                        {
                            let initial = self.data.notes[source.note].pos;
                            self.drag = Some(Drag {
                                source,
                                origin: pos,
                                initial,
                                current: pos,
                            });
                        }
                    }
                }
                egui::Event::PointerMoved(pos) => {
                    if let Some(d) = &mut self.drag {
                        if d.source.viewport == viewport {
                            d.current = pos;
                            if d.source.card && d.origin.distance(pos) > 4. {
                                self.data.notes[d.source.note].pos = (d.initial + (pos - d.origin))
                                    .clamp(Vec2::ZERO, Vec2::new(1196., 766.));
                            }
                        }
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    ..
                } => {
                    if let Some(d) = self.drag.take() {
                        if d.source.viewport == viewport && d.origin.distance(pos) > 4. {
                            if d.source.card {
                                self.data.notes[d.source.note].pos = (d.initial + (pos - d.origin))
                                    .clamp(Vec2::ZERO, Vec2::new(1196., 766.));
                            } else if let Some(t) = self
                                .targets
                                .iter()
                                .rev()
                                .find(|t| t.viewport == viewport && t.rect.contains(pos))
                                .cloned()
                            {
                                match t.kind {
                                    Destination::Date(date) => {
                                        self.data.notes[d.source.note].date = Some(date);
                                        self.day = date;
                                        self.toast = format!(
                                            "{} → {}",
                                            self.data.notes[d.source.note].title, date
                                        );
                                    }
                                    Destination::Parent(parent) => {
                                        if !self.data.reparent(d.source.note, parent) {
                                            self.toast =
                                                "자기 자신 또는 순환 관계는 만들 수 없습니다"
                                                    .into();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                egui::Event::WindowFocused(false) => {
                    self.drag = None;
                }
                _ => {}
            }
        }
        self.sources.retain(|s| s.viewport != viewport);
        self.targets.retain(|t| t.viewport != viewport);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn simulate(a: &mut App, from: Pos2, to: Pos2) {
        let ctx = egui::Context::default();
        let events = vec![
            egui::Event::PointerMoved(from),
            egui::Event::PointerButton {
                pos: from,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerMoved(to),
            egui::Event::PointerButton {
                pos: to,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| a.drag_input(ctx),
        );
    }
    fn source() -> Source {
        Source {
            viewport: ViewportId::ROOT,
            rect: Rect::from_min_max(Pos2::new(10., 10.), Pos2::new(100., 50.)),
            note: 0,
            card: false,
        }
    }
    #[test]
    fn fast_drag_in_one_frame_moves_date_and_preserves_original_note() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
        let mut a = App::sample(date);
        a.sources.push(source());
        a.targets.push(Target {
            viewport: ViewportId::ROOT,
            rect: Rect::from_min_max(Pos2::new(200., 10.), Pos2::new(300., 100.)),
            kind: Destination::Date(date + chrono::Duration::days(3)),
        });
        let body = a.data.notes[0].body.clone();
        let time = a.data.notes[0].time;
        simulate(&mut a, Pos2::new(30., 30.), Pos2::new(230., 30.));
        assert_eq!(a.data.notes[0].date, Some(date + chrono::Duration::days(3)));
        assert_eq!(a.data.notes[0].body, body);
        assert_eq!(a.data.notes[0].time, time);
        assert_eq!(a.data.notes.len(), 6);
    }
    #[test]
    fn dropping_outside_dates_keeps_schedule() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
        let mut a = App::sample(date);
        a.sources.push(source());
        simulate(&mut a, Pos2::new(30., 30.), Pos2::new(500., 500.));
        assert_eq!(a.data.notes[0].date, Some(date));
    }
    #[test]
    fn card_drag_changes_layout_without_changing_schedule() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
        let mut a = App::sample(date);
        let mut s = source();
        s.card = true;
        a.sources.push(s);
        let initial = a.data.notes[0].pos;
        simulate(&mut a, Pos2::new(30., 30.), Pos2::new(80., 60.));
        assert_eq!(a.data.notes[0].pos, initial + Vec2::new(50., 30.));
        assert_eq!(a.data.notes[0].date, Some(date));
    }
}
