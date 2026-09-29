use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::app::state::AppState;

const LABEL: &str = "study-assistant";

#[tauri::command]
pub(crate) async fn open_study_assistant(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    let webview_dir = app.state::<AppState>().webview_dir.clone();
    let builder = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("index.html?study_assistant=1".into()))
        .title("OJ Insight · 做题小助手")
        .inner_size(160.0, 160.0)
        .min_inner_size(160.0, 160.0)
        .resizable(false)
        .center()
        .decorations(false)
        .always_on_top(true)
        .data_directory(webview_dir);
    #[cfg(not(target_os = "macos"))]
    let builder = builder.shadow(false);
    let window = builder.build()
        .map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn resize_study_assistant(app: AppHandle, expanded: bool) -> Result<(), String> {
    let window = app.get_webview_window(LABEL).ok_or("小助手窗口尚未打开")?;
    window
        .set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize::new(
            if expanded { 560.0 } else { 160.0 },
            if expanded { 480.0 } else { 160.0 },
        ))))
        .map_err(|error| error.to_string())?;
    window
        .set_resizable(expanded)
        .map_err(|error| error.to_string())?;
    let (width, height) = if expanded { (720.0, 540.0) } else { (160.0, 160.0) };
    window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(width, height)))
        .map_err(|error| error.to_string())?;
    if expanded {
        if let Some(monitor) = window.current_monitor().map_err(|error| error.to_string())? {
            let work = monitor.work_area();
            let position = window.outer_position().map_err(|error| error.to_string())?;
            let scale = window.scale_factor().map_err(|error| error.to_string())?;
            let target_width = (width * scale).ceil() as i32;
            let target_height = (height * scale).ceil() as i32;
            let x = position.x.clamp(work.position.x, (work.position.x + work.size.width as i32 - target_width).max(work.position.x));
            let y = position.y.clamp(work.position.y, (work.position.y + work.size.height as i32 - target_height).max(work.position.y));
            if x != position.x || y != position.y {
                window.set_position(tauri::PhysicalPosition::new(x, y)).map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn drag_study_assistant(app: AppHandle) -> Result<(), String> {
    app.get_webview_window(LABEL).ok_or("小助手窗口尚未打开")?
        .start_dragging().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn close_study_assistant(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.close().map_err(|error| error.to_string())?;
    }
    Ok(())
}
