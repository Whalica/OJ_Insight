use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::app::state::AppState;
use crate::sync::normalize_qoj_cookie;

const LABEL: &str = "qoj-login";
const QOJ_URL: &str = "https://qoj.ac/";

#[tauri::command]
pub(crate) async fn open_qoj_login(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    let url = "https://qoj.ac/login".parse().map_err(|error| format!("QOJ 地址无效：{error}"))?;
    let webview_dir = app.state::<AppState>().webview_dir.clone();
    WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::External(url))
        .title("QOJ 登录 · OJ Insight")
        .inner_size(1000.0, 760.0)
        .center()
        .data_directory(webview_dir)
        .build()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn read_qoj_login_cookie(app: AppHandle) -> Result<String, String> {
    let window = app.get_webview_window(LABEL)
        .ok_or("请先打开 QOJ 登录页，完成登录后再导入")?;
    let url = QOJ_URL.parse().map_err(|error| format!("QOJ 地址无效：{error}"))?;
    let cookies = window.cookies_for_url(url).map_err(|error| error.to_string())?;
    let header = cookies.into_iter()
        .map(|cookie| format!("{}={}", cookie.name(), cookie.value()))
        .filter(|part| normalize_qoj_cookie(part).is_ok())
        .collect::<Vec<_>>()
        .join("; ");
    if header.is_empty() {
        return Err("尚未读到 QOJ Cookie；请在登录窗口完成登录后重试".into());
    }
    Ok(header)
}
