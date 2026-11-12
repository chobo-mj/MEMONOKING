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

/// Native handle of the window `ctx` draws, if it can be found.
pub fn hwnd(ctx: &egui::Context) -> Option<isize> {
    #[cfg(windows)]
    return unsafe { imp::find(ctx).map(|h| h as isize) };
    #[cfg(not(windows))]
    {
        let _ = ctx;
        None
    }
}

/// A screen rectangle in physical pixels: left, top, right, bottom.
pub type PxRect = [i32; 4];

/// Put window `hwnd` on the desktop wallpaper layer at `rect`: below the
/// desktop icons, so clicks go to the desktop and icons can sit on top.
pub fn pin_to_desktop(hwnd: isize, rect: PxRect) -> bool {
    #[cfg(windows)]
    return unsafe { desk::pin(hwnd, rect) };
    #[cfg(not(windows))]
    {
        let _ = (hwnd, rect);
        false
    }
}

/// Make a pinned window an ordinary top-level window again at `rect`.
pub fn unpin_from_desktop(hwnd: isize, rect: PxRect) {
    #[cfg(windows)]
    unsafe {
        desk::unpin(hwnd, rect)
    };
    #[cfg(not(windows))]
    let _ = (hwnd, rect);
}

/// Watches, on a small background thread, for a click on `rect` while the
/// desktop (not another window) is under the pointer. The standing calendar
/// lives below the desktop icons and gets no input of its own.
pub struct DesktopClick {
    hit: std::sync::Arc<std::sync::atomic::AtomicBool>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl DesktopClick {
    pub fn watch(rect: PxRect) -> Self {
        use std::sync::{atomic::AtomicBool, Arc};
        let hit = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        #[cfg(windows)]
        desk::watch(rect, hit.clone(), stop.clone());
        #[cfg(not(windows))]
        let _ = rect;
        Self { hit, stop }
    }
    /// True once after a click was seen.
    pub fn take(&self) -> bool {
        self.hit.swap(false, std::sync::atomic::Ordering::Relaxed)
    }
}

impl Drop for DesktopClick {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        #[cfg(windows)]
        desk::unwatch();
    }
}

