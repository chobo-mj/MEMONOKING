//! Embeds the app icon into the Windows executable (file icon, Alt-Tab,
//! taskbar fallback). No-op on other platforms.

#[cfg(windows)]
fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("icon/memo_app_yellow.ico");
    res.compile()
        .expect("failed to embed the app icon resource");
}

#[cfg(not(windows))]
fn main() {}
