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
}
