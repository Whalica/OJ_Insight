use tauri::State;

use crate::app::state::AppState;
use crate::db;
use crate::models::{AccountConfig, SyncStatus, PLATFORMS};

#[tauri::command]
pub(crate) fn get_accounts(state: State<'_, AppState>) -> Result<Vec<AccountConfig>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::get_accounts(&conn)
}

#[tauri::command]
pub(crate) fn save_account(
    state: State<'_, AppState>,
    platform: String,
    account: String,
    mut secret: String,
) -> Result<(), String> {
    if !PLATFORMS.contains(&platform.as_str()) {
        return Err("不支持的平台".into());
    }
    let _operation = state.operations.enter()?;
    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    if platform == "qoj" && !account.trim().is_empty() && !secret.trim().is_empty() {
        secret = crate::sync::normalize_qoj_cookie(&secret)?;
    }
    db::save_account(&mut conn, &platform, &account, &secret)
}

#[tauri::command]
pub(crate) fn save_accounts(
    state: State<'_, AppState>,
    platform: String,
    mut accounts: Vec<AccountConfig>,
) -> Result<(), String> {
    if !PLATFORMS.contains(&platform.as_str()) {
        return Err("不支持的平台".into());
    }
    if accounts.iter().any(|entry| entry.platform != platform) {
        return Err("账号列表的平台不一致".into());
    }
    let _operation = state.operations.enter()?;
    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    normalize_saved_cookies(&mut accounts)?;
    db::replace_accounts(&mut conn, &platform, &accounts)
}

#[tauri::command]
pub(crate) fn save_all_accounts(
    state: State<'_, AppState>,
    mut accounts: Vec<AccountConfig>,
) -> Result<(), String> {
    if accounts
        .iter()
        .any(|entry| !PLATFORMS.contains(&entry.platform.as_str()))
    {
        return Err("不支持的平台".into());
    }
    let _operation = state.operations.enter()?;
    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    normalize_saved_cookies(&mut accounts)?;
    db::replace_all_accounts(&mut conn, &accounts)
}

#[tauri::command]
pub(crate) fn get_sync_statuses(state: State<'_, AppState>) -> Result<Vec<SyncStatus>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::statuses(&conn)
}

fn normalize_saved_cookies(accounts: &mut [AccountConfig]) -> Result<(), String> {
    for entry in accounts {
        if entry.platform == "qoj" && !entry.account.trim().is_empty() && !entry.secret.trim().is_empty() {
            entry.secret = crate::sync::normalize_qoj_cookie(&entry.secret)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod cookie_save_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn saved_cookie_round_trips_without_changing_its_name_or_value() {
        let mut conn = db::open(Path::new(":memory:")).unwrap();
        let mut accounts = vec![AccountConfig { platform: "qoj".into(), account: "tester".into(), secret: "Cookie: future_name=abc==; theme=dark; ".into() }];
        normalize_saved_cookies(&mut accounts).unwrap();
        db::replace_all_accounts(&mut conn, &accounts).unwrap();
        let saved = db::get_accounts(&conn).unwrap();
        assert_eq!(saved[0].secret, "future_name=abc==; theme=dark");
        accounts[0].secret = "bare_value".into();
        assert!(normalize_saved_cookies(&mut accounts).is_err());
        assert_eq!(db::get_accounts(&conn).unwrap()[0].secret, saved[0].secret);
    }
}
