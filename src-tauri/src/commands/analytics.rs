use tauri::State;

use crate::app::state::AppState;
use crate::db;
use crate::models::{DayDetail, DifficultyDetail, Snapshot, PLATFORMS};

#[tauri::command]
pub(crate) fn get_snapshot(
    state: State<'_, AppState>,
    platform: Option<String>,
    start_day: Option<String>,
    end_day: Option<String>,
    metric: String,
    account: Option<String>,
    source: Option<String>,
    time_zone: Option<String>,
) -> Result<Snapshot, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::snapshot(
        &conn,
        platform.as_deref(),
        start_day.as_deref(),
        end_day.as_deref(),
        &metric,
        account.as_deref(),
        source.as_deref(),
        time_zone.as_deref().unwrap_or("Asia/Shanghai"),
    )
}

#[tauri::command]
pub(crate) fn get_day_detail(
    state: State<'_, AppState>,
    day: String,
    platform: Option<String>,
    account: Option<String>,
    source: Option<String>,
    time_zone: Option<String>,
) -> Result<DayDetail, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::day_detail(
        &conn,
        &day,
        platform.as_deref(),
        account.as_deref(),
        source.as_deref(),
        time_zone.as_deref().unwrap_or("Asia/Shanghai"),
    )
}

#[tauri::command]
pub(crate) fn get_difficulty_detail(
    state: State<'_, AppState>,
    platform: String,
    label: String,
    account: Option<String>,
    source: Option<String>,
) -> Result<DifficultyDetail, String> {
    if !PLATFORMS.contains(&platform.as_str()) {
        return Err("不支持的平台".into());
    }
    if label.trim().is_empty() {
        return Err("难度不能为空".into());
    }
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::difficulty_detail(
        &conn,
        &platform,
        &label,
        account.as_deref(),
        source.as_deref(),
    )
}
