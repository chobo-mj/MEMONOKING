use crate::*;
use egui::{Pos2, Rect, ViewportId};
#[derive(Clone)]
pub struct Source {
    pub layer: egui::LayerId,
    pub viewport: ViewportId,
    pub rect: Rect,
    pub note: usize,
    pub card: bool,
    pub folder: bool,
    pub occurrence: Option<NaiveDate>,
}
#[derive(Clone)]
pub enum Destination {
    Date(NaiveDate),
    Parent(usize),
    WikiFolder(Option<usize>),
}
#[derive(Clone)]
pub struct Target {
    pub layer: egui::LayerId,
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
            layer: ui.layer_id(),
            rect: r.rect.intersect(ui.clip_rect()),
            note,
            card,
            folder: false,
            occurrence: None,
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
                    if !self.capture && !self.settings {
                        if let Some(source) = self
                            .sources
                            .iter()
                            .rev()
                            .find(|s| {
                                s.viewport == viewport
                                    && s.rect.contains(
                                        ctx.layer_transform_from_global(s.layer)
                                            .unwrap_or(egui::emath::TSTransform::IDENTITY)
                                            * pos,
                                    )
                            })
                            .cloned()
                        {
                            let initial = if source.folder {
                                Vec2::ZERO
                            } else {
                                self.data.notes[source.note].pos
                            };
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
                                self.data.notes[d.source.note].pos = (d.initial
                                    + (pos - d.origin)
                                        / ctx
                                            .layer_transform_to_global(d.source.layer)
                                            .map_or(1., |t| t.scaling))
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
                                self.data.notes[d.source.note].pos = (d.initial
                                    + (pos - d.origin)
                                        / ctx
                                            .layer_transform_to_global(d.source.layer)
                                            .map_or(1., |t| t.scaling))
                                .clamp(Vec2::ZERO, Vec2::new(1196., 766.));
                            } else if let Some(t) = self
                                .targets
                                .iter()
                                .rev()
                                .find(|t| {
                                    t.viewport == viewport
                                        && t.rect.contains(
                                            ctx.layer_transform_from_global(t.layer)
                                                .unwrap_or(egui::emath::TSTransform::IDENTITY)
                                                * pos,
                                        )
                                })
                                .cloned()
                            {
                                match t.kind {
                                    Destination::Date(date) => {
                                        if !d.source.folder {
                                            let n = &mut self.data.notes[d.source.note];
                                            let new_start = match (n.date, d.source.occurrence) {
                                                (Some(start), Some(from)) => start + (date - from),
                                                _ => date,
                                            };
                                            n.move_start(new_start);
                                            self.day = date;
                                            self.toast = format!("{} → {}", n.title, new_start);
                                        }
                                    }
                                    Destination::WikiFolder(folder) => {
                                        if d.source.folder {
                                            if !self.move_folder(d.source.note, folder) {
                                                self.toast="폴더의 자기 자신/하위 폴더 안으로는 이동할 수 없습니다".into();
                                            }
                                        } else if self.data.notes[d.source.note].wiki {
                                            self.data.notes[d.source.note].folder = folder;
                                            self.toast = "위키 메모를 분류했습니다".into();
                                        }
                                    }
                                    Destination::Parent(parent) => {
                                        if !d.source.folder
                                            && !self.data.reparent(d.source.note, parent)
                                        {
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
    pub(super) fn simulate(a: &mut App, from: Pos2, to: Pos2) {
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
    pub(super) fn source() -> Source {
        Source {
            viewport: ViewportId::ROOT,
            layer: egui::LayerId::background(),
            rect: Rect::from_min_max(Pos2::new(10., 10.), Pos2::new(100., 50.)),
            note: 0,
            card: false,
            folder: false,
            occurrence: None,
        }
    }
    #[test]
    fn fast_drag_in_one_frame_moves_date_and_preserves_original_note() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
        let mut a = App::sample(date);
        a.sources.push(source());
        a.targets.push(Target {
            viewport: ViewportId::ROOT,
            layer: egui::LayerId::background(),
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

#[cfg(test)]
mod zoom_tests {
    use super::*;
    #[test]
    fn scaled_card_drag_uses_scene_coordinates() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let mut a = App::sample(date);
        let layer = egui::LayerId::new(egui::Order::Middle, egui::Id::new("zoom_test"));
        a.sources.push(Source {
            viewport: ViewportId::ROOT,
            layer,
            rect: Rect::from_min_max(Pos2::new(10., 10.), Pos2::new(100., 50.)),
            note: 0,
            card: true,
            folder: false,
            occurrence: None,
        });
        let initial = a.data.notes[0].pos;
        let ctx = egui::Context::default();
        let from = Pos2::new(215., 115.);
        let to = Pos2::new(255., 135.);
        let events = vec![
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
            |ctx| {
                ctx.set_transform_layer(
                    layer,
                    egui::emath::TSTransform {
                        scaling: 0.5,
                        translation: Vec2::new(200., 100.),
                    },
                );
                a.drag_input(ctx);
            },
        );
        assert_eq!(a.data.notes[0].pos, initial + Vec2::new(80., 40.));
    }
}
#[cfg(test)]
mod wiki_period_tests {
    use super::*;
    fn target(kind: Destination) -> Target {
        Target {
            viewport: ViewportId::ROOT,
            layer: egui::LayerId::background(),
            rect: Rect::from_min_max(Pos2::new(200., 10.), Pos2::new(300., 100.)),
            kind,
        }
    }
    #[test]
    fn wiki_drop_changes_classification_without_copying_note() {
        let mut a = App::sample(Local::now().date_naive());
        let count = a.data.notes.len();
        a.sources.push(super::tests::source());
        a.targets.push(target(Destination::WikiFolder(None)));
        super::tests::simulate(&mut a, Pos2::new(30., 30.), Pos2::new(230., 30.));
        assert!(a.data.notes[0].folder.is_none());
        a.sources.push(super::tests::source());
        a.targets.push(target(Destination::WikiFolder(Some(0))));
        super::tests::simulate(&mut a, Pos2::new(30., 30.), Pos2::new(230., 30.));
        assert_eq!(a.data.notes[0].folder, Some(0));
        assert_eq!(a.data.notes.len(), count);
    }
    #[test]
    fn folder_drag_keeps_note_data_untouched() {
        let mut a = App::sample(Local::now().date_naive());
        a.extra.folders.push(crate::extras::Folder {
            title: "두번째".into(),
            description: String::new(),
            parent: None,
        });
        let mut source = super::tests::source();
        source.folder = true;
        source.note = 1;
        a.sources.push(source);
        a.targets.push(target(Destination::WikiFolder(Some(0))));
        super::tests::simulate(&mut a, Pos2::new(30., 30.), Pos2::new(230., 30.));
        assert_eq!(a.extra.folders[1].parent, Some(0));
        assert_eq!(a.data.notes[1].parents, Vec::<usize>::new());
    }
    #[test]
    fn dragging_middle_day_moves_whole_period_by_offset() {
        let start = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let mut a = App::sample(start);
        a.data.notes[0].duration = 3;
        let mut s = super::tests::source();
        s.occurrence = Some(start + chrono::Duration::days(1));
        a.sources.push(s);
        a.targets
            .push(target(Destination::Date(start + chrono::Duration::days(4))));
        super::tests::simulate(&mut a, Pos2::new(30., 30.), Pos2::new(230., 30.));
        assert_eq!(
            a.data.notes[0].date,
            Some(start + chrono::Duration::days(3))
        );
        assert_eq!(a.data.notes[0].duration, 3);
    }
}
