use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::app::state::AppState;
use crate::db::{self, FavoriteCategory, FavoriteInput, FavoriteItem};

#[tauri::command]
pub(crate) fn list_favorite_categories(state: State<'_, AppState>) -> Result<Vec<FavoriteCategory>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::list_favorite_categories(&conn)
}

#[tauri::command]
pub(crate) fn save_favorite_category(state: State<'_, AppState>, id: Option<i64>, name: String) -> Result<FavoriteCategory, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::save_favorite_category(&conn, id, &name)
}

#[tauri::command]
pub(crate) fn delete_favorite_category(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::delete_favorite_category(&conn, id)
}

#[tauri::command]
pub(crate) fn list_favorite_items(state: State<'_, AppState>) -> Result<Vec<FavoriteItem>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::list_favorite_items(&conn)
}

#[tauri::command]
pub(crate) fn save_favorite_item(state: State<'_, AppState>, input: FavoriteInput) -> Result<FavoriteItem, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::save_favorite_item(&conn, input)
}

#[tauri::command]
pub(crate) fn delete_favorite_item(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::delete_favorite_item(&conn, id)
}

#[tauri::command]
pub(crate) fn open_favorite_link(app: tauri::AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let url = {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        db::get_favorite_item(&conn, id)?.url
    };
    let parsed = reqwest::Url::parse(&url).map_err(|_| "链接格式无效".to_string())?;
    if !["http", "https"].contains(&parsed.scheme()) || parsed.host_str().is_none() { return Err("仅支持 http(s) 网页链接".into()); }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}
