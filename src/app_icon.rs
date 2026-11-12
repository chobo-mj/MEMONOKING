//! The app's own icon (designed in `icon/`, bundled here as `.ico` files),
//! decoded once per use: a larger frame for window/taskbar icons and a
//! 32×32 frame for the system tray.

use eframe::egui;

const TASKBAR_ICO: &[u8] = include_bytes!("../icon/memo_taskbar_yellow.ico");
const TRAY_ICO: &[u8] = include_bytes!("../icon/memo_tray_yellow.ico");

/// A window/taskbar icon, picked close to 128×128.
pub fn window() -> egui::IconData {
    decode(TASKBAR_ICO, 128)
}

/// A 32×32 RGBA buffer for the system-tray icon.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn tray_rgba() -> Vec<u8> {
    decode(TRAY_ICO, 32).rgba
}

/// Decodes the bundled `.ico`'s frame closest to `size` pixels square.
fn decode(bytes: &[u8], size: u32) -> egui::IconData {
    let dir =
        ico::IconDir::read(std::io::Cursor::new(bytes)).expect("bundled icon is a valid .ico");
    let entry = dir
        .entries()
        .iter()
        .min_by_key(|e| (e.width() as i32 - size as i32).abs())
        .expect("bundled icon has at least one frame");
    let image = entry.decode().expect("bundled icon frame decodes");
    egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.rgba_data().to_vec(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn window_icon_decodes_to_a_square_rgba_buffer_near_128px() {
        let icon = super::window();
        assert_eq!(icon.width, icon.height);
        assert!((100..=256).contains(&icon.width), "{}", icon.width);
        assert_eq!(icon.rgba.len() as u32, icon.width * icon.height * 4);
    }

    #[test]
    fn tray_icon_decodes_to_32_square_rgba() {
        let px = super::tray_rgba();
        assert_eq!(px.len(), 32 * 32 * 4);
    }
}
