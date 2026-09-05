#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
fn configure_linux_webview() {
    use std::env;

    let is_appimage = env::var_os("APPIMAGE").is_some() || env::var_os("APPDIR").is_some();
    let is_wayland = env::var("XDG_SESSION_TYPE")
        .map(|value| value.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
        || env::var_os("WAYLAND_DISPLAY").is_some();
    let is_niri = env::var_os("NIRI_SOCKET").is_some() || env::var("XDG_CURRENT_DESKTOP")
        .map(|value| value.to_ascii_lowercase().contains("niri"))
        .unwrap_or(false)
        || env::var("XDG_SESSION_DESKTOP")
            .map(|value| value.to_ascii_lowercase().contains("niri"))
            .unwrap_or(false);

    // linuxdeploy's GTK hook can force AppImages onto X11 even inside a
    // Wayland session. Select native Wayland before GTK/WebKit is initialized,
    // while retaining X11 as a fallback. An app-specific override remains
    // available for systems that deliberately need another backend.
    if let Some(value) = env::var_os("OJ_INSIGHT_GDK_BACKEND") {
        env::set_var("GDK_BACKEND", value);
    } else if is_appimage && is_wayland {
        env::set_var("GDK_BACKEND", "wayland,x11");
    }

    // WebKitGTK's DMA-BUF renderer is a known source of GBM/EGL aborts on
    // Wayland, especially in portable bundles using newer host graphics
    // stacks. Keep explicit user values, otherwise try the compatibility renderer.
    if is_appimage && is_wayland && env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    // niri reports the surfaceless EGL_BAD_ALLOC failure before the UI can
    // offer a recovery action. Prefer reliable software compositing for this
    // AppImage/session combination (requires confirmation on affected hardware).
    // Users may opt back into accelerated
    // compositing with OJ_INSIGHT_ENABLE_GPU=1.
    let enable_gpu = env::var("OJ_INSIGHT_ENABLE_GPU")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let software_requested = env::var("OJ_INSIGHT_SOFTWARE_RENDERING")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if (is_appimage && is_wayland && is_niri && !enable_gpu) || software_requested {
        env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    configure_linux_webview();
    oj_insight_lib::run();
}