#[cfg(windows)]
mod desk {
    use super::PxRect;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };
    use windows_sys::Win32::{
        Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, WPARAM},
        Graphics::Gdi::ScreenToClient,
        System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
        UI::WindowsAndMessaging::{
            CallNextHookEx, EnumWindows, FindWindowExW, FindWindowW, GetClassNameW, GetMessageW,
            PostThreadMessageW, SendMessageTimeoutW, SetParent, SetWindowPos, SetWindowsHookExW,
            UnhookWindowsHookEx, WindowFromPoint, MSG, MSLLHOOKSTRUCT, SMTO_NORMAL, SWP_NOACTIVATE,
            SWP_NOZORDER, SWP_SHOWWINDOW, WH_MOUSE_LL, WM_LBUTTONUP, WM_QUIT,
        },
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// The window that draws the wallpaper, behind the desktop icons.
    unsafe fn wallpaper_layer() -> HWND {
        let progman = FindWindowW(wide("Progman").as_ptr(), std::ptr::null());
        if progman.is_null() {
            return progman;
        }
        // Ask Explorer to create the WorkerW that sits behind the icons.
        let mut result = 0usize;
        SendMessageTimeoutW(progman, 0x052C, 0xD, 1, SMTO_NORMAL, 1000, &mut result);
        SendMessageTimeoutW(progman, 0x052C, 0, 0, SMTO_NORMAL, 1000, &mut result);
        let mut worker: HWND = std::ptr::null_mut();
        unsafe extern "system" fn visit(h: HWND, data: LPARAM) -> BOOL {
            let worker = &mut *(data as *mut HWND);
            let view = FindWindowExW(
                h,
                std::ptr::null_mut(),
                wide("SHELLDLL_DefView").as_ptr(),
                std::ptr::null(),
            );
            if !view.is_null() {
                // Classic layout: the next top-level WorkerW is the wallpaper.
                *worker = FindWindowExW(
                    std::ptr::null_mut(),
                    h,
                    wide("WorkerW").as_ptr(),
                    std::ptr::null(),
                );
            }
            1
        }
        EnumWindows(Some(visit), &mut worker as *mut HWND as LPARAM);
        if worker.is_null() {
            // Windows 11 24H2: the WorkerW is a child of Progman, under the icons.
            worker = FindWindowExW(
                progman,
                std::ptr::null_mut(),
                wide("WorkerW").as_ptr(),
                std::ptr::null(),
            );
        }
        if worker.is_null() {
            progman
        } else {
            worker
        }
    }

    pub unsafe fn pin(hwnd: isize, r: PxRect) -> bool {
        let layer = wallpaper_layer();
        if layer.is_null() {
            return false;
        }
        let h = hwnd as HWND;
        // SetParent's return is the window's previous parent on success and
        // null on failure. Left unchecked, a failed reparent was still
        // reported as a successful pin: decorations stayed off and the
        // window never got its title bar back, with no way to tell it had
        // never actually moved onto the desktop layer.
        if SetParent(h, layer).is_null() {
            return false;
        }
        let mut p = POINT { x: r[0], y: r[1] };
        ScreenToClient(layer, &mut p);
        SetWindowPos(
            h,
            std::ptr::null_mut(),
            p.x,
            p.y,
            r[2] - r[0],
            r[3] - r[1],
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        true
    }

    pub unsafe fn unpin(hwnd: isize, r: PxRect) {
        let h = hwnd as HWND;
        SetParent(h, std::ptr::null_mut());
        SetWindowPos(
            h,
            std::ptr::null_mut(),
            r[0],
            r[1],
            r[2] - r[0],
            r[3] - r[1],
            SWP_NOZORDER | SWP_SHOWWINDOW,
        );
    }

    /// The desktop is under `p` (not some other window).
    unsafe fn desktop_at(p: POINT) -> bool {
        let top = WindowFromPoint(p);
        let mut buf = [0u16; 64];
        let n = GetClassNameW(top, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
        let class = String::from_utf16_lossy(&buf[..n]);
        matches!(
            class.as_str(),
            "SysListView32" | "SHELLDLL_DefView" | "WorkerW" | "Progman"
        ) || top.is_null()
    }

    type Watch = (PxRect, Arc<AtomicBool>);
    static WATCH: Mutex<Option<Watch>> = Mutex::new(None);
    static THREAD: Mutex<Option<u32>> = Mutex::new(None);

    unsafe extern "system" fn hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
        if code >= 0 && w as u32 == WM_LBUTTONUP {
            let info = &*(l as *const MSLLHOOKSTRUCT);
            let p = info.pt;
            if let Some((r, hit)) = WATCH.lock().ok().and_then(|w| w.clone()) {
                let inside = p.x >= r[0] && p.x < r[2] && p.y >= r[1] && p.y < r[3];
                if inside && desktop_at(p) {
                    hit.store(true, Ordering::Relaxed);
                }
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, w, l)
    }

    /// A low-level mouse hook on its own thread reports clicks on `rect`
    /// even though the desktop, not this app, receives them.
    pub fn watch(rect: PxRect, hit: Arc<AtomicBool>, stop: Arc<AtomicBool>) {
        unwatch();
        if let Ok(mut w) = WATCH.lock() {
            *w = Some((rect, hit));
        }
        std::thread::spawn(move || unsafe {
            let module = GetModuleHandleW(std::ptr::null());
            let h = SetWindowsHookExW(WH_MOUSE_LL, Some(hook), module, 0);
            if let Ok(mut t) = THREAD.lock() {
                *t = Some(GetCurrentThreadId());
            }
            let mut msg: MSG = std::mem::zeroed();
            while !stop.load(Ordering::Relaxed)
                && GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0
            {}
            if !h.is_null() {
                UnhookWindowsHookEx(h);
            }
        });
    }

    pub fn unwatch() {
        if let Ok(mut w) = WATCH.lock() {
            *w = None;
        }
        if let Some(t) = THREAD.lock().ok().and_then(|mut t| t.take()) {
            unsafe { PostThreadMessageW(t, WM_QUIT, 0, 0) };
        }
    }
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
            // Checked every frame: winit rewrites the extended style whenever
            // it changes the window (e.g. toggling resizable for the color
            // box), which silently drops the layered flag and the opacity.
            // An opaque window is left unlayered: a layered OpenGL window
            // stopped presenting new frames after growing back from a title
            // strip or color box.
            let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
            let layered = ex & WS_EX_LAYERED as isize != 0;
            if alpha == u8::MAX {
                if layered {
                    SetWindowLongPtrW(h, GWL_EXSTYLE, ex & !(WS_EX_LAYERED as isize));
                }
            } else if !layered || s.alpha != Some(alpha) {
                if !layered {
                    SetWindowLongPtrW(h, GWL_EXSTYLE, ex | WS_EX_LAYERED as isize);
                }
                SetLayeredWindowAttributes(h, 0, alpha, LWA_ALPHA);
            }
            s.alpha = Some(alpha);
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
    pub unsafe fn find(ctx: &egui::Context) -> Option<HWND> {
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
