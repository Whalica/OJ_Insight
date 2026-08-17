mod db;
mod models;
mod sync;

use reqwest::Client;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};
use tauri_plugin_opener::OpenerExt;

use models::*;

struct AppState {
    db: Mutex<rusqlite::Connection>,
    client: Client,
    root_dir: PathBuf,
    data_dir: PathBuf,
    export_dir: PathBuf,
    webview_dir: PathBuf,
    log_dir: PathBuf,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageInfo {
    root_dir: String,
    data_dir: String,
    database_path: String,
    export_dir: String,
    webview_dir: String,
    log_dir: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInfo {
    current_version: String,
    latest_version: String,
    release_url: String,
    update_available: bool,
}

fn redact(input: &str, secret: &str) -> String {
    let mut value = if secret.trim().is_empty() {
        input.to_string()
    } else {
        input.replace(secret, "[REDACTED]")
    };
    if let Ok(re) = regex::Regex::new(r"(?i)UOJSESSID=[^;\s]+") {
        value = re.replace_all(&value, "UOJSESSID=[REDACTED]").into_owned();
    }
    value
}

fn log_event(state: &AppState, platform: &str, message: &str, secret: &str) {
    let path = state.log_dir.join("oj-insight.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let safe = redact(message, secret);
        let _ = writeln!(
            file,
            "{} [{}] {}",
            chrono::Utc::now().to_rfc3339(),
            platform,
            safe
        );
    }
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
        database_path: state
            .data_dir
            .join("oj-insight.sqlite3")
            .to_string_lossy()
            .into_owned(),
        export_dir: state.export_dir.to_string_lossy().into_owned(),
        webview_dir: state.webview_dir.to_string_lossy().into_owned(),
        log_dir: state.log_dir.to_string_lossy().into_owned(),
    }
}

#[tauri::command]
fn get_accounts(state: State<'_, AppState>) -> Result<Vec<AccountConfig>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::get_accounts(&*conn)
}

#[tauri::command]
fn save_account(
    state: State<'_, AppState>,
    platform: String,
    account: String,
    secret: String,
) -> Result<(), String> {
    if !PLATFORMS.contains(&platform.as_str()) {
        return Err("不支持的平台".into());
    }
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::save_account(&*conn, &platform, &account, &secret)
}

#[tauri::command]
fn get_sync_statuses(state: State<'_, AppState>) -> Result<Vec<SyncStatus>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::statuses(&*conn)
}

async fn sync_one_inner(
    state: &AppState,
    platform: &str,
    full: bool,
) -> Result<SyncResult, String> {
    let (account, cursor) = {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        let account = db::get_account(&conn, platform)?;
        if account.account.trim().is_empty() {
            return Err(format!("{} 尚未填写账号", platform));
        }
        let cursor = if full {
            0
        } else {
            db::get_cursor(&conn, platform)?
        };
        db::mark_syncing(&conn, platform, &account.account)?;
        (account, cursor)
    };
    log_event(
        state,
        platform,
        if full {
            "full rebuild started"
        } else {
            "incremental sync started"
        },
        &account.secret,
    );
    match sync::fetch_platform(&state.client, &account, full, cursor).await {
        Ok(remote) => {
            let (inserted, updated) = {
                let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
                db::apply_remote(&mut conn, &remote)?
            };
            log_event(
                state,
                platform,
                &format!("sync completed inserted={inserted} updated={updated}"),
                &account.secret,
            );
            Ok(SyncResult {
                platform: platform.into(),
                inserted,
                updated,
                message: format!("同步成功 · 新增 {inserted}，更新 {updated}"),
                status: "ok".into(),
            })
        }
        Err(err) => {
            let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
            let _ = db::mark_failed(&conn, platform, &account.account, &err.status, &err.message);
            log_event(
                state,
                platform,
                &format!("sync failed status={} message={}", err.status, err.message),
                &account.secret,
            );
            Err(err.message)
        }
    }
}

#[tauri::command]
async fn sync_platform(
    state: State<'_, AppState>,
    platform: String,
    full: bool,
) -> Result<SyncResult, String> {
    if !PLATFORMS.contains(&platform.as_str()) {
        return Err("不支持的平台".into());
    }
    sync_one_inner(&state, &platform, full).await
}

#[tauri::command]
async fn sync_all(state: State<'_, AppState>) -> Result<Vec<SyncResult>, String> {
    let configured = {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        db::get_accounts(&conn)?
            .into_iter()
            .filter(|a| !a.account.trim().is_empty())
            .map(|a| a.platform)
            .collect::<Vec<_>>()
    };
    let mut out = Vec::new();
    for p in configured {
        match sync_one_inner(&state, &p, false).await {
            Ok(r) => out.push(r),
            Err(message) => out.push(SyncResult {
                platform: p,
                inserted: 0,
                updated: 0,
                message,
                status: "error".into(),
            }),
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
fn get_snapshot(
    state: State<'_, AppState>,
    platform: Option<String>,
    start_day: Option<String>,
    end_day: Option<String>,
    metric: String,
) -> Result<Snapshot, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::snapshot(
        &*conn,
        platform.as_deref(),
        start_day.as_deref(),
        end_day.as_deref(),
        &metric,
    )
}

#[tauri::command]
fn get_day_detail(
    state: State<'_, AppState>,
    day: String,
    platform: Option<String>,
) -> Result<DayDetail, String> {
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

#[tauri::command]
async fn check_for_updates(state: State<'_, AppState>) -> Result<UpdateInfo, String> {
    let value = state
        .client
        .get("https://api.github.com/repos/Whalica/OJ_Insight/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("检查更新失败：{e}"))?
        .error_for_status()
        .map_err(|e| format!("GitHub Releases：{e}"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("解析版本信息失败：{e}"))?;
    let latest = value
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    if latest.is_empty() {
        return Err("GitHub Releases 没有可用版本".into());
    }
    let current = env!("CARGO_PKG_VERSION").to_string();
    let update_available = version_tuple(&latest) > version_tuple(&current);
    Ok(UpdateInfo {
        current_version: current,
        latest_version: latest,
        release_url: value
            .get("html_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://github.com/Whalica/OJ_Insight/releases")
            .into(),
        update_available,
    })
}

fn version_tuple(v: &str) -> (u64, u64, u64) {
    let mut p = v
        .split('.')
        .map(|x| x.split('-').next().unwrap_or("0").parse().unwrap_or(0));
    (
        p.next().unwrap_or(0),
        p.next().unwrap_or(0),
        p.next().unwrap_or(0),
    )
}

#[tauri::command]
fn open_external(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let allowed = [
        "https://github.com/Whalica/OJ_Insight",
        "https://github.com/Whalica/OJ_Insight/issues",
        "https://github.com/Whalica/OJ_Insight/releases",
    ];
    if !allowed.iter().any(|prefix| url.starts_with(prefix)) {
        return Err("不允许打开该链接".into());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
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
            let log_dir = root_dir.join("logs");
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(&export_dir)?;
            std::fs::create_dir_all(&webview_dir)?;
            std::fs::create_dir_all(&log_dir)?;

            let conn =
                db::open(&data_dir.join("oj-insight.sqlite3")).map_err(std::io::Error::other)?;
            let client = Client::builder()
                .user_agent("OJ-Insight/0.2.0")
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
                log_dir,
            });

            // The main WebView is created manually so WebView2 localStorage/cache also
            // stays inside the application root instead of the system app-data folders.
            tauri::WebviewWindowBuilder::from_config(app.handle(), &app.config().app.windows[0])?
                .data_directory(webview_dir)
                .build()?;
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_storage_info,
            get_accounts,
            save_account,
            get_sync_statuses,
            sync_platform,
            sync_all,
            clear_platform_records,
            clear_all_records,
            get_snapshot,
            get_day_detail,
            write_export_file,
            check_for_updates,
            open_external
        ])
        .run(tauri::generate_context!())
        .expect("error while running OJ Insight");
}
