use std::path::PathBuf;
#[cfg(not(target_os = "windows"))]
use tauri::Manager;

#[cfg(target_os = "windows")]
fn executable_root_dir() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    exe.parent()
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other("无法定位 OJ Insight 可执行文件所在目录"))
}

/// Resolve the root directory that hosts `data/`, `exports/`, `logs` and `webview/`.
///
/// Windows ships a portable folder layout, so data lives next to the executable.
/// macOS and Linux use Tauri's per-user application data directory because their
/// installed application locations are not generally writable.
pub(crate) fn portable_root_dir(app: &tauri::AppHandle) -> std::io::Result<PathBuf> {
    #[cfg(not(target_os = "windows"))]
    {
        app.path().app_data_dir().map_err(std::io::Error::other)
    }
    #[cfg(target_os = "windows")]
    {
        let _ = app;
        executable_root_dir()
    }
}
