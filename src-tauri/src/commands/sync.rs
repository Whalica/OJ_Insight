use tauri::State;

use crate::app::state::AppState;
use crate::db;
use crate::models::{SyncResult, PLATFORMS};
use crate::sync::service;

#[tauri::command]
pub(crate) async fn sync_platform(
    state: State<'_, AppState>,
    platform: String,
    full: bool,
) -> Result<SyncResult, String> {
    if !PLATFORMS.contains(&platform.as_str()) {
        return Err("不支持的平台".into());
    }
    service::sync_platform(&state, &platform, full).await
}

#[tauri::command]
pub(crate) async fn sync_all(state: State<'_, AppState>) -> Result<Vec<SyncResult>, String> {
    service::sync_all(&state).await
}

#[tauri::command]
pub(crate) fn clear_platform_records(
    state: State<'_, AppState>,
    platform: String,
) -> Result<(), String> {
    if !PLATFORMS.contains(&platform.as_str()) {
        return Err("不支持的平台".into());
    }
    let _operation = state.operations.enter()?;
    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::clear_platform(&mut conn, &platform)
}

#[tauri::command]
pub(crate) fn clear_all_records(state: State<'_, AppState>) -> Result<(), String> {
    let _operation = state.operations.enter()?;
    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::clear_all(&mut conn)
}
