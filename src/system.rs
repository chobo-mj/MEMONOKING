//! Small Windows integrations: start with Windows and the alert sound.

/// Start (or stop starting) OmniMemoJang when the user signs in to Windows:
/// the per-user `Run` key, so no administrator rights are needed.
pub fn set_autostart(on: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
            KEY_SET_VALUE, REG_SZ,
        };
        let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
        let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
        let name = wide("OmniMemoJang");
        let mut h: HKEY = std::ptr::null_mut();
        // SAFETY: plain registry calls on a key this function opens and closes.
        unsafe {
            if RegOpenKeyExW(HKEY_CURRENT_USER, key.as_ptr(), 0, KEY_SET_VALUE, &mut h) != 0 {
                return Err("시작 프로그램 설정을 열지 못했습니다".into());
            }
            let result = if on {
                let exe = std::env::current_exe().map_err(|e| e.to_string())?;
                // Started by Windows: straight to the tray, stickers on the desktop.
                let value = wide(&format!("\"{}\" --tray", exe.display()));
                RegSetValueExW(
                    h,
                    name.as_ptr(),
                    0,
                    REG_SZ,
                    value.as_ptr() as *const u8,
                    (value.len() * 2) as u32,
                )
            } else {
                let r = RegDeleteValueW(h, name.as_ptr());
                // Already absent is fine.
                if r == 2 {
                    0
                } else {
                    r
                }
            };
            RegCloseKey(h);
            if result != 0 {
                return Err(format!("시작 프로그램 설정을 바꾸지 못했습니다 ({result})"));
            }
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = on;
        Ok(())
    }
}

/// The Windows notification sound.
pub fn alert_sound() {
    #[cfg(windows)]
    // SAFETY: no pointers involved.
    unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::MessageBeep(
            windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONASTERISK,
        );
    }
}
