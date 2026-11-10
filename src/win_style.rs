//! Native window shaping on Windows.
//!
//! Many GPU drivers give OpenGL windows no per-pixel alpha, so anything egui
//! paints transparent shows up black (including the area outside rounded
//! corners). Borderless windows are therefore painted opaque and square:
//! Windows rounds their corners and applies whole-window opacity through a
//! layered window, which every driver supports.

use eframe::egui;

/// True when the OS shapes windows, so content must be painted opaque and
/// unrounded. Elsewhere egui's own alpha and rounding are used.
pub const NATIVE: bool = cfg!(windows);

/// Round the corners of the window `ctx` draws and set its whole-window
/// opacity (`0.0..=1.0`). Cheap to call every frame: work is only done when
/// a value changes.
pub fn shape(ctx: &egui::Context, opacity: f32) {
    #[cfg(windows)]
    imp::shape(ctx, opacity);
    #[cfg(not(windows))]
    let _ = (ctx, opacity);
}

#[cfg(windows)]
mod imp {
    use eframe::egui;
    use windows_sys::Win32::{
        Foundation::{BOOL, HWND, LPARAM, RECT},
        Graphics::{
            Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND},
            Gdi::{CreateRoundRectRgn, SetWindowRgn},
        },
        System::Threading::GetCurrentThreadId,
        UI::WindowsAndMessaging::{
            EnumThreadWindows, GetWindowLongPtrW, GetWindowRect, GetWindowTextW, IsWindow,
            SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED,
        },
    };

    #[derive(Clone, Copy, Default)]
    struct State {
        hwnd: isize,
        alpha: Option<u8>,
        native_round: Option<bool>,
        region: (i32, i32),
    }

    pub fn shape(ctx: &egui::Context, opacity: f32) {
        let key = egui::Id::new(("win_style", ctx.viewport_id()));
        let mut s: State = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
        // SAFETY: plain Win32 calls on a window owned by this thread.
        unsafe {
            if s.hwnd == 0 || IsWindow(s.hwnd as HWND) == 0 {
                let Some(h) = find(ctx) else {
                    return;
                };
                s = State {
                    hwnd: h as isize,
                    ..Default::default()
                };
            }
            let h = s.hwnd as HWND;
            let alpha = (opacity.clamp(0., 1.) * 255.).round() as u8;
            if s.alpha != Some(alpha) {
                let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
                if ex & WS_EX_LAYERED as isize == 0 {
                    SetWindowLongPtrW(h, GWL_EXSTYLE, ex | WS_EX_LAYERED as isize);
                }
                SetLayeredWindowAttributes(h, 0, alpha, LWA_ALPHA);
                s.alpha = Some(alpha);
            }
            if s.native_round.is_none() {
                let pref = DWMWCP_ROUND;
                let hr = DwmSetWindowAttribute(
                    h,
                    DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                    &pref as *const _ as *const _,
                    std::mem::size_of_val(&pref) as u32,
                );
                s.native_round = Some(hr >= 0);
            }
            // Windows 10 has no rounded-corner preference: clip to a region.
            if s.native_round == Some(false) {
                let mut r: RECT = std::mem::zeroed();
                GetWindowRect(h, &mut r);
                let size = (r.right - r.left, r.bottom - r.top);
                if size != s.region {
                    let d = (16. * ctx.pixels_per_point()).round() as i32;
                    let rgn = CreateRoundRectRgn(0, 0, size.0 + 1, size.1 + 1, d, d);
                    SetWindowRgn(h, rgn, 1);
                    s.region = size;
                }
            }
        }
        ctx.data_mut(|d| d.insert_temp(key, s));
    }

    struct Search {
        title: Vec<u16>,
        rect: RECT,
        best: Option<(i32, HWND)>,
    }

    /// egui does not expose native handles of child viewports, so the window
    /// is matched by its title and on-screen rectangle among this thread's
    /// top-level windows.
    unsafe fn find(ctx: &egui::Context) -> Option<HWND> {
        let (title, rect, ppp) = ctx.input(|i| {
            let v = i.viewport();
            (
                v.title.clone(),
                v.outer_rect,
                v.native_pixels_per_point.unwrap_or(1.),
            )
        });
        let (title, rect) = (title?, rect?);
        let mut search = Search {
            title: title.encode_utf16().collect(),
            rect: RECT {
                left: (rect.min.x * ppp).round() as i32,
                top: (rect.min.y * ppp).round() as i32,
                right: (rect.max.x * ppp).round() as i32,
                bottom: (rect.max.y * ppp).round() as i32,
            },
            best: None,
        };
        unsafe extern "system" fn visit(h: HWND, data: LPARAM) -> BOOL {
            let search = &mut *(data as *mut Search);
            let mut buf = [0u16; 512];
            let n = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
            if buf[..n] == search.title[..] {
                let mut r: RECT = std::mem::zeroed();
                GetWindowRect(h, &mut r);
                let t = search.rect;
                let d = (r.left - t.left).abs()
                    + (r.top - t.top).abs()
                    + (r.right - t.right).abs()
                    + (r.bottom - t.bottom).abs();
                if search.best.is_none_or(|(b, _)| d < b) {
                    search.best = Some((d, h));
                }
            }
            1
        }
        EnumThreadWindows(
            GetCurrentThreadId(),
            Some(visit),
            &mut search as *mut Search as LPARAM,
        );
        // A far-off match is a different window with the same title.
        search.best.filter(|(d, _)| *d < 64).map(|(_, h)| h)
    }
}
