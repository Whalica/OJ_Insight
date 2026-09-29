use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::app::state::AppState;

const LABEL: &str = "study-assistant";

#[cfg(target_os = "windows")]
fn set_assistant_window_shape(window: &WebviewWindow, round: bool) -> Result<(), String> {
    #[link(name = "user32")]
    extern "system" {
        fn SetWindowRgn(hwnd: isize, region: isize, redraw: i32) -> i32;
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn CreateEllipticRgn(left: i32, top: i32, right: i32, bottom: i32) -> isize;
        fn DeleteObject(object: isize) -> i32;
    }

    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0 as isize;
    let region = if round {
        // The collapsed window is 100 logical pixels; Win32 regions use physical pixels.
        let scale = window.scale_factor().map_err(|error| error.to_string())?;
        let diameter = (100.0 * scale).round() as i32;
        unsafe { CreateEllipticRgn(0, 0, diameter, diameter) }
    } else {
        0
    };
    if round && region == 0 {
        return Err("无法创建小助手的圆形窗口".into());
    }
    if unsafe { SetWindowRgn(hwnd, region, 1) } == 0 {
        if region != 0 {
            unsafe { DeleteObject(region); }
        }
        return Err("无法设置小助手窗口形状".into());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_assistant_window_shape(_window: &WebviewWindow, _round: bool) -> Result<(), String> {
    Ok(())
}

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
        .inner_size(100.0, 100.0)
        .min_inner_size(100.0, 100.0)
        .resizable(false)
        .center()
        .decorations(false)
        .always_on_top(true)
        .visible(false)
        .data_directory(webview_dir);
    #[cfg(not(target_os = "macos"))]
    let builder = builder
        .transparent(true)
        .shadow(false)
        .background_color(tauri::webview::Color(0, 0, 0, 0));
    let window = builder.build()
        .map_err(|error| error.to_string())?;
    if let Err(error) = set_assistant_window_shape(&window, true) {
        let _ = window.close();
        return Err(error);
    }
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn resize_study_assistant(app: AppHandle, expanded: bool) -> Result<(), String> {
    let window = app.get_webview_window(LABEL).ok_or("小助手窗口尚未打开")?;
    if expanded {
        set_assistant_window_shape(&window, false)?;
    }
    window
        .set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize::new(
            if expanded { 560.0 } else { 100.0 },
            if expanded { 480.0 } else { 100.0 },
        ))))
        .map_err(|error| error.to_string())?;
    window
        .set_resizable(expanded)
        .map_err(|error| error.to_string())?;
    let (width, height) = if expanded { (720.0, 540.0) } else { (100.0, 100.0) };
    window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(width, height)))
        .map_err(|error| error.to_string())?;
    if !expanded {
        set_assistant_window_shape(&window, true)?;
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
