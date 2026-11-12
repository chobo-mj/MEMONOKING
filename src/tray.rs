//! The notification-area (tray) icon, and keeping a single running copy.
//!
//! Closing the main window leaves the program running in the tray, with its
//! stickers on the desktop; "종료" in the tray menu really quits. Starting the
//! program again while it runs only brings the main window back.

/// What the user asked for from the tray (or a second launch).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Action {
    Open,
    NewSticker,
    Quit,
}

#[derive(Default)]
pub struct Tray {
    #[cfg(windows)]
    icon: Option<tray_icon::TrayIcon>,
    #[cfg(windows)]
    ids: Vec<(tray_icon::menu::MenuId, Action)>,
}

impl Tray {
    /// Show the tray icon (on Windows; elsewhere nothing).
    pub fn new() -> Self {
        #[cfg(windows)]
        {
            use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
            let open = MenuItem::new("열기", true, None);
            let sticker = MenuItem::new("새 스티커 메모", true, None);
            let quit = MenuItem::new("종료", true, None);
            let menu = Menu::new();
            let _ = menu.append(&open);
            let _ = menu.append(&sticker);
            let _ = menu.append(&PredefinedMenuItem::separator());
            let _ = menu.append(&quit);
            let icon = tray_icon::Icon::from_rgba(crate::app_icon::tray_rgba(), 32, 32)
                .ok()
                .and_then(|icon| {
                    tray_icon::TrayIconBuilder::new()
                        .with_tooltip("OmniMemoJang")
                        .with_icon(icon)
                        .with_menu(Box::new(menu))
                        .with_menu_on_left_click(false)
                        .build()
                        .ok()
                });
            Self {
                icon,
                ids: vec![
                    (open.id().clone(), Action::Open),
                    (sticker.id().clone(), Action::NewSticker),
                    (quit.id().clone(), Action::Quit),
                ],
            }
        }
        #[cfg(not(windows))]
        Self::default()
    }

    /// True when the icon is in the tray, so the main window may be closed
    /// without quitting.
    pub fn active(&self) -> bool {
        #[cfg(windows)]
        return self.icon.is_some();
        #[cfg(not(windows))]
        false
    }

    /// Actions requested since the last call.
    pub fn poll(&self) -> Vec<Action> {
        let mut out = vec![];
        #[cfg(windows)]
        {
            use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
            while let Ok(e) = TrayIconEvent::receiver().try_recv() {
                match e {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                    | TrayIconEvent::DoubleClick { .. } => out.push(Action::Open),
                    _ => {}
                }
            }
            while let Ok(e) = tray_icon::menu::MenuEvent::receiver().try_recv() {
                if let Some((_, a)) = self.ids.iter().find(|(id, _)| *id == e.id) {
                    out.push(*a);
                }
            }
        }
        if SHOW.swap(false, std::sync::atomic::Ordering::Relaxed) {
            out.push(Action::Open);
        }
        out
    }
}

/// Set when another launch asked this one to show its main window.
static SHOW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Make this the only running copy. Returns false when another copy is
/// already running; it has then been asked to show its main window.
pub fn single_instance() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
        use windows_sys::Win32::System::Threading::{
            CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject, INFINITE,
        };
        let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
        let (mutex, event) = (
            wide("Local\\OmniMemoJang.Running"),
            wide("Local\\OmniMemoJang.Show"),
        );
        // SAFETY: plain Win32 calls with valid, nul-terminated names. The
        // handles live as long as the process.
        unsafe {
            let _keep = CreateMutexW(std::ptr::null(), 0, mutex.as_ptr());
            let first = GetLastError() != ERROR_ALREADY_EXISTS;
            let show = CreateEventW(std::ptr::null(), 0, 0, event.as_ptr());
            if !first {
                // Started by Windows (`--tray`) while already running: stay quiet.
                if !show.is_null() && !std::env::args().any(|x| x == "--tray") {
                    SetEvent(show);
                }
                return false;
            }
            if !show.is_null() {
                let show = show as usize;
                std::thread::spawn(move || loop {
                    if WaitForSingleObject(show as _, INFINITE) != 0 {
                        break;
                    }
                    SHOW.store(true, std::sync::atomic::Ordering::Relaxed);
                });
            }
        }
        true
    }
    #[cfg(not(windows))]
    true
}
