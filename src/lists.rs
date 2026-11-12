//! Keeping long memo lists fast: rows off screen are not drawn, and the
//! filtered, sorted row list is computed again only after something changed.
use eframe::egui;
use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Counts user input (keys, clicks, wheel, drops) in any window. Data and
/// filters change only through input, so an unchanged count means a list
/// computed earlier is still right.
pub static CHANGES: AtomicU64 = AtomicU64::new(0);

/// Lists computed earlier, by slot.
#[derive(Default)]
pub struct Cache {
    slots: HashMap<&'static str, (u64, Rc<dyn Any>)>,
}

impl Cache {
    /// The list in `slot`, computed by `make` when there was input since it
    /// was made, or when `key` (anything else it depends on, like the day or
    /// the number of memos) differs.
    pub fn get<T: 'static>(
        &mut self,
        slot: &'static str,
        key: u64,
        make: impl FnOnce() -> T,
    ) -> Rc<T> {
        let key = key
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(CHANGES.load(Ordering::Relaxed));
        if let Some((k, v)) = self.slots.get(slot) {
            if *k == key {
                if let Ok(v) = v.clone().downcast::<T>() {
                    return v;
                }
            }
        }
        let v = Rc::new(make());
        self.slots.insert(slot, (key, v.clone()));
        v
    }
}

/// A key from values a list depends on besides input.
pub fn key(parts: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

/// Draw `add` only when its row is on screen. A row off screen reserves the
/// height it had when last drawn (`estimate` before that), so scrolling and
/// the scroll bar behave as if every row were there. A `keep` row (such as
/// the selected one, which may scroll itself into view) is always drawn.
pub fn lazy_row(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    estimate: f32,
    keep: bool,
    add: impl FnOnce(&mut egui::Ui),
) {
    let id = ui.id().with(("lazy_row", id));
    let height = ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(estimate);
    let width = ui.available_width();
    let guess = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(width, height));
    if !keep && !ui.is_rect_visible(guess) {
        ui.allocate_space(egui::vec2(width, height));
        return;
    }
    let drawn = ui.push_id(id, add).response.rect.height();
    ui.ctx().data_mut(|d| d.insert_temp(id, drawn));
}

/// When each cached picture texture was last drawn (egui time).
static TEXTURES: std::sync::Mutex<Option<HashMap<egui::Id, f64>>> = std::sync::Mutex::new(None);

/// Seconds a picture texture is kept after it was last on screen.
pub const TEXTURE_KEEP: f64 = 60.;

/// A picture texture made by `make` on first use and kept while it is being
/// drawn; `release_textures` frees it once it has been off screen a while.
pub fn texture(
    ctx: &egui::Context,
    id: egui::Id,
    make: impl FnOnce() -> Option<egui::ColorImage>,
) -> Option<egui::TextureHandle> {
    let now = ctx.input(|i| i.time);
    let cached = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id));
    let texture = match cached {
        Some(t) => t,
        None => {
            let t = ctx.load_texture(
                id.short_debug_format(),
                make()?,
                egui::TextureOptions::LINEAR,
            );
            ctx.data_mut(|d| d.insert_temp(id, t.clone()));
            t
        }
    };
    let mut used = TEXTURES.lock().unwrap_or_else(|e| e.into_inner());
    used.get_or_insert_with(Default::default).insert(id, now);
    Some(texture)
}

/// Free cached textures not drawn for `TEXTURE_KEEP` seconds.
pub fn release_textures(ctx: &egui::Context) {
    let now = ctx.input(|i| i.time);
    let mut used = TEXTURES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(used) = used.as_mut() {
        used.retain(|id, at| {
            let keep = now - *at < TEXTURE_KEEP;
            if !keep {
                ctx.data_mut(|d| d.remove::<egui::TextureHandle>(*id));
            }
            keep
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_list_is_reused_until_input_or_key_changes() {
        let mut cache = Cache::default();
        let mut made = 0;
        let mut get = |cache: &mut Cache, key| {
            cache.get("t", key, || {
                made += 1;
                vec![made]
            })
        };
        assert_eq!(*get(&mut cache, 1), vec![1]);
        assert_eq!(*get(&mut cache, 1), vec![1]);
        assert_eq!(*get(&mut cache, 2), vec![2]);
        CHANGES.fetch_add(1, Ordering::Relaxed);
        assert_eq!(*get(&mut cache, 2), vec![3]);
    }

    #[test]
    fn rows_off_screen_are_not_drawn() {
        let ctx = egui::Context::default();
        let mut drawn = 0;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(200.)
                    .show(ui, |ui| {
                        for k in 0..10_000 {
                            lazy_row(ui, k, 30., false, |ui| {
                                drawn += 1;
                                ui.label(format!("row {k}"));
                            });
                        }
                    });
            });
        });
        assert!(drawn > 0 && drawn < 50, "{drawn} rows drawn");
    }

    #[test]
    fn textures_are_released_after_being_off_screen() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("texture-test");
        let image = || Some(egui::ColorImage::new([2, 2], egui::Color32::RED));
        let at = |time: f64| egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        let made = std::cell::Cell::new(0);
        let draw = |time: f64| {
            let _ = ctx.run(at(time), |ctx| {
                texture(ctx, id, || {
                    made.set(made.get() + 1);
                    image()
                });
            });
        };
        draw(0.);
        draw(1.);
        let _ = ctx.run(at(30.), release_textures);
        draw(31.);
        assert_eq!(made.get(), 1);
        let _ = ctx.run(at(31. + TEXTURE_KEEP + 1.), release_textures);
        assert!(ctx
            .data(|d| d.get_temp::<egui::TextureHandle>(id))
            .is_none());
        draw(200.);
        assert_eq!(made.get(), 2);
    }
}
