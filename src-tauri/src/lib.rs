mod db;
mod models;
mod sync;

use std::path::PathBuf;
use std::sync::Mutex;
use reqwest::Client;
use tauri::{Manager, State};

use models::*;

struct AppState {
    db: Mutex<rusqlite::Connection>,
    client: Client,
    root_dir: PathBuf,
    data_dir: PathBuf,
    export_dir: PathBuf,
    webview_dir: PathBuf,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageInfo {
    root_dir: String,
    data_dir: String,
    database_path: String,
    export_dir: String,
    webview_dir: String,
}

fn executable_root_dir() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    exe.parent()
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other("无法定位 OJ Insight 可执行文件所在目录"))
}


#[tauri::command]
fn get_storage_info(state: State<'_, AppState>) -> StorageInfo {
    StorageInfo {
        root_dir: state.root_dir.to_string_lossy().into_owned(),
        data_dir: state.data_dir.to_string_lossy().into_owned(),
        database_path: state.data_dir.join("oj-insight.sqlite3").to_string_lossy().into_owned(),
        export_dir: state.export_dir.to_string_lossy().into_owned(),
        webview_dir: state.webview_dir.to_string_lossy().into_owned(),
    }
}

#[tauri::command]
fn get_accounts(state: State<'_, AppState>) -> Result<Vec<AccountConfig>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::get_accounts(&*conn)
}

#[tauri::command]
fn save_account(state: State<'_, AppState>, platform: String, account: String, secret: String) -> Result<(), String> {
    if !PLATFORMS.contains(&platform.as_str()) { return Err("不支持的平台".into()); }
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::save_account(&*conn, &platform, &account, &secret)
}

#[tauri::command]
fn get_sync_statuses(state: State<'_, AppState>) -> Result<Vec<SyncStatus>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::statuses(&*conn)
}

async fn sync_one_inner(state: &AppState, platform: &str, full: bool) -> Result<SyncResult, String> {
    let (account, cursor) = {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        let account = db::get_account(&conn, platform)?;
        if account.account.trim().is_empty() { return Err(format!("{} 尚未填写账号", platform)); }
        let cursor = if full { 0 } else { db::get_cursor(&conn, platform)? };
        db::mark_syncing(&conn, platform, &account.account)?;
        (account, cursor)
    };
    match sync::fetch_platform(&state.client, &account, full, cursor).await {
        Ok(remote) => {
            let (inserted, updated) = {
                let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
                db::apply_remote(&mut conn, &remote)?
            };
            Ok(SyncResult { platform: platform.into(), inserted, updated, message: format!("同步成功 · 新增 {inserted}，更新 {updated}"), status: "ok".into() })
        }
        Err(err) => {
            let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
            let _ = db::mark_failed(&conn, platform, &account.account, &err.status, &err.message);
            Err(err.message)
        }
    }
}

#[tauri::command]
async fn sync_platform(state: State<'_, AppState>, platform: String, full: bool) -> Result<SyncResult, String> {
    if !PLATFORMS.contains(&platform.as_str()) { return Err("不支持的平台".into()); }
    sync_one_inner(&state, &platform, full).await
}

#[tauri::command]
async fn sync_all(state: State<'_, AppState>) -> Result<Vec<SyncResult>, String> {
    let configured = {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        db::get_accounts(&conn)?.into_iter().filter(|a| !a.account.trim().is_empty()).map(|a| a.platform).collect::<Vec<_>>()
    };
    let mut out = Vec::new();
    for p in configured {
        match sync_one_inner(&state, &p, false).await {
            Ok(r) => out.push(r),
            Err(message) => out.push(SyncResult { platform: p, inserted: 0, updated: 0, message, status: "error".into() }),
        }
    }
    Ok(out)
}

#[tauri::command]
fn clear_platform_records(state: State<'_, AppState>, platform: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::clear_platform(&*conn, &platform)
}

#[tauri::command]
fn clear_all_records(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::clear_all(&*conn)
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>, platform: Option<String>, start_day: Option<String>, end_day: Option<String>, metric: String) -> Result<Snapshot, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::snapshot(&*conn, platform.as_deref(), start_day.as_deref(), end_day.as_deref(), &metric)
}

#[tauri::command]
fn get_day_detail(state: State<'_, AppState>, day: String, platform: Option<String>) -> Result<DayDetail, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::day_detail(&*conn, &day, platform.as_deref())
}

#[tauri::command]
fn write_export_file(path: String, data: Vec<u8>) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("导出路径为空".into());
    }
    std::fs::write(&path, data).map_err(|e| format!("写入导出文件失败：{e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Portable-data layout: every piece of persistent application data lives
            // next to the executable instead of %APPDATA% / LocalAppData.
            let root_dir = executable_root_dir()?;
            let data_dir = root_dir.join("data");
            let export_dir = root_dir.join("exports");
            let webview_dir = root_dir.join("webview");
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(&export_dir)?;
            std::fs::create_dir_all(&webview_dir)?;

            let conn = db::open(&data_dir.join("oj-insight.sqlite3")).map_err(std::io::Error::other)?;
            let client = Client::builder()
                .user_agent("OJ-Insight/0.1.1")
                .timeout(std::time::Duration::from_secs(35))
                .connect_timeout(std::time::Duration::from_secs(12))
                .build()?;
            app.manage(AppState {
                db: Mutex::new(conn),
                client,
                root_dir: root_dir.clone(),
                data_dir,
                export_dir,
                webview_dir: webview_dir.clone(),
            });

            // The main WebView is created manually so WebView2 localStorage/cache also
            // stays inside the application root instead of the system app-data folders.
            tauri::WebviewWindowBuilder::from_config(app.handle(), &app.config().app.windows[0])?
                .data_directory(webview_dir)
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_storage_info, get_accounts, save_account, get_sync_statuses, sync_platform, sync_all, clear_platform_records, clear_all_records, get_snapshot, get_day_detail, write_export_file])
        .run(tauri::generate_context!())
        .expect("error while running OJ Insight");
}
